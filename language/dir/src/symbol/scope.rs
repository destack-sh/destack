use std::collections::HashMap;

use dyst_ast::StringId;

use crate::SymbolId;

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
    /// The parent scope.
    pub parent: Option<ScopeId>,
    /// The symbols in the scope.
    pub symbols: HashMap<StringId, SymbolId>,
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
