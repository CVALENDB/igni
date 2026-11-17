

/// Identificador único de entidad en el ECS.
///
/// Internamente es un `u64` empaquetado:
/// - Los **32 bits superiores** guardan la **generación**.
/// - Los **32 bits inferiores** guardan el **índice**.
///
/// El campo es privado para que nadie pueda fabricar identificadores arbitrarios
/// saltándose el empaquetado. Solo `EntityManager` crea entidades válidas.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Entity(u64);

impl Entity {
    const INDEX_MASK: u64 = 0xFFFF_FFFF;
    const GENERATION_SHIFT: u32 = 32;

    /// Empaqueta índice y generación en un solo `u64`.
    #[inline]
    const fn from_parts(index: u32, generation: u32) -> Self {
        Self(((generation as u64) << Self::GENERATION_SHIFT) | index as u64)
    }

    /// Índice de 32 bits, sin los bits de generación.
    #[inline]
    pub const fn index(self) -> u32 {
        (self.0 & Self::INDEX_MASK) as u32
    }

    /// Generación de 32 bits, sin los bits de índice.
    #[inline]
    pub const fn generation(self) -> u32 {
        (self.0 >> Self::GENERATION_SHIFT) as u32
    }
}

/// Administrador del ciclo de vida de las entidades.
///
/// Reutiliza índices mediante una pila (LIFO) e invalida las referencias viejas
/// incrementando la generación de la ranura al destruir.
#[derive(Default)]
pub struct EntityManager {
    /// Generación actual de cada ranura. La posición es el índice de la entidad.
    generations: Vec<u32>,

    /// Pila de índices libres listos para reutilizarse.
    free_indices: Vec<u32>,
}

impl EntityManager {
    /// Generación reservada para ranuras agotadas. Ninguna entidad viva puede tenerla:
    /// cuando una ranura llega a este valor se retira y no se vuelve a reciclar,
    /// así la generación nunca da la vuelta (evita el problema ABA).
    const RETIRED_GENERATION: u32 = u32::MAX;

    /// Crea un administrador vacío.
    pub fn new() -> Self {
        Self::default()
    }

    /// Crea una entidad, reciclando una ranura libre si existe.
    ///
    /// # Panics
    /// Si se supera el número máximo de ranuras (`u32::MAX`).
    pub fn create_entity(&mut self) -> Entity {
        if let Some(index) = self.free_indices.pop() {
            Entity::from_parts(index, self.generations[index as usize])
        } else {
            let index = u32::try_from(self.generations.len())
                .expect("se superó el máximo de entidades (u32::MAX)");
            self.generations.push(0);
            Entity::from_parts(index, 0)
        }
    }

    /// Destruye una entidad viva. Devuelve `false` si la referencia no es válida
    /// (índice fuera de rango, generación distinta o ya destruida).
    ///
    /// Si la ranura llega a la generación reservada, se retira definitivamente
    /// en lugar de volver a `free_indices`.
    pub fn destroy_entity(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        let index = entity.index();
        // Seguro: una entidad viva siempre tiene generación < RETIRED_GENERATION.
        let next = entity.generation() + 1;
        self.generations[index as usize] = next;

        if next != Self::RETIRED_GENERATION {
            self.free_indices.push(index);
        }

        true
    }

    /// Indica si la entidad sigue viva.
    pub fn is_alive(&self, entity: Entity) -> bool {
        let generation = entity.generation();

        generation != Self::RETIRED_GENERATION
            && self
            .generations
            .get(entity.index() as usize)
            .is_some_and(|&g| g == generation)
    }
}


#[cfg(test)]
mod test;