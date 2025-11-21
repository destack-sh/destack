use std::collections::HashMap;

use crate::{SymbolId, SymbolKey};

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

/// Unique identifier for Scopes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScopeId(pub u32);

impl ScopeId {
    /// Wrap an id as a ScopeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A Scope is a container for symbols.
#[derive(Debug, Clone, PartialEq)]
pub struct Scope {
    /// The id of the scope.
    pub id: ScopeId,
    /// The kind of the scope.
    pub kind: ScopeKind,
    /// The parent scope.
    pub parent: Option<ScopeId>,
    /// The owner of the scope.
    pub owner: Option<SymbolId>,
    /// The symbols in the scope.
    pub symbols: HashMap<SymbolKey, SymbolId>,
    /// The children scopes.
    pub children: Vec<ScopeId>,
}

impl Scope {
    /// Whether the scope is the root scope.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Insert a symbol into the scope.
    pub fn insert_symbol(&mut self, key: SymbolKey, symbol_id: SymbolId) {
        self.symbols.insert(key, symbol_id);
    }

    /// Get a symbol from the scope.
    pub fn get_symbol(&self, key: SymbolKey) -> Option<SymbolId> {
        self.symbols.get(&key).cloned()
    }

    /// Insert a child scope into the scope.
    pub fn insert_child_scope(&mut self, scope_id: ScopeId) {
        self.children.push(scope_id);
    }
}
