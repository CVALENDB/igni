/// Representa un identificador único de entidad en el ECS.
///
/// Internamente, utiliza un valor de 64 bits (`u64`) empaquetado mediante operaciones de bits:
/// - Los **32 bits superiores** almacenan la **generación** (versión o ciclo de vida del índice).
/// - Los **32 bits inferiores** almacenan el **índice** base para el acceso a los datos.
///
/// Este diseño previene el problema de referencias colgantes (*dangling references* o problema ABA)
/// cuando se reciclan identificadores de entidades que ya han sido destruidas.
pub struct Entity(pub u64);

impl Entity {
    /// Extrae y devuelve únicamente el índice de 32 bits de la entidad,
    /// ignorando los bits de generación.
    ///
    /// Este índice se utiliza internamente en el motor para realizar búsquedas directas
    /// o accesos seguros a los contenedores de componentes y arquetipos.
    pub fn index(self) -> u32 {
        (self.0 & EntityManager::INDEX_MASK) as u32
    }
}

/// Administrador encargado de controlar el ciclo de vida de las entidades en el ECS.
/// Gestiona la creación, destrucción, el reciclaje eficiente de índices mediante una pila
/// y la validación de generaciones para garantizar la seguridad de las referencias.
pub struct EntityManager {
    /// Vector que almacena la generación actual de cada ranura de índice.
    /// La posición en el vector corresponde al índice de la entidad, y su valor numérico
    /// indica cuántas veces ha sido reciclada o reutilizada dicha ranura.
    generations: Vec<u32>,

    /// Pila de índices que han quedado libres tras destruir entidades previas.
    /// Permite reutilizar posiciones de memoria de manera óptima sin inflar el tamaño de las listas.
    free_indices: Vec<u32>,
}

impl EntityManager {
    /// Máscara de bits para aislar los 32 bits inferiores correspondientes al índice (`0xFFFF_FFFF`).
    pub const INDEX_MASK: u64 = 0xFFFF_FFFF;

    /// Desplazamiento de bits (`32`) necesario para ubicar o extraer la generación en la mitad superior del `u64`.
    pub const GENERATION_SHIFT: u32 = 32;

    /// Crea e inicializa una nueva instancia vacía del administrador de entidades.
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_indices: Vec::new(),
        }
    }

    /// Crea una nueva entidad o recicla una ranura previamente liberada.
    ///
    /// - Si existen índices disponibles en la pila `free_indices`, extrae uno y conserva
    ///   su generación actual (la cual ya fue incrementada de forma segura al destruirse).
    /// - Si no hay índices libres, extiende el vector de generaciones agregando un nuevo
    ///   espacio con una generación inicial de `0`.
    ///
    /// Retorna una instancia de `Entity` con el bitmask empaquetado de generación e índice.
    pub fn create_entity(&mut self) -> Entity {
        let (index, generation) = if let Some(free_idx) = self.free_indices.pop() {
            (free_idx, self.generations[free_idx as usize])
        } else {
            let new_idx = self.generations.len() as u32;
            self.generations.push(0);
            (new_idx, 0)
        };

        let bits = ((generation as u64) << Self::GENERATION_SHIFT) | (index as u64);
        Entity(bits)
    }

    /// Destruye una entidad activa en el sistema de manera segura.
    ///
    /// Válida que la entidad sea legítima antes de destruirla:
    /// 1. Extrae el índice y la generación del `u64` de la entidad.
    /// 2. Comprueba que el índice se encuentre dentro de los límites del vector.
    /// 3. Verifica que la generación de la entidad coincida exactamente con la registrada
    ///    en el administrador (evitando dobles eliminaciones o el uso de referencias viejas).
    ///
    /// Si la validación es exitosa, incrementa la generación de esa ranura (lo que invalida
    /// de inmediato cualquier referencia antigua que la poseyera), añade el índice a `free_indices`
    /// y retorna `true`. Si falla, retorna `false`.
    pub fn destroy_entity(&mut self, entity: Entity) -> bool {
        let index = (entity.0 & Self::INDEX_MASK) as u32;
        let generation = (entity.0 >> Self::GENERATION_SHIFT) as u32;

        let idx_usize = index as usize;

        if idx_usize >= self.generations.len() || self.generations[idx_usize] != generation {
            return false;
        }

        self.generations[idx_usize] += 1;
        self.free_indices.push(index);

        true
    }

    /// Comprueba si una referencia a una entidad sigue estando viva y activa.
    ///
    /// Extrae el índice y la generación de la entidad y confirma que el índice
    /// exista dentro del almacenamiento y que su generación coincida exactamente
    /// con la versión actual guardada en el administrador.
    ///
    /// Retorna `true` si la entidad es completamente válida y activa, o `false` en caso contrario.
    pub fn is_alive(&self, entity: Entity) -> bool {
        let index = (entity.0 & Self::INDEX_MASK) as u32;
        let generation = (entity.0 >> Self::GENERATION_SHIFT) as u32;

        let idx_usize = index as usize;

        idx_usize < self.generations.len() && self.generations[idx_usize] == generation
    }
}