use destack_core::{StringId, StringPool};
use destack_serde::Reflect;
use destack_source::{ModuleId, ProvenanceId, ProvenanceJournal, ProvenanceTable};
use serde::{Deserialize, Serialize};

use crate::{
    LocalNodeId, ScopeId, ScopeTable, Statement, SymbolId, SymbolNamespace, SymbolTable, Tree,
};

/// One lowered JavaScript module tree.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Module {
    /// The module identity.
    pub id: ModuleId,
    /// The JavaScript tree.
    pub tree: Tree,
    /// The root statements.
    pub roots: Vec<LocalNodeId<Statement>>,
    /// The strings referenced by the module.
    pub strings: StringPool,
    /// The symbols declared by the module.
    pub symbols: SymbolTable,
    /// The lexical scopes in the module.
    pub scopes: ScopeTable,
    /// The provenance referenced by the tree.
    pub provenance: ProvenanceTable,
}

impl Module {
    /// Create an empty JavaScript module.
    pub fn new(id: ModuleId, provenance: ProvenanceTable) -> Self {
        Self {
            id,
            tree: Tree::new(),
            roots: Vec::new(),
            strings: StringPool::new(),
            symbols: SymbolTable::new(),
            scopes: ScopeTable::new(),
            provenance,
        }
    }

    /// Rewrite this module under one recorded transform.
    pub fn rewrite<T>(
        &mut self,
        transform: &str,
        rewrite: impl FnOnce(&mut Self, &mut ProvenanceJournal<'_>) -> T,
    ) -> T {
        let mut provenance = self.provenance.extend();
        let mut journal = provenance.record(transform);
        let result = rewrite(self, &mut journal);
        self.provenance = provenance.finish();

        result
    }

    /// Allocate one lexical scope.
    pub fn insert_scope(&mut self, parent: ScopeId) -> ScopeId {
        self.scopes.insert(parent)
    }

    /// Allocate one symbol and declare it in its lexical scope.
    pub fn insert_symbol(
        &mut self,
        name: StringId,
        namespace: SymbolNamespace,
        scope: ScopeId,
        provenance: ProvenanceId,
    ) -> SymbolId {
        let symbol = self.symbols.insert(name, namespace, scope, provenance);
        self.scopes.get_mut(scope).symbols.push(symbol);

        symbol
    }
}
