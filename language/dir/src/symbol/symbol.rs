use crate::{GlobalNodeIdAny, LocalNodeId, LocalNodeIdAny, LocalScopeId, ModuleId, StringId, Type};

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

/// The space of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolSpace {
    /// The type space.
    Type,
    /// The value space.
    Value,
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
    pub fn into_local(self) -> LocalSymbolId {
        self.local_id
    }
}

impl From<GlobalSymbolId> for LocalSymbolId {
    fn from(id: GlobalSymbolId) -> Self {
        id.local_id
    }
}

/// A Symbol is a bindable item in a scope (which may also declare a scope).
/// Some symbols are virtual / anonymous (like block targets).
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    /// The id of the symbol.
    pub id: LocalSymbolId,
    /// The "space" of the symbol.
    pub space: SymbolSpace,
    /// The key of the symbol.
    pub key: Option<SymbolKey>,
    /// The scope that introduces the symbol.
    pub scope: LocalScopeId,
    /// The module id of the scope.
    pub module_id: ModuleId,
    /// The main declaration node of the symbol.
    pub primary_declaration: Option<GlobalNodeIdAny>,
    /// Secondary declaration nodes of the symbol.
    pub secondary_declarations: Vec<GlobalNodeIdAny>,
    /// The declared type of the symbol.
    pub declared_ty: Option<LocalNodeId<Type>>,
    /// The inferred type of the symbol.
    pub inferred_ty: Option<LocalNodeId<Type>>,
    /// Forward to another remote symbol (like for imports, pattern bindings, etc.).
    pub remote_symbol: Option<GlobalSymbolId>,
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
}
