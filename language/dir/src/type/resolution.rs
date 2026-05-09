use serde::{Deserialize, Serialize};

use crate::{DependencyTarget, GlobalSymbolId, LocalInstantiationId, LocalTypeId, StaticArgument};

/// Semantic target selected for one DIR node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Resolution {
    /// Lexical symbol binding.
    Symbol(GlobalSymbolId),
    /// Dependency binding.
    Dependency(DependencyResolution),
    /// Control flow target binding.
    Control(ControlResolution),
    /// Type directed implementation binding.
    Dispatch(DispatchResolution),
}

/// Target selected by a dependency edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyResolution {
    /// A module import target.
    Module(DependencyTarget),
    /// A resolved exported or imported binding.
    Symbol(GlobalSymbolId),
}

/// Target selected by a control flow transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlResolution {
    /// A labelled control target.
    Label(GlobalSymbolId),
    /// The nearest loop target.
    Loop,
    /// The nearest function target.
    Function,
}

/// Type-directed target selected at a usage site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DispatchResolution {
    /// Builtin primitive operation.
    Builtin {
        /// The receiver type.
        receiver: Option<LocalTypeId>,
    },
    /// Exactly one target known at compile time.
    Static {
        /// The receiver type.
        receiver: Option<LocalTypeId>,
        /// The selected target.
        target: DispatchTarget,
    },
    /// Runtime selection between typed targets.
    Dynamic {
        /// The receiver type.
        receiver: Option<LocalTypeId>,
        /// The selected targets.
        targets: Vec<DispatchTarget>,
    },
}

impl DispatchResolution {
    /// Return the receiver type.
    pub fn receiver(&self) -> Option<LocalTypeId> {
        match self {
            DispatchResolution::Builtin { receiver } => *receiver,
            DispatchResolution::Static { receiver, .. } => *receiver,
            DispatchResolution::Dynamic { receiver, .. } => *receiver,
        }
    }
}

/// Key used when runtime type dispatch needs more than the receiver.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DispatchKey {
    /// Single type dispatch.
    Single { ty: LocalTypeId },
    /// Multiple type dispatch.
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

    /// Return the key types.
    pub fn types(&self) -> &[LocalTypeId] {
        match self {
            Self::Single { ty } => std::slice::from_ref(ty),
            Self::Multiple { types } => types,
        }
    }

    /// Return true when this is a single-type key.
    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single { .. })
    }
}

/// Selected call signature used at a dispatch site.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchSignature {
    /// The dynamic parameter types after static substitutions.
    pub parameters: Vec<LocalTypeId>,
    /// The return type after static substitutions.
    pub return_type: Option<LocalTypeId>,
    /// The selected static arguments in declared order.
    pub generic_arguments: Vec<StaticArgument>,
}

/// Selected dispatch target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchTarget {
    /// The dispatch key for runtime overload selection.
    pub key: Option<DispatchKey>,
    /// The selected target symbol.
    pub symbol: GlobalSymbolId,
    /// The instantiation of the symbol, if statically applied.
    pub instantiation: Option<LocalInstantiationId>,
    /// The selected call signature when available.
    pub signature: Option<DispatchSignature>,
}
