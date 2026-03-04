use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{LocalSymbolId, StaticKey};

/// The kind of a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ScopeKind {
    /// Namespace.
    Namespace,
    /// Type.
    Type,
    /// Conditional type infer scope.
    TypeConditional,
    /// Block.
    Block,
}

/// Unique identifier for local scopes.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalScopeId(pub u32);

impl LocalScopeId {
    /// Wrap an id as a ScopeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalScopeId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalScopeId {
        GlobalScopeId {
            module_id,
            local_id: self,
        }
    }
}

impl Display for LocalScopeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl Display for LocalScopeMark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == u32::MAX {
            write!(f, ".END")
        } else {
            write!(f, ".{}", self.0)
        }
    }
}

/// Global scope id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

    /// Turn into a LocalScopeId.
    #[inline]
    pub fn into_local(self) -> LocalScopeId {
        self.local_id
    }
}

impl From<GlobalScopeId> for LocalScopeId {
    fn from(id: GlobalScopeId) -> Self {
        id.local_id
    }
}

/// Mark a position in a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct LocalScopeMark(pub u32);

/// Local scope id and mark pair used for node and symbol insertion.
pub type LocalScope = (LocalScopeId, LocalScopeMark);

impl LocalScopeMark {
    /// Get the full scope view.
    pub fn end() -> Self {
        Self(u32::MAX)
    }
}

/// A Scope is a container for symbols.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    /// The kind of the scope.
    pub kind: ScopeKind,
    /// The parent scope.
    pub parent: Option<(LocalScopeId, LocalScopeMark)>,
    /// The module id of the scope.
    pub module_id: ModuleId,
    /// The owner of the scope.
    pub owner_id: Option<LocalSymbolId>,
    /// The symbols in the scope.
    pub named_symbols: Vec<(StaticKey, LocalSymbolId)>,
    /// THe anonymous symbols in the scope.
    pub anonymous_symbols: Vec<LocalSymbolId>,
    /// The children scopes.
    pub children: Vec<LocalScopeId>,
}

impl Scope {
    /// Whether the scope is the root scope.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Get the current scope mark.
    pub fn mark(&self) -> LocalScopeMark {
        LocalScopeMark(self.named_symbols.len() as u32)
    }

    /// Insert a symbol into the scope.
    pub fn append(&mut self, key: Option<StaticKey>, symbol_id: LocalSymbolId) -> LocalScopeMark {
        let mark = LocalScopeMark(self.named_symbols.len() as u32);
        match key {
            Some(key) => {
                self.named_symbols.push((key, symbol_id));
            }
            None => {
                self.anonymous_symbols.push(symbol_id);
            }
        }
        mark
    }

    /// Get a symbol from the scope by its key.
    pub fn find(&self, key: StaticKey) -> Option<LocalSymbolId> {
        for (candidate_key, id) in self.named_symbols.iter().rev() {
            if *candidate_key == key {
                return Some(*id);
            }
        }
        None
    }

    /// Get a symbol from the scope by its id up to a given mark.
    pub fn find_up_to(&self, key: StaticKey, mark: LocalScopeMark) -> Option<LocalSymbolId> {
        let limit = mark.0 as usize;
        for (candidate_key, id) in self.named_symbols.iter().take(limit).rev() {
            if *candidate_key == key {
                return Some(*id);
            }
        }
        None
    }

    /// Insert a child scope into the scope.
    pub fn append_child(&mut self, scope_id: LocalScopeId) {
        self.children.push(scope_id);
    }
}
