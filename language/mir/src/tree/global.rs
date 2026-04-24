use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Constant, Mutability, Node, NodeType, TypeReference};

/// Symbol linkage (visibility and definition location).
///
/// Controls how a symbol (function or global) is linked:
/// - Where it's defined (here or elsewhere)
/// - Who can see it (local to module or exported)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Linkage {
    /// Defined here, not visible outside the module (private).
    /// This is the default.
    #[default]
    Local,
    /// Defined here, visible outside the module (public).
    Export,
    /// Declared here but defined elsewhere (imported).
    Import,
}

impl Linkage {
    /// Returns true if the symbol is defined in this module.
    pub fn is_defined(&self) -> bool {
        matches!(self, Linkage::Local | Linkage::Export)
    }

    /// Returns true if the symbol is visible outside the module.
    pub fn is_exported(&self) -> bool {
        matches!(self, Linkage::Export)
    }

    /// Returns true if the symbol is imported from elsewhere.
    pub fn is_import(&self) -> bool {
        matches!(self, Linkage::Import)
    }
}

/// Global data definition (module-level variable or constant).
///
/// Globals can be mutable (variable) or immutable (constant).
/// - Mutable globals: module-level state, like `static mut` in Rust
/// - Immutable globals: constant data, like string literals or lookup tables
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Global {
    /// Name for linking and debugging.
    pub name: StringId,
    /// The type of the global.
    pub ty: TypeReference,
    /// Whether this global is mutable.
    pub mutability: Mutability,
    /// Linkage (local, export, or import).
    pub linkage: Linkage,
    /// Initial value. None for imported globals.
    pub initializer: Option<GlobalInitializer>,
}

impl Node for Global {
    const TYPE: NodeType = NodeType::Global;
}

impl Global {
    /// Create a new local (private) global.
    pub fn new(
        name: StringId,
        ty: TypeReference,
        mutability: Mutability,
        init: GlobalInitializer,
    ) -> Self {
        Self {
            name,
            ty,
            mutability,
            linkage: Linkage::Local,
            initializer: Some(init),
        }
    }

    /// Create a mutable global (variable), local by default.
    pub fn variable(name: StringId, ty: TypeReference, init: GlobalInitializer) -> Self {
        Self::new(name, ty, Mutability::Mutable, init)
    }

    /// Create an immutable global (constant), local by default.
    pub fn constant(name: StringId, ty: TypeReference, init: GlobalInitializer) -> Self {
        Self::new(name, ty, Mutability::Immutable, init)
    }

    /// Create an imported global declaration (no initializer).
    pub fn import(name: StringId, ty: TypeReference, mutability: Mutability) -> Self {
        Self {
            name,
            ty,
            mutability,
            linkage: Linkage::Import,
            initializer: None,
        }
    }

    /// Set the linkage and return self (builder pattern).
    pub fn with_linkage(mut self, linkage: Linkage) -> Self {
        self.linkage = linkage;
        self
    }

    /// Check if this global is mutable.
    pub fn is_mutable(&self) -> bool {
        self.mutability == Mutability::Mutable
    }

    /// Check if this global is imported (defined elsewhere).
    pub fn is_import(&self) -> bool {
        self.linkage.is_import()
    }
}

/// Initializer for global data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GlobalInitializer {
    /// Zero-initialized (all bytes zero).
    Zero,
    /// Scalar constant (bool, int, float).
    Scalar(Constant),
    /// String literal data (as bytes).
    String(String),
    /// Raw bytes (blobs).
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

    /// Create from a string literal.
    pub fn string(s: &str) -> Self {
        Self::String(s.to_string())
    }

    /// Create an aggregate initializer.
    pub fn aggregate(elements: Vec<GlobalInitializer>) -> Self {
        Self::Aggregate(elements)
    }
}
