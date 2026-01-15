use serde::{Deserialize, Serialize};

use destack_base::StringId;

/// Identifier for a memory alias domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AliasDomainId(u32);

impl AliasDomainId {
    /// Create a domain id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Identifier for a memory alias scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AliasScopeId(u32);

impl AliasScopeId {
    /// Create a scope id from a raw index.
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get the raw index for this id.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Alias analysis domain for grouping alias scopes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAliasDomain {
    /// Optional name for diagnostics or debugging.
    pub name: Option<StringId>,
}

/// Alias scope for noalias or scoped aliasing metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryAliasScope {
    /// The domain this scope belongs to.
    pub domain: AliasDomainId,
    /// Optional name for diagnostics or debugging.
    pub name: Option<StringId>,
}

/// Table of alias scopes and domains.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AliasScopeTable {
    /// Registered alias domains.
    pub domains: Vec<MemoryAliasDomain>,
    /// Registered alias scopes.
    pub scopes: Vec<MemoryAliasScope>,
}

impl AliasScopeTable {
    /// Create a new empty alias scope table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new alias domain.
    pub fn create_domain(&mut self, name: Option<StringId>) -> AliasDomainId {
        let id = AliasDomainId::new(self.domains.len() as u32);
        self.domains.push(MemoryAliasDomain { name });
        id
    }

    /// Create a new alias scope within a domain.
    pub fn create_scope(&mut self, domain: AliasDomainId, name: Option<StringId>) -> AliasScopeId {
        let id = AliasScopeId::new(self.scopes.len() as u32);
        self.scopes.push(MemoryAliasScope { domain, name });
        id
    }

    /// Return the alias domain for an id.
    pub fn domain(&self, id: AliasDomainId) -> &MemoryAliasDomain {
        &self.domains[id.index()]
    }

    /// Return the alias scope for an id.
    pub fn scope(&self, id: AliasScopeId) -> &MemoryAliasScope {
        &self.scopes[id.index()]
    }
}
