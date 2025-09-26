use destack_uuid::Uuid;

/// An Entity Epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Epoch(pub u64);

impl Epoch {
    pub const ZERO: Epoch = Epoch(0);
}

/// The materialization level of an Entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Materialization {
    Virtual = 0,
    Partial = 1,
    Full = 2,
    Root = 3,
}

/// How an Entity should be treated for processing by the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessFlag(pub u8);

impl ProcessFlag {
    pub const DEFAULT: ProcessFlag = ProcessFlag(0);
    pub const DELETED: ProcessFlag = ProcessFlag(1);
    pub const INACTIVE: ProcessFlag = ProcessFlag(2);
    pub const INACTIVE_INPUT: ProcessFlag = ProcessFlag(3);
    pub const SLEEPING: ProcessFlag = ProcessFlag(4);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtensionFlag(pub u8);

impl ExtensionFlag {
    pub const DEFAULT: ExtensionFlag = ExtensionFlag(0);
    pub const INSTANTIABLE: ExtensionFlag = ExtensionFlag(1);
    pub const EXTENSIBLE: ExtensionFlag = ExtensionFlag(2);
}

/// A reference to an Entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reference {
    /// The universally unique identifier of this Entity.
    pub id: Uuid,
    /// The Space this Entity is in.
    pub space_id: Uuid,
    /// The Branch this Entity is part of (in its Space).
    pub branch_id: Uuid,
}

impl Reference {
    pub fn new(id: Uuid, space_id: Uuid, branch_id: Uuid) -> Self {
        Self {
            id,
            space_id,
            branch_id,
        }
    }
}

/// An Identity is globally unique identifier for an Entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identity {
    /// The universally unique identifier of this Entity.
    pub id: Uuid,
    /// The Space the Entity is in.
    pub space: Reference,
    /// The Branch this Entity is part of (in its Space).
    pub branch: Reference,
}

/// A named, versioned, mutable object in the system.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// The materialization level of the `Entity`.
    pub materialization: Materialization,
    /// The Snapshot the `Entity` is part of (in its `Space` and `Branch`).
    pub snapshot: Reference,
    /// The definition the `Entity` is an instance of.
    pub definition: Option<Reference>,
    /// The (root) `Entity` that is being instantiated.
    pub instance: Option<Reference>,
    /// The logical time the `Entity` was created.
    pub created_at: Epoch,
    /// The logical time the `Entity` was last updated.
    pub updated_at: Epoch,
    /// The exclusive owner of the `Entity` (the authority on access).
    pub owner: Reference,
    /// The exclusive controller of the `Entity` (the authority on changes).
    pub controller: Reference,
    /// The process flags of the `Entity`.
    pub processing: ProcessFlag,
    /// The parent of the `Entity`. Most `Entity`s can be attached to any other `Entity`.
    pub parent: Option<Reference>,
    /// The name of the `Entity`.
    pub name: String,
    /// The key of the `Entity` (for reconciliation and querying).
    pub key: Option<String>,
    /// The extension flags of the `Entity`.
    pub extensibility: ExtensionFlag,
    /// The Script of the `Entity`.
    pub script: Option<Reference>,
    /// The identity of the `Entity`.
    pub identity: Identity,
}
