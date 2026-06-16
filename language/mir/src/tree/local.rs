use serde::{Deserialize, Serialize};

use crate::{Mutability, Node, NodeType, TypeReference};

/// Local variable (stack slot) in a function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Local {
    /// The type of the value stored in this slot.
    pub ty: TypeReference,
    /// Whether this local can be mutated after initialization.
    pub mutability: Mutability,
}

impl Node for Local {
    const TYPE: NodeType = NodeType::Local;
}

impl Local {
    /// Create a new local.
    pub fn new(ty: TypeReference, mutability: Mutability) -> Self {
        Self { ty, mutability }
    }

    /// Create a mutable local.
    pub fn mutable(ty: TypeReference) -> Self {
        Self {
            ty,
            mutability: Mutability::Mutable,
        }
    }

    /// Create an immutable local.
    pub fn immutable(ty: TypeReference) -> Self {
        Self {
            ty,
            mutability: Mutability::Immutable,
        }
    }
}
