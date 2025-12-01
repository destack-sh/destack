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

/// Dispatch key for runtime type-based selection.
/// Captures the types we check at runtime to select an overload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DispatchKey {
    /// Single type dispatch (for a single variable).
    Single { ty: LocalTypeId },
    /// Multiple type dispatch (for multiple variables).
    Multiple { types: Vec<LocalTypeId> },
}

impl DispatchKey {
    /// Create a dispatch key from one or more types.
    pub fn new(types: &[LocalTypeId]) -> Self {
        if types.len() == 1 {
            Self::Single { ty: types[0] }
        } else {
            Self::Multiple { types: types.to_vec() }
        }
    }

    /// Create a dispatch key for a single type.
    pub fn single(ty: LocalTypeId) -> Self {
        Self::Single { ty }
    }

    /// Get the types as a slice.
    pub fn types(&self) -> &[LocalTypeId] {
        match self {
            Self::Single { ty } => std::slice::from_ref(ty),
            Self::Multiple { types } => types,
        }
    }

    /// Check if this is a single-type key.
    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single { .. })
    }
}

/// Resolution of an overload/method/operator at some usage site.
///
/// Each variant captures:
/// - **Static part**: The receiver type (if any) that selected the "family" of overloads.
/// - **Dynamic part**: The dispatch key(s) for runtime selection within that family.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// Resolution failed: some or all type combinations had no valid overload.
    Unresolved {
        /// The receiver type (None for free functions).
        receiver: Option<LocalTypeId>,
        /// Keys we couldn't find overloads for.
        missing_keys: Vec<DispatchKey>,
        /// Candidates we did find (for "did you mean?" suggestions).
        found_candidates: Vec<ResolutionCandidate>,
    },
    /// Builtin primitive operation (no symbol needed, built-in handles it).
    Builtin {
        /// The receiver type (None for free functions).
        receiver: Option<LocalTypeId>,
    },
    /// Static resolution: exactly one target, known at compile time.
    Static {
        /// The receiver type (None for free functions).
        receiver: Option<LocalTypeId>,
        /// The resolved candidate.
        candidate: ResolutionCandidate,
    },
    /// Dynamic resolution: runtime dispatch needed based on types.
    Dynamic {
        /// The receiver type (None for free functions).
        receiver: Option<LocalTypeId>,
        /// The candidates to dispatch between.
        candidates: Vec<ResolutionCandidate>,
    },
}

/// Candidate for dispatch resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolutionCandidate {
    /// The dispatch key that selects this candidate at runtime.
    /// For `a + b` where `b: number`, key is `Single { number }`.
    /// For `foo(x, y)` where `x: string, y: int`, key is `Multiple { [string, int] }`.
    pub key: DispatchKey,
    /// The resolved symbol for this candidate.
    pub symbol: GlobalSymbolId,
    /// The instance of the symbol, if it was generically instantiated.
    pub instance: Option<LocalInstanceId>,
}
