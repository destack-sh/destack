use dyst_ast::StringId;
use indexmap::IndexMap;

use crate::{
    Arena, LocalNodeId, LocalScopeId, LocalSymbolId, ModuleId, Node, NodeTree, Scope, ScopeKind,
    Symbol, SymbolKey, SymbolSpace,
};
use std::fmt::Debug;

/// A SymbolTable is a side table for mapping symbols and scopes. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// The module id of the symbol table.
    pub module_id: ModuleId,

    /// The next symbol id to allocate.
    pub(crate) next_symbol_id: u32,
    /// The next scope id to allocate.
    pub(crate) next_scope_id: u32,

    /// The symbols in the table.
    pub(crate) symbols: Arena<Symbol>,
    /// The scopes in the table.
    pub(crate) scopes: Arena<Scope>,

    /// The resolved modules by target.
    pub(crate) imported_module_by_target: IndexMap<StringId, ModuleId>,
    /// The exported symbol by key.
    pub(crate) exported_symbol_by_key: IndexMap<(SymbolSpace, SymbolKey), LocalSymbolId>,
}

impl SymbolTable {
    /// Create a new SymbolTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            next_symbol_id: 0,
            next_scope_id: 0,
            symbols: Arena::new(),
            scopes: Arena::new(),
            imported_module_by_target: IndexMap::new(),
            exported_symbol_by_key: IndexMap::new(),
        }
    }

    // nocheckin: bind exports, report duplicate declaration bindings, ..
    /// Create a new local symbol.
    pub fn insert_symbol(
        &mut self,
        space: SymbolSpace,
        key: Option<SymbolKey>,
        scope: LocalScopeId,
    ) -> LocalSymbolId {
        let symbol_id = LocalSymbolId::new(self.next_symbol_id);
        self.next_symbol_id += 1;
        let symbol = Symbol {
            id: symbol_id,
            space,
            key,
            scope_id: scope,
            module_id: self.module_id,
            primary_declaration: None,
            secondary_declarations: Vec::new(),
            target_symbol: None,
        };
        self.symbols.allocate(symbol);
        self.scopes.get_mut(scope.0).insert_symbol(key, symbol_id);
        symbol_id
    }

    /// Create a new local anonymous symbol.
    pub fn insert_anonymous_symbol(&mut self, scope: LocalScopeId) -> LocalSymbolId {
        self.insert_symbol(SymbolSpace::Value, None, scope)
    }

    /// Create a new local scope.
    pub fn insert_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<LocalScopeId>,
        owner: Option<LocalSymbolId>,
    ) -> LocalScopeId {
        let scope_id = LocalScopeId::new(self.next_scope_id);
        self.next_scope_id += 1;
        let scope = Scope {
            id: scope_id,
            kind,
            owner,
            parent_id: parent,
            module_id: self.module_id,
            symbols_by_key: IndexMap::new(),
            anonymous_symbols: Vec::new(),
            children: Vec::new(),
        };
        self.scopes.allocate(scope);
        if let Some(parent) = parent {
            self.scopes.get_mut(parent.0).insert_child_scope(scope_id);
        }
        scope_id
    }

    /// Create a new local symbol with an owned scope. Symbols belongs to outer scope.
    pub fn insert_symbol_with_scope(
        &mut self,
        space: SymbolSpace,
        key: SymbolKey,
        kind: ScopeKind,
        scope: LocalScopeId,
    ) -> (LocalSymbolId, LocalScopeId) {
        let symbol_id = self.insert_symbol(space, Some(key), scope);
        let scope_id = self.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Create a new anonymous symbol and scope. Symbol belongs to outer scope.
    pub fn insert_anonymous_symbol_with_scope(
        &mut self,
        kind: ScopeKind,
        scope: LocalScopeId,
    ) -> (LocalSymbolId, LocalScopeId) {
        let symbol_id = self.insert_symbol(SymbolSpace::Value, None, scope);
        let scope_id = self.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Get a symbol by its id.
    #[inline]
    pub fn get_symbol(&self, symbol_id: LocalSymbolId) -> &Symbol {
        self.symbols.get(symbol_id.0)
    }

    /// Get the symbol mutable by its id.
    #[inline]
    pub fn get_symbol_mut(&mut self, symbol_id: LocalSymbolId) -> &mut Symbol {
        self.symbols.get_mut(symbol_id.0)
    }

    /// Set the primary declaration for a symbol.
    #[inline]
    pub fn set_primary_declaration<T: Node>(
        &mut self,
        symbol_id: LocalSymbolId,
        node_id: LocalNodeId<T>,
    ) {
        self.symbols.get_mut(symbol_id.0).primary_declaration =
            Some(node_id.into_global_any(self.module_id));
    }

    /// Add a secondary declaration for a symbol.
    #[inline]
    pub fn add_secondary_declaration<T: Node>(
        &mut self,
        symbol_id: LocalSymbolId,
        node_id: LocalNodeId<T>,
    ) {
        self.symbols
            .get_mut(symbol_id.0)
            .secondary_declarations
            .push(node_id.into_global_any(self.module_id));
    }

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<'a, T: Node>(&'a self, node_id: LocalNodeId<T>, tree: &NodeTree) -> &'a Scope {
        let scope_id = tree.get_scope(node_id);
        self.scopes.get(scope_id.0)
    }

    /// Get the scope for a symbol id.
    #[inline]
    pub fn get_scope_by_symbol(&self, symbol_id: LocalSymbolId) -> &Scope {
        let symbol = self.get_symbol(symbol_id);
        self.scopes.get(symbol.scope_id.0)
    }

    /// Get a scope by its id.
    #[inline]
    pub fn get_scope_by_id(&self, scope_id: LocalScopeId) -> &Scope {
        self.scopes.get(scope_id.0)
    }

    /// Get the scope mutable by its id.
    #[inline]
    pub fn get_scope_by_id_mut(&mut self, scope_id: LocalScopeId) -> &mut Scope {
        self.scopes.get_mut(scope_id.0)
    }

    /// Set a resolved import for a target.
    #[inline]
    pub fn resolve_import(&mut self, target: StringId, module_id: ModuleId) {
        self.imported_module_by_target.insert(target, module_id);
    }

    /// Get a resolved import for a target.
    #[inline]
    pub fn get_resolved_import(&self, target: StringId) -> Option<ModuleId> {
        self.imported_module_by_target.get(&target).cloned()
    }

    /// Set an exported symbol for a key.
    #[inline]
    pub fn resolve_export(&mut self, key: (SymbolSpace, SymbolKey), symbol_id: LocalSymbolId) {
        self.exported_symbol_by_key.insert(key, symbol_id);
    }

    /// Get an exported symbol for a key.
    #[inline]
    pub fn get_exported_symbol(&self, key: (SymbolSpace, SymbolKey)) -> Option<LocalSymbolId> {
        self.exported_symbol_by_key.get(&key).cloned()
    }
}
