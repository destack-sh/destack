use crate::{GlobalSymbolId, LocalInstanceId, LocalTypeId, ModuleId};

/// Unique identifier for Resolutions.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalResolutionId(pub u32);

impl LocalResolutionId {
    /// Wrap an id as a ResolutionId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalResolutionId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalResolutionId {
        GlobalResolutionId {
            module_id,
            local_id: self,
        }
    }
}

/// Global resolution id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalResolutionId {
    /// The module id of the global resolution.
    pub module_id: ModuleId,
    /// The local id of the global resolution.
    pub local_id: LocalResolutionId,
}

impl GlobalResolutionId {
    /// Create a new global resolution id.
    pub fn new(module_id: ModuleId, local_id: LocalResolutionId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalResolutionId.
    pub fn into_local(self) -> LocalResolutionId {
        self.local_id
    }
}

impl From<GlobalResolutionId> for LocalResolutionId {
    fn from(id: GlobalResolutionId) -> Self {
        id.local_id
    }
}

impl std::fmt::Display for LocalResolutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.0)
    }
}

/// Resolution of an overload/method/operator at some usage site.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
    /// The id of the Resolution.
    pub id: LocalResolutionId,
    /// The kind of resolution.
    pub kind: ResolutionKind,
}

/// The kind of resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolutionKind {
    /// Resolution failed: we tried to resolve but couldn't find a valid target.
    Unresolved {
        ty: LocalTypeId,
        symbol: GlobalSymbolId,
    },
    /// Builtin operation (primitive arithmetic, etc.) - no symbol needed.
    Builtin {
        ty: LocalTypeId,
        symbol: GlobalSymbolId,
    },
    /// Static resolution, i.e., we know exactly which overload/method/operator to call at compile time.
    Static { candidate: ResolutionCandidate },
    /// Dynamic resolution, i.e., the target depends on runtime types (e.g., union dispatch).
    Dynamic {
        /// The possible candidates for overload/method/operator dispatch.
        candidates: Vec<ResolutionCandidate>,
    },
}

/// Candidate for dynamic dispatch resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionCandidate {
    /// The type that this candidate handles (one arm of a union, typically).
    pub ty: LocalTypeId,
    /// The resolved symbol for this type.
    pub symbol: GlobalSymbolId,
    /// The instance of the symbol, if it was generically instantiated.
    pub instance: Option<LocalInstanceId>,
}
