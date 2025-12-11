use destack_source::ModuleId;

use crate::{
    DependencyMode, GlobalNodeIdAny, LocalNodeId, LocalScopeId, LocalScopeMark, Node, StaticKey,
    StringId,
};

/// The space of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolSpace {
    /// The type space.
    Type,
    /// The value space.
    Value,
}

/// The kind of a symbol (scope behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    /// Namespace.
    Namespace,
    /// Item (must be unique within its scope).
    Item,
    /// Local (may be shadowed within its scope).
    Local,
}

/// The type of a symbol (declaration type).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// Global symbol id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    /// The kind of the symbol.
    pub kind: SymbolKind,
    /// The type of the symbol.
    pub ty: SymbolType,
    /// The "space" of the symbol.
    pub space: SymbolSpace,
    /// The key of the symbol.
    pub key: Option<StaticKey>,
    /// The scope that introduces the symbol.
    pub scope: (LocalScopeId, LocalScopeMark),
    /// The module id of the scope.
    pub module_id: ModuleId,
    /// The export mode of the symbol.
    pub export: Option<DependencyMode>,
    /// The main declaration node of the symbol.
    pub primary_declaration: Option<GlobalNodeIdAny>,
    /// Secondary declaration nodes of the symbol.
    pub secondary_declarations: Option<Box<Vec<GlobalNodeIdAny>>>,
    /// Forward to the *next* remote symbol (like for imports, pattern bindings, etc.).
    pub target_symbol: Option<GlobalSymbolId>,
    /// Final remote symbol in the chain (end of target-symbol chain).
    pub canonical_symbol: Option<GlobalSymbolId>,
}

impl Symbol {
    /// Get the name of the symbol.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self.key {
            Some(StaticKey::Name(name)) => Some(name),
            _ => None,
        }
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
