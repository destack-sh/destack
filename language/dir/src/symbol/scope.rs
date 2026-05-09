use std::fmt::Display;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{LocalSymbolId, StaticKey};

/// The kind of a scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ScopeKind {
    /// Module root.
    Module,
    /// Namespace declaration or object declaration surface.
    Namespace,
    /// Function body and parameter surface.
    Function,
    /// Type expression or declaration surface.
    Type,
    /// Conditional type infer scope.
    TypeConditional,
    /// Block expression or statement surface.
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

/// One binding entry in lexical order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeBinding {
    /// The binding key.
    pub key: Option<StaticKey>,
    /// The bound symbol.
    pub symbol: LocalSymbolId,
}

/// A lexical container for symbols.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    /// The kind of the scope.
    pub kind: ScopeKind,
    /// The parent scope.
    pub parent: Option<(LocalScopeId, LocalScopeMark)>,
    /// The owner of the scope.
    pub owner: Option<LocalSymbolId>,

    /// The bindings in lexical order.
    pub bindings: Vec<ScopeBinding>,

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
        LocalScopeMark(self.bindings.len() as u32)
    }

    /// Insert a symbol into the scope.
    pub fn append(&mut self, key: Option<StaticKey>, symbol_id: LocalSymbolId) -> LocalScopeMark {
        let mark = LocalScopeMark(self.bindings.len() as u32);
        self.bindings.push(ScopeBinding {
            key,
            symbol: symbol_id,
        });
        mark
    }

    /// Insert a child scope into the scope.
    pub fn append_child(&mut self, scope_id: LocalScopeId) {
        self.children.push(scope_id);
    }
}
