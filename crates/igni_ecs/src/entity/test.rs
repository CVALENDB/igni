//! Pruebas unitarias del módulo `entity`.
//!
//! Este archivo es hijo de `entity`, por lo que puede acceder a los elementos privados
//! (`Entity::from_parts`, `EntityManager::generations`, `EntityManager::free_indices`).
//!
//! Meta del módulo: que `EntityManager` entregue identificadores únicos y que, al reciclar
//! índices, una referencia vieja nunca pueda confundirse con una entidad nueva.
//!
//! Cada prueba documenta **qué busca probar** y **qué debe cumplirse**.

use crate::entity::{Entity, EntityManager};



/// **Busca probar:** que un administrador nuevo reparta índices consecutivos desde cero.
///
/// **Debe cumplirse:**
/// - Las primeras entidades tienen índices `0, 1, 2, ...`.
/// - Todas nacen con generación `0`.
/// - Todas están vivas apenas se crean.
#[test]
fn first_entities_have_sequential_indices_and_generation_zero() {
    let mut m = EntityManager::new();
    for expected in 0..5u32 {
        let e = m.create_entity();
        assert_eq!(e.index(), expected);
        assert_eq!(e.generation(), 0);
        assert!(m.is_alive(e));
    }
}

/// **Busca probar:** que dos entidades vivas nunca compartan identificador.
///
/// **Debe cumplirse:**
/// - Al crear 1000 entidades seguidas, las 1000 son distintas entre sí.
/// - Todas siguen vivas.
#[test]
fn created_entities_are_all_unique() {
    let mut m = EntityManager::new();
    let entities: Vec<Entity> = (0..1000).map(|_| m.create_entity()).collect();

    let unique: std::collections::HashSet<Entity> = entities.iter().copied().collect();
    assert_eq!(unique.len(), 1000);
    assert!(entities.iter().all(|&e| m.is_alive(e)));
}


/// **Busca probar:** que una entidad solo se pueda destruir una vez (sin doble eliminación).
///
/// **Debe cumplirse:**
/// - La primera destrucción devuelve `true`.
/// - Después, `is_alive` devuelve `false`.
/// - La segunda destrucción de la misma referencia devuelve `false`.
#[test]
fn destroy_returns_true_once_then_false() {
    let mut m = EntityManager::new();
    let e = m.create_entity();

    assert!(m.destroy_entity(e));
    assert!(!m.is_alive(e));
    assert!(!m.destroy_entity(e));
}

/// **Busca probar:** que destruir una entidad no tenga efectos sobre las demás.
///
/// **Debe cumplirse:**
/// - Al destruir la entidad del medio, las otras dos siguen vivas.
/// - Solo la destruida deja de estar viva.
#[test]
fn destroying_one_entity_does_not_affect_others() {
    let mut m = EntityManager::new();
    let a = m.create_entity();
    let b = m.create_entity();
    let c = m.create_entity();

    assert!(m.destroy_entity(b));

    assert!(m.is_alive(a));
    assert!(!m.is_alive(b));
    assert!(m.is_alive(c));
}

/// **Busca probar:** el caso central del diseño: reciclar un índice sin que la
/// referencia vieja pueda confundirse con la nueva (problema ABA).
///
/// **Debe cumplirse:**
/// - La entidad nueva reutiliza el mismo índice que la destruida.
/// - Su generación es la anterior + 1, por lo que ambas son distintas.
/// - La referencia vieja sigue muerta aunque su índice esté ocupado otra vez.
/// - El vector de generaciones no crece al reciclar.
#[test]
fn recycled_slot_reuses_index_and_bumps_generation() {
    let mut m = EntityManager::new();
    let old = m.create_entity();
    m.destroy_entity(old);

    let new = m.create_entity();

    assert_eq!(new.index(), old.index());
    assert_eq!(new.generation(), old.generation() + 1);
    assert_ne!(new, old);
    assert!(!m.is_alive(old));
    assert!(m.is_alive(new));
    assert_eq!(m.generations.len(), 1, "no debe crecer el vector al reciclar");
}

/// **Busca probar:** que los índices liberados se reutilicen como una pila (LIFO).
///
/// **Debe cumplirse:**
/// - Se destruye `a` y luego `c`.
/// - La siguiente entidad creada recibe el índice de `c` (el último liberado).
/// - La que sigue recibe el índice de `a`.
#[test]
fn free_indices_are_recycled_in_lifo_order() {
    let mut m = EntityManager::new();
    let a = m.create_entity();
    let _b = m.create_entity();
    let c = m.create_entity();

    m.destroy_entity(a);
    m.destroy_entity(c);

    assert_eq!(m.create_entity().index(), c.index());
    assert_eq!(m.create_entity().index(), a.index());
}


/// **Busca probar:** que una entidad inventada, con un índice que nunca se creó,
/// se rechace de forma segura.
///
/// **Debe cumplirse:**
/// - `is_alive` devuelve `false`.
/// - `destroy_entity` devuelve `false`.
/// - Ninguna de las dos hace `panic`.
#[test]
fn out_of_range_entity_is_rejected_without_panic() {
    let mut m = EntityManager::new();
    m.create_entity();

    let fake = Entity::from_parts(9999, 0);
    assert!(!m.is_alive(fake));
    assert!(!m.destroy_entity(fake));
}

/// **Busca probar:** que una referencia con generación mayor a la actual de la ranura
/// no se acepte como válida.
///
/// **Debe cumplirse:**
/// - `is_alive` y `destroy_entity` rechazan la entidad con generación futura.
/// - La entidad real, con el mismo índice, no se ve afectada y sigue viva.
#[test]
fn future_generation_is_rejected() {
    let mut m = EntityManager::new();
    let e = m.create_entity();

    let future = Entity::from_parts(e.index(), e.generation() + 1);
    assert!(!m.is_alive(future));
    assert!(!m.destroy_entity(future));
    assert!(m.is_alive(e), "la entidad real no debe verse afectada");
}


/// **Busca probar:** que el empaquetado en `u64` mantenga índice y generación
/// totalmente separados.
///
/// **Debe cumplirse:**
/// - Con generación casi al máximo, el índice se lee intacto.
/// - Con índice al máximo, la generación se lee intacta.
#[test]
fn index_and_generation_do_not_contaminate_each_other() {
    let e = Entity::from_parts(5, u32::MAX - 1);
    assert_eq!(e.index(), 5);
    assert_eq!(e.generation(), u32::MAX - 1);

    let e = Entity::from_parts(u32::MAX, 7);
    assert_eq!(e.index(), u32::MAX);
    assert_eq!(e.generation(), 7);
}


/// **Busca probar:** que el ciclo crear/destruir sobre una misma ranura sea estable
/// tras muchas repeticiones.
///
/// **Debe cumplirse:**
/// - En 100 000 ciclos siempre se usa el índice `0`.
/// - La generación avanza de uno en uno, sin saltos.
/// - No hay `panic` y el vector nunca pasa de una ranura.
/// - La generación final de la ranura es `100_000`.
#[test]
fn stress_same_slot_100k_cycles() {
    let mut m = EntityManager::new();

    for i in 0..100_000u32 {
        let e = m.create_entity();
        assert_eq!(e.index(), 0);
        assert_eq!(e.generation(), i);
        assert!(m.destroy_entity(e));
    }

    assert_eq!(m.generations.len(), 1);
    assert_eq!(m.generations[0], 100_000);
}

/// **Busca probar:** que una ranura con la generación agotada se retire en lugar de
/// dar la vuelta a cero (lo que reabriría el problema ABA).
///
/// Se simula el límite escribiendo la generación a mano, para no tener que reciclar
/// 4 mil millones de veces.
///
/// **Debe cumplirse:**
/// - La última entidad de la ranura se puede destruir una vez.
/// - La ranura no vuelve a `free_indices`.
/// - Ni la referencia vieja ni una con la generación reservada se consideran vivas.
/// - La siguiente entidad creada usa una ranura nueva (índice `1`).
#[test]
fn exhausted_slot_is_retired_instead_of_wrapping() {
    let mut m = EntityManager::new();
    let _ = m.create_entity();
    m.generations[0] = u32::MAX - 1;

    let last = Entity::from_parts(0, u32::MAX - 1);
    assert!(m.is_alive(last));
    assert!(m.destroy_entity(last));

    assert!(m.free_indices.is_empty());
    assert!(!m.is_alive(last));
    assert!(!m.is_alive(Entity::from_parts(0, u32::MAX)));

    assert_eq!(m.create_entity().index(), 1);
}