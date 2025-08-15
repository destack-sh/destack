//! destack.core.builtin.entity@2025.08.14.0

#![destack::partial(destack.core.builtin.entity, file)]

#[destack::generated(Materialization, enum, block)]
/// The materialization level of an Entity.
pub enum Materialization {
    /// Entity matches its definition, only exists when queried
    VIRTUAL = 1,
    /// Entity is a partial override of its definition
    PARTIAL = 2,
    /// Entity is a full copy of its definition
    FULL = 3,
    /// Entity is its own root (no other definition)
    ROOT = 4
}

#[destack::generated(ProcessFlag, enum, block)]
/// How an Entity should be treated for processing by the system.
pub enum ProcessFlag {
    DEFAULT = 0,
    /// Entity is (soft) deleted.
    DELETED = 1,
    /// Entity is inactive (i.e. paused).
    INACTIVE = 2,
    /// Entity is inactive to InputEvents.
    INACTIVE_INPUT = 4,
    /// Entity is sleeping (i.e. not processing).
    SLEEPING = 8,
    /// Entity is sleeping to InputEvents.
    SLEEPING_INPUT = 16
}

#[destack::generated(ExtensionFlag, enum, block)]
/// How an Entity should be treated for extension by the system.
pub enum ExtensionFlag {
    DEFAULT = 0,
    INSTANTIABLE = 1,
    EXTENSIBLE = 2
}