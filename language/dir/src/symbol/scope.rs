use std::collections::HashMap;

use crate::{SymbolId, SymbolKey};

/// The kind of a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScopeKind {
    /// Global root scope.
    Root,
    /// Module.
    Module,
    /// Namespace.
    Namespace,
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
}
