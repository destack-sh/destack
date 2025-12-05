//! MIR local variables (stack slots).

use crate::{LocalNodeId, Node, NodeType, Type};

/// Mutability of a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutability {
    /// Immutable (const).
    Immutable,
    /// Mutable (var).
    Mutable,
}

/// Ownership semantics of a local or parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ownership {
    /// Owned value (this binding owns the data and is responsible for dropping it).
    Owned,
    /// Borrowed reference (this binding borrows data owned elsewhere).
    Borrowed,
    /// Copy (this is a copy of a value, no ownership transfer, no drop needed).
    Copy,
}

/// A local variable (stack slot) in a function.
/// - May be mutated
/// - Have their address taken
/// - Need to persist across basic blocks
#[derive(Debug, Clone, PartialEq)]
pub struct Local {
    /// Type of the local.
    pub ty: LocalNodeId<Type>,
    /// Whether the local is mutable.
    pub mutability: Mutability,
    /// Ownership semantics.
    pub ownership: Ownership,
}

impl Node for Local {
    const TYPE: NodeType = NodeType::Local;
}

impl Local {
    /// Create a new local.
    pub fn new(ty: LocalNodeId<Type>, mutability: Mutability, ownership: Ownership) -> Self {
        Self {
            ty,
            mutability,
            ownership,
        }
    }

    /// Create an owned, mutable local (common case for `var` bindings).
    pub fn owned_mutable(ty: LocalNodeId<Type>) -> Self {
        Self {
            ty,
            mutability: Mutability::Mutable,
            ownership: Ownership::Owned,
        }
    }

    /// Create an owned, immutable local (common case for `const` bindings).
    pub fn owned_immutable(ty: LocalNodeId<Type>) -> Self {
        Self {
            ty,
            mutability: Mutability::Immutable,
            ownership: Ownership::Owned,
        }
    }

    /// Create a borrowed local (for references).
    pub fn borrowed(ty: LocalNodeId<Type>, mutability: Mutability) -> Self {
        Self {
            ty,
            mutability,
            ownership: Ownership::Borrowed,
        }
    }
}
