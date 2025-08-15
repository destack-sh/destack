//! destack.core.builtin.entity@2025.08.15.1

#![destack::partial(destack.core.builtin.entity, file)]

#[destack::generated(Materialization, enum, block)]
/// The materialization level of an Entity.
pub enum Materialization {
    /// Entity matches its definition, only exists when queried
    Virtual = 1,
    /// Entity is a partial override of its definition
    Partial = 2,
    /// Entity is a full copy of its definition
    Full = 3,
    /// Entity is its own root (no other definition)
    Root = 4,
}

#[destack::generated(ProcessFlag, enum, block)]
/// How an Entity should be treated for processing by the system.
pub enum ProcessFlag {
    Default = 0,
    /// Entity is (soft) deleted.
    Deleted = 1,
    /// Entity is inactive (i.e. paused).
    Inactive = 2,
    /// Entity is inactive to InputEvents.
    InactiveInput = 4,
    /// Entity is sleeping (i.e. not processing).
    Sleeping = 8,
    /// Entity is sleeping to InputEvents.
    SleepingInput = 16,
}

#[destack::generated(ExtensionFlag, enum, block)]
/// How an Entity should be treated for extension by the system.
pub enum ExtensionFlag {
    Default = 0,
    Instantiable = 1,
    Extensible = 2,
}
