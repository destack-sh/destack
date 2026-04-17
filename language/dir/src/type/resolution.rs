use destack_source::{AdaptImage, ModuleId};
use serde::{Deserialize, Serialize};

use crate::{GlobalSymbolId, LocalInstanceId, LocalTypeId, StaticArgument};

/// Unique identifier for Resolutions.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage,
)]
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

/// Dispatch key for runtime type-based overload selection.
///
/// Used when multiple overloads exist and the specific implementation
/// must be selected based on argument types (potentially at runtime for unions).
/// For simple lookups (member access, field access), no dispatch key is needed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, AdaptImage)]
pub enum DispatchKey {
    /// Single type dispatch (e.g., `a + b` dispatches on type of `b`).
    Single { ty: LocalTypeId },
    /// Multiple type dispatch (e.g., `foo(x, y)` dispatches on types of both args).
    Multiple { types: Vec<LocalTypeId> },
}

impl DispatchKey {
    /// Create a dispatch key from one or more types.
    pub fn new(types: &[LocalTypeId]) -> Self {
        if types.len() == 1 {
            Self::Single { ty: types[0] }
        } else {
            Self::Multiple {
                types: types.to_vec(),
            }
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

/// Resolution of a symbol lookup at a usage site.
/// Member access like `a.foo` resolves to the member symbol.
/// Field access in patterns like `{ x }` resolves to the field symbol.
/// Call and operator dispatch like `a + b` or `foo(x)` resolve to overloads.
///
/// The receiver type identifies the family of implementations.
/// Dispatch cases carry keys for runtime selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Resolution {
    /// Resolution failed: couldn't find a valid target.
    Unresolved {
        /// The receiver type (None for free functions/lookups).
        receiver: Option<LocalTypeId>,
        /// Dispatch keys we couldn't find overloads for (empty for simple lookups).
        missing_keys: Vec<DispatchKey>,
        /// Candidates we did find (for "did you mean?" suggestions).
        candidates: Vec<ResolutionCandidate>,
    },
    /// Builtin primitive operation (no symbol needed, codegen handles it).
    Builtin {
        /// The receiver type.
        receiver: Option<LocalTypeId>,
    },
    /// Static resolution: exactly one target, known at compile time.
    /// Used for both simple lookups (member, field) and single-overload calls.
    /// A static target may still dispatch via vtable if the symbol is virtual.
    Static {
        /// The receiver type (None for free functions/lookups).
        receiver: Option<LocalTypeId>,
        /// The resolved candidate.
        candidate: ResolutionCandidate,
    },
    /// Dynamic resolution: runtime dispatch needed based on argument types.
    /// Used when different union members resolve to different target symbols or instances.
    Dynamic {
        /// The receiver type (the union type).
        receiver: Option<LocalTypeId>,
        /// The candidates to dispatch between at runtime.
        candidates: Vec<ResolutionCandidate>,
    },
}

impl Resolution {
    /// Get the receiver type.
    pub fn receiver(&self) -> Option<LocalTypeId> {
        match self {
            Resolution::Unresolved { receiver, .. } => *receiver,
            Resolution::Builtin { receiver, .. } => *receiver,
            Resolution::Static { receiver, .. } => *receiver,
            Resolution::Dynamic { receiver, .. } => *receiver,
        }
    }
}

/// The resolved call signature used at a dispatch site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct ResolvedSignature {
    /// The dynamic parameter types after static substitutions.
    pub parameters: Vec<LocalTypeId>,
    /// The return type after static substitutions.
    pub return_type: Option<LocalTypeId>,
    /// The resolved static arguments in declared order.
    pub generic_arguments: Vec<StaticArgument>,
}

/// A resolved target symbol, optionally with dispatch information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct ResolutionCandidate {
    /// The dispatch key for runtime overload selection.
    /// Use `None` for simple lookups such as member or field access.
    /// Use `Some(Single { ty })` for single argument dispatch like `a + b` on `b` type.
    /// Use `Some(Multiple { types })` for multi argument dispatch.
    pub key: Option<DispatchKey>,
    /// The resolved target symbol.
    pub target_symbol: GlobalSymbolId,
    /// The instance of the symbol, if generically instantiated.
    pub instance: Option<LocalInstanceId>,
    /// The resolved call signature when available.
    pub resolved_signature: Option<ResolvedSignature>,
}
