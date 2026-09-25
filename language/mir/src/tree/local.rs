use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Mutability, Node, NodeType, TypeId};

/// Local variable (stack slot) in a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Local {
    /// The type of the value stored in this slot.
    pub ty: TypeId,
    /// Whether this local can be mutated after initialization.
    pub mutability: Mutability,
}

impl Node for Local {
    const TYPE: NodeType = NodeType::Local;
}

impl Local {
    /// Create a new local.
    pub fn new(ty: TypeId, mutability: Mutability) -> Self {
        Self { ty, mutability }
    }

    /// Create a mutable local.
    pub fn mutable(ty: TypeId) -> Self {
        Self {
            ty,
            mutability: Mutability::Mutable,
        }
    }

    /// Create an immutable local.
    pub fn immutable(ty: TypeId) -> Self {
        Self {
            ty,
            mutability: Mutability::Immutable,
        }
    }
}
