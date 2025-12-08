//! MIR global data definitions.

use destack_source::StringId;

use crate::{Constant, LocalNodeId, Mutability, Node, NodeType, Type};

/// Global data definition (module-level variable or constant, may be external).
///
/// Globals can be mutable (variable) or immutable (constant).
/// - Mutable globals: module-level state, like `static mut` in Rust
/// - Immutable globals: constant data, like string literals or lookup tables
#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    /// Name for linking and debugging.
    pub name: StringId,
    /// The type of the global.
    pub ty: LocalNodeId<Type>,
    /// Whether this global is mutable.
    pub mutability: Mutability,
    /// Whether this global is external (declared but not defined here).
    pub is_external: bool,
    /// Initial value. None for external globals.
    pub initializer: Option<GlobalInitializer>,
}

impl Node for Global {
    const TYPE: NodeType = NodeType::Global;
}

impl Global {
    /// Create a new global.
    pub fn new(
        name: StringId,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
        init: GlobalInitializer,
    ) -> Self {
        Self {
            name,
            ty,
            mutability,
            is_external: false,
            initializer: Some(init),
        }
    }

    /// Create a mutable global (variable).
    pub fn variable(name: StringId, ty: LocalNodeId<Type>, init: GlobalInitializer) -> Self {
        Self::new(name, ty, Mutability::Mutable, init)
    }

    /// Create an immutable global (constant).
    pub fn constant(name: StringId, ty: LocalNodeId<Type>, init: GlobalInitializer) -> Self {
        Self::new(name, ty, Mutability::Immutable, init)
    }

    /// Create an external global declaration (no initializer).
    pub fn external(name: StringId, ty: LocalNodeId<Type>, mutability: Mutability) -> Self {
        Self {
            name,
            ty,
            mutability,
            is_external: true,
            initializer: None,
        }
    }

    /// Check if this global is mutable.
    pub fn is_mutable(&self) -> bool {
        self.mutability == Mutability::Mutable
    }
}

/// Initializer for global data.
#[derive(Debug, Clone, PartialEq)]
pub enum GlobalInitializer {
    /// Zero-initialized (all bytes zero).
    Zero,
    /// Scalar constant (bool, int, float).
    Scalar(Constant),
    /// Raw bytes (strings, blobs).
    Bytes(Vec<u8>),
    /// Aggregate (array/struct fields).
    Aggregate(Vec<GlobalInitializer>),
}

impl GlobalInitializer {
    /// Create a zero initializer.
    pub fn zero() -> Self {
        Self::Zero
    }

    /// Create from a scalar constant.
    pub fn scalar(value: Constant) -> Self {
        Self::Scalar(value)
    }

    /// Create from raw bytes.
    pub fn bytes(data: Vec<u8>) -> Self {
        Self::Bytes(data)
    }

    /// Create from a string (UTF-8 bytes).
    pub fn string(s: &str) -> Self {
        Self::Bytes(s.as_bytes().to_vec())
    }

    /// Create an aggregate initializer.
    pub fn aggregate(elements: Vec<GlobalInitializer>) -> Self {
        Self::Aggregate(elements)
    }
}
