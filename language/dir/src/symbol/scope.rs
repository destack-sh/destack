use std::collections::HashMap;

use crate::{LocalSymbolId, ModuleId, SymbolKey};

/// The kind of a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScopeKind {
    /// Namespace.
    Namespace,
    /// Type.
    Type,
    /// Block.
    Block,
}

/// Unique identifier for local scopes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LocalScopeId(pub u32);

impl LocalScopeId {
    /// Wrap an id as a ScopeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Global scope id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalScopeId {
    /// The module id of the global scope.
    pub module_id: ModuleId,
    /// The local id of the global scope.
    pub local_id: LocalScopeId,
}

impl GlobalScopeId {
    /// Create a new global scope id.
    pub fn new(module_id: ModuleId, local_id: LocalScopeId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }
}

/// A Scope is a container for symbols.
#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    /// The id of the scope.
    pub id: LocalScopeId,
    /// The kind of the scope.
    pub kind: ScopeKind,
    /// The parent scope.
    pub parent: Option<LocalScopeId>,
    /// The owner of the scope.
    pub owner: Option<LocalSymbolId>,
    /// The symbols in the scope.
    pub symbols: HashMap<SymbolKey, LocalSymbolId>,
    /// The children scopes.
    pub children: Vec<LocalScopeId>,
}

impl Scope {
    /// Whether the scope is the root scope.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Insert a symbol into the scope.
    pub fn insert_symbol(&mut self, key: SymbolKey, symbol_id: LocalSymbolId) {
        self.symbols.insert(key, symbol_id);
    }

    /// Get a symbol from the scope.
    pub fn get_symbol(&self, key: SymbolKey) -> Option<LocalSymbolId> {
        self.symbols.get(&key).cloned()
    }

    /// Insert a child scope into the scope.
    pub fn insert_child_scope(&mut self, scope_id: LocalScopeId) {
        self.children.push(scope_id);
    }
}
