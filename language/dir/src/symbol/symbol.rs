use crate::{
    DependencyMode, GlobalNodeIdAny, LocalNodeId, LocalNodeIdAny, LocalScopeId, LocalScopeMark,
    ModuleId, Node, Program, StringId,
};

/// Key for a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Eq)]
pub enum SymbolKey {
    /// Regular name key (like `x` or `"weird identifier"`).
    Name(StringId),
    /// Unique symbol expression (like `const x = Symbol("x");`).
    UniqueSymbol(LocalNodeIdAny),
    /// Global symbol key (like `Symbol.iterator`).
    GlobalSymbol(StringId),
}

impl From<StringId> for SymbolKey {
    fn from(name: StringId) -> Self {
        SymbolKey::Name(name)
    }
}

impl SymbolKey {
    /// Get the name of the symbol key.
    pub fn name(&self) -> Option<StringId> {
        match self {
            SymbolKey::Name(name) => Some(*name),
            SymbolKey::UniqueSymbol(..) => None,
            SymbolKey::GlobalSymbol(name) => Some(*name),
        }
    }

    /// Get the debug string in a given program.
    pub fn debug_string(&self, program: &Program) -> String {
        match self {
            SymbolKey::Name(name) => {
                format!("'{}'", program.strings.get(*name).as_str()).to_string()
            }
            SymbolKey::UniqueSymbol(..) => "<unique symbol>".to_string(),
            SymbolKey::GlobalSymbol(name) => {
                format!("'{}'", program.strings.get(*name).as_str()).to_string()
            }
        }
    }
}

/// The space of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolSpace {
    /// The type space.
    Type,
    /// The value space.
    Value,
}

/// The kind of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    /// Namespace.
    Namespace,
    /// Item (must be unique within its scope).
    Item,
    /// Local (may be shadowed within its scope).
    Local,
}

/// Unique identifier for Symbols.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalSymbolId(pub u32);

impl LocalSymbolId {
    /// Wrap an id as a SymbolId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalSymbolId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalSymbolId {
        GlobalSymbolId {
            module_id,
            local_id: self,
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
    /// The "space" of the symbol.
    pub space: SymbolSpace,
    /// The key of the symbol.
    pub key: Option<SymbolKey>,
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
    /// Forward to another remote symbol (like for imports, pattern bindings, etc.).
    pub target_symbol: Option<GlobalSymbolId>,
    /// Final remote symbol in the chain (end of target-symbol chain).
    pub final_symbol: Option<GlobalSymbolId>,
}

impl Symbol {
    /// Get the name of the symbol.
    #[inline]
    pub fn name(&self) -> Option<StringId> {
        match self.key {
            Some(SymbolKey::Name(name)) => Some(name),
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
