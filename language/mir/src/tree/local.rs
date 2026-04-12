use serde::{Deserialize, Serialize};

use crate::{Mutability, Node, NodeType, TypeReference};

/// Ownership semantics of a local or parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ownership {
    /// Owned value (this binding owns the data and is responsible for dropping it).
    Owned,
    /// Borrowed reference (this binding borrows data owned elsewhere).
    Borrowed,
    /// Copy (this is a copy of a value, no ownership transfer, no drop needed).
    Copy,
}

/// Local variable (stack slot) in a function.
/// - May be mutated
/// - Have their address taken
/// - Need to persist across basic blocks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Local {
    /// The type of the value stored in this slot.
    pub ty: TypeReference,
    /// Whether this local can be mutated after initialization.
    pub mutability: Mutability,
    /// Ownership semantics (owned, borrowed, or copy).
    pub ownership: Ownership,
}

impl Node for Local {
    const TYPE: NodeType = NodeType::Local;
}

impl Local {
    /// Create a new local.
    pub fn new(ty: TypeReference, mutability: Mutability, ownership: Ownership) -> Self {
        Self {
            ty,
            mutability,
            ownership,
        }
    }

    /// Create an owned, mutable local (common case for `var` bindings).
    pub fn owned_mutable(ty: TypeReference) -> Self {
        Self {
            ty,
            mutability: Mutability::Mutable,
            ownership: Ownership::Owned,
        }
    }

    /// Create an owned, immutable local (common case for `const` bindings).
    pub fn owned_immutable(ty: TypeReference) -> Self {
        Self {
            ty,
            mutability: Mutability::Immutable,
            ownership: Ownership::Owned,
        }
    }

    /// Create a borrowed local (for references).
    pub fn borrowed(ty: TypeReference, mutability: Mutability) -> Self {
        Self {
            ty,
            mutability,
            ownership: Ownership::Borrowed,
        }
    }
}
