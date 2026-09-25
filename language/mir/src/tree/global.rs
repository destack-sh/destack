use serde::{Deserialize, Serialize};
use tspp_core::StringId;
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{Constant, FunctionId, GlobalId, Mutability, Node, NodeType, Space, Symbol, TypeId};

/// Symbol linkage (visibility and definition location).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum Linkage {
    /// Defined here, not visible outside the module (private).
    #[default]
    Local,
    /// Defined here, visible outside the module (public).
    Export,
    /// Declared here but defined elsewhere (imported).
    Import,
    /// Defined here and in every module instantiating it, deduplicated by the linker.
    Shared,
}

impl Linkage {
    /// Returns true if the symbol is defined in this module.
    pub fn is_defined(&self) -> bool {
        matches!(self, Linkage::Local | Linkage::Export | Linkage::Shared)
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

/// One module-level global or Program constant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Global {
    /// Name for linking and debugging.
    pub name: StringId,
    /// The global's persistent mangled symbol: its linkable identity.
    pub symbol: Symbol,
    /// The type of the global.
    pub ty: TypeId,
    /// Whether this global is mutable.
    pub mutability: Mutability,
    /// The space that owns this global.
    pub space: Space,
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
        module: ModuleId,
        name: StringId,
        ty: TypeId,
        mutability: Mutability,
        init: GlobalInitializer,
    ) -> Self {
        Self {
            name,
            symbol: Symbol::named(module, name),
            ty,
            mutability,
            space: Space::Local,
            linkage: Linkage::Local,
            initializer: Some(init),
        }
    }

    /// Create a mutable global (variable), local by default.
    pub fn variable(module: ModuleId, name: StringId, ty: TypeId, init: GlobalInitializer) -> Self {
        Self::new(module, name, ty, Mutability::Mutable, init)
    }

    /// Create an immutable program constant.
    pub fn constant(module: ModuleId, name: StringId, ty: TypeId, init: GlobalInitializer) -> Self {
        let mut global = Self::new(module, name, ty, Mutability::Immutable, init);
        global.space = Space::Constant;

        global
    }

    /// Create an imported global declaration (no initializer).
    pub fn import(module: ModuleId, name: StringId, ty: TypeId, mutability: Mutability) -> Self {
        Self {
            name,
            symbol: Symbol::named(module, name),
            ty,
            mutability,
            space: Space::Local,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum GlobalInitializer {
    /// Zero-initialized (all bytes zero).
    Zero,
    /// Scalar constant (bool, int, float).
    Scalar(Constant),
    /// Address of one function inside the program.
    FunctionAddress(FunctionId),
    /// Address of one constant global inside the program.
    GlobalAddress(GlobalId),
    /// Raw bytes (blobs).
    Bytes(Vec<u8>),
    /// Aggregate (array/struct fields).
    Aggregate(Vec<GlobalInitializer>),
    /// One string value, rewritten into its representation's fields by the constant encoder.
    String(StringId),
    /// One bigint value, rewritten into its representation's fields by the constant encoder.
    BigInt(i64),
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

    /// Create from a function address.
    pub fn function_address(function: FunctionId) -> Self {
        Self::FunctionAddress(function)
    }

    /// Create from a constant global address.
    pub fn global_address(global: GlobalId) -> Self {
        Self::GlobalAddress(global)
    }

    /// Create from raw bytes.
    pub fn bytes(data: Vec<u8>) -> Self {
        Self::Bytes(data)
    }

    /// Create from a string literal.
    pub fn string(s: &str) -> Self {
        Self::Bytes(s.as_bytes().to_vec())
    }

    /// Create an aggregate initializer.
    pub fn aggregate(elements: Vec<GlobalInitializer>) -> Self {
        Self::Aggregate(elements)
    }
}
