use destack_source::{AdaptImage, ModuleId};
use serde::{Deserialize, Serialize};

use crate::{
    ExportMode, GlobalNodeIdAny, LocalNodeId, LocalScopeId, LocalScopeMark, Mutability, Node,
    NodeType, StaticKey, StringId, SymbolDecorators,
};

/// The space of a symbol.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub enum SymbolSpace {
    /// The type space.
    Type,
    /// The value space.
    Value,
    /// The type-value space (Destack: types ARE values, conflict with both).
    TypeValue,
    /// The label space.
    Label,
}

impl SymbolSpace {
    /// Check if this space conflicts with another space.
    ///
    /// In TypeScript mode, `Type` and `Value` are separate (no conflict).
    /// In Destack mode, `TypeValue` conflicts with both `Type` and `Value`.
    /// `Label` only conflicts with `Label`.
    #[inline]
    pub fn conflicts_with(self, other: Self) -> bool {
        match (self, other) {
            // Label only conflicts with Label
            (Self::Label, Self::Label) => true,
            (Self::Label, _) | (_, Self::Label) => false,
            // TypeValue conflicts with Type, Value, and TypeValue
            (Self::TypeValue, _) | (_, Self::TypeValue) => true,
            // Type and Value are separate in TS mode
            (Self::Type, Self::Type) => true,
            (Self::Value, Self::Value) => true,
            (Self::Type, Self::Value) | (Self::Value, Self::Type) => false,
        }
    }
}

/// The kind of a symbol (scope behavior).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub enum SymbolKind {
    /// Namespace.
    Namespace,
    /// Item (must be unique within its scope).
    Item,
    /// Local (may be shadowed within its scope).
    Local,
}

/// How a symbol was introduced/bound.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    AdaptImage,
)]
pub enum SymbolBinding {
    /// A runtime definition (let, const, var, class, function, etc.)
    #[default]
    Runtime,
    /// An ambient declaration (declare var, declare function, .d.ts)
    Ambient,
}

/// Where a symbol originated in the source.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    AdaptImage,
)]
pub enum SymbolOrigin {
    /// Declaration in module scope.
    #[default]
    Primary,
    /// Declaration inside a `declare global` block.
    GlobalAugmentation,
}

impl SymbolOrigin {
    /// Check if this symbol originates from a global augmentation.
    #[inline]
    pub fn is_global_augmentation(self) -> bool {
        matches!(self, SymbolOrigin::GlobalAugmentation)
    }
}

/// The binding category used for early duplicate-binding validation.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    AdaptImage,
)]
pub enum BindingCategory {
    /// Category has not been assigned yet.
    #[default]
    Unclassified,
    /// Symbol does not participate in duplicate-binding early errors.
    NonBinding,
    /// Function-scoped declaration category (`var`-style).
    FunctionScoped,
    /// Block-scoped declaration category (`let` and `const`-style).
    BlockScoped,
    /// Formal parameter declaration category (including catch parameters).
    Parameter,
}

/// The type of a symbol (declaration type).
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    AdaptImage,
)]
pub enum SymbolType {
    /// Not a type declaration (variables, labels, namespaces, extensions, imports).
    #[default]
    Void,
    /// A class declaration.
    Class,
    /// A struct declaration.
    Struct,
    /// An interface declaration.
    Interface,
    /// An enum declaration.
    Enum,
    /// A function declaration.
    Function,
    /// An extension declaration.
    Extension,
    /// A type alias declaration (transparent, structural equivalence).
    TypeAlias,
    /// A newtype declaration (nominal, distinct type).
    Newtype,
}

impl SymbolType {
    /// Check if this is an interface.
    #[inline]
    pub fn is_interface(self) -> bool {
        self == SymbolType::Interface
    }
}

/// Unique identifier for Symbols.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub struct LocalSymbolId {
    /// The numeric id.
    pub id: u32,
    /// The type of the symbol.
    pub ty: SymbolType,
}

impl LocalSymbolId {
    /// Create a new symbol id with unknown type.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            ty: SymbolType::Void,
        }
    }

    /// Create a new symbol id with a specific type.
    pub fn new_typed(id: u32, ty: SymbolType) -> Self {
        Self { id, ty }
    }

    /// Turn into a GlobalSymbolId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalSymbolId {
        GlobalSymbolId {
            module_id,
            local_id: self,
        }
    }

    /// Set the symbol type, returning a new ID.
    pub fn with_type(self, symbol_type: SymbolType) -> Self {
        Self {
            ty: symbol_type,
            ..self
        }
    }
}

/// Unique identifier for a merge group of symbols.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub struct LocalMergeGroupId(pub u32);

impl LocalMergeGroupId {
    /// Create a new merge group id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Global symbol id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage,
)]
pub struct GlobalSymbolId {
    /// The module id of the global symbol.
    pub module_id: ModuleId,
    /// The local id of the global symbol.
    pub local_id: LocalSymbolId,
}

impl GlobalSymbolId {
    /// Create a new global symbol id.
    pub fn new(module_id: ModuleId, local_id: LocalSymbolId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalSymbolId.
    #[inline]
    pub fn into_local(self) -> LocalSymbolId {
        self.local_id
    }

    /// Get the symbol type.
    #[inline]
    pub fn ty(self) -> SymbolType {
        self.local_id.ty
    }
}

impl From<GlobalSymbolId> for LocalSymbolId {
    fn from(id: GlobalSymbolId) -> Self {
        id.local_id
    }
}

/// A Symbol is a bindable item or local in a scope (which may also declare a scope).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct Symbol {
    /// The kind of the symbol.
    pub kind: SymbolKind,
    /// The type of the symbol.
    pub ty: SymbolType,
    /// The "space" of the symbol.
    pub space: SymbolSpace,
    /// How this symbol was introduced/bound.
    pub binding: SymbolBinding,
    /// The mutability for value bindings when known.
    pub binding_mutability: Option<Mutability>,
    /// The binding category used for early duplicate-binding checks.
    pub binding_category: BindingCategory,
    /// Where this symbol was introduced.
    pub origin: SymbolOrigin,
    /// The key of the symbol.
    pub key: Option<StaticKey>,
    /// The scope that introduces the symbol.
    pub scope: (LocalScopeId, LocalScopeMark),
    /// The module id of the scope.
    pub module_id: ModuleId,
    /// The export mode of the symbol.
    pub export: Option<ExportMode>,
    /// The main declaration node of the symbol.
    pub primary_declaration: Option<GlobalNodeIdAny>,
    /// Secondary declaration nodes of the symbol (for merging with other symbols within this module).
    pub secondary_declarations: Option<Box<Vec<GlobalNodeIdAny>>>,
    /// Merge group for cross-space declarations.
    pub merge_group: Option<LocalMergeGroupId>,
    /// Forward to the *next* remote symbol (like for imports, pattern bindings, etc.).
    pub target_symbol: Option<GlobalSymbolId>,
    /// Final remote symbol in the chain (end of target-symbol chain).
    pub canonical_symbol: Option<GlobalSymbolId>,
    /// Decorators applied to the symbol.
    pub decorators: SymbolDecorators,
    /// Whether the symbol is active for the current profile.
    pub is_active: bool,
}

impl Symbol {
    /// Check whether the symbol is active.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Get the name of the symbol.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self.key {
            Some(StaticKey::Name(name)) => Some(name),
            _ => None,
        }
    }

    /// Check whether this symbol is a static parameter.
    pub fn is_static_parameter(&self) -> bool {
        self.space == SymbolSpace::Type
            && self
                .primary_declaration
                .is_some_and(|declaration| declaration.local_id.ty == NodeType::Parameter)
    }

    /// Declare this symbol from a declaration node.
    pub fn declare_primary<T: Node>(&mut self, node_id: LocalNodeId<T>) {
        self.primary_declaration = Some(node_id.into_global_any(self.module_id));
    }

    /// Declare a secondary declaration for this symbol.
    pub fn declare_secondary<T: Node>(&mut self, node_id: LocalNodeId<T>) {
        if self.secondary_declarations.is_none() {
            self.secondary_declarations = Some(Box::new(Vec::new()));
        }
        self.secondary_declarations
            .as_mut()
            .unwrap()
            .push(node_id.into_global_any(self.module_id));
    }

    /// Resolve a target symbol for this symbol.
    pub fn resolve_to(&mut self, target_symbol: GlobalSymbolId) {
        self.target_symbol = Some(target_symbol);
    }
}
