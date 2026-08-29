use destack_core::Arena;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::SymbolId;

/// One JavaScript scope identifier.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct ScopeId(pub u32);

impl ScopeId {
    /// The root JavaScript scope.
    pub const ROOT: Self = Self(0);
}

/// One lexical JavaScript scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Scope {
    /// The parent scope when one exists.
    pub parent: Option<ScopeId>,
    /// The symbols declared directly in this scope.
    pub symbols: Vec<SymbolId>,
}

/// The lexical scopes of one JavaScript module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ScopeTable {
    /// The scopes in allocation order.
    scopes: Arena<Scope>,
}

impl ScopeTable {
    /// Create a scope table containing its root scope.
    pub fn new() -> Self {
        let mut scopes = Arena::new();
        scopes.allocate(Scope {
            parent: None,
            symbols: Vec::new(),
        });

        Self { scopes }
    }

    /// Allocate one scope.
    pub(crate) fn insert(&mut self, parent: ScopeId) -> ScopeId {
        let id = ScopeId(self.scopes.len() as u32);
        let scope = Scope {
            parent: Some(parent),
            symbols: Vec::new(),
        };
        self.scopes.allocate(scope);

        id
    }

    /// Return one scope.
    #[inline]
    pub fn get(&self, id: ScopeId) -> &Scope {
        self.scopes.get(id.0)
    }

    /// Iterate over scopes in allocation order.
    pub fn iter(&self) -> impl Iterator<Item = (ScopeId, &Scope)> {
        self.scopes
            .iter()
            .enumerate()
            .map(|(index, scope)| (ScopeId(index as u32), scope))
    }

    /// Return one mutable scope.
    #[inline]
    pub(crate) fn get_mut(&mut self, id: ScopeId) -> &mut Scope {
        self.scopes.get_mut(id.0)
    }
}

impl Default for ScopeTable {
    fn default() -> Self {
        Self::new()
    }
}
