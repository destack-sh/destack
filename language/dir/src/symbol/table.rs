use crate::{
    Arena, LocalNodeId, LocalScopeId, LocalSymbolId, ModuleId, Node, Scope, ScopeKind, Symbol,
    SymbolKey, SymbolSpace,
};
use std::collections::HashMap;
use std::fmt::Debug;

/// A SymbolTable is a side table for a node. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// The module id of the symbol table.
    pub module_id: ModuleId,

    // meta index
    /// The next symbol id to allocate.
    pub(crate) next_symbol_id: u32,
    /// The next scope id to allocate.
    pub(crate) next_scope_id: u32,

    // meta arenas
    pub(crate) symbols: Arena<Symbol>,
    pub(crate) scopes: Arena<Scope>,

    // meta side data
    /// The symbols by node id. Index is the global node id.
    pub(crate) symbol_by_node_id: Vec<Option<LocalSymbolId>>,
    /// The scopes by node id. Index is the global node id.
    pub(crate) scope_by_node_id: Vec<LocalScopeId>,
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
            symbol_by_node_id: Vec::new(),
            scope_by_node_id: Vec::new(),
        }
    }

    /// Create a new symbol in the tree.
    pub fn create_symbol(
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
            scope,
            module_id: Some(self.module_id),
            owned_scope: None,
            primary_declaration: None,
            secondary_declarations: Vec::new(),
            declared_ty: None,
            inferred_ty: None,
            remote_symbol: None,
        };
        self.symbols.allocate(symbol);
        if let Some(key) = key {
            self.scopes.get_mut(scope.0).insert_symbol(key, symbol_id);
        }
        symbol_id
    }

    /// Create a new scope in the tree.
    pub fn create_scope(
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
            owner: owner.map(|id| id.into_global(self.module_id)),
            parent_id: parent,
            module_id: Some(self.module_id),
            symbols: HashMap::new(),
            children: Vec::new(),
        };
        self.scopes.allocate(scope);
        if let Some(parent) = parent {
            self.scopes.get_mut(parent.0).insert_child_scope(scope_id);
        }
        scope_id
    }

    /// Create a new symbol with a scope.
    pub fn create_symbol_with_scope(
        &mut self,
        space: SymbolSpace,
        key: Option<SymbolKey>,
        kind: ScopeKind,
        parent: LocalScopeId,
    ) -> (LocalSymbolId, LocalScopeId) {
        let symbol_id = self.create_symbol(space, key, parent);
        let scope_id = self.create_scope(kind, Some(parent), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Set the symbol for a node.
    #[inline]
    pub fn set_symbol(&mut self, node_id: u32, symbol_id: LocalSymbolId) {
        self.symbol_by_node_id[node_id as usize] = Some(symbol_id);
    }

    /// Set the owner of a symbol.
    #[inline]
    pub fn set_symbol_owner<T: Node>(&mut self, symbol_id: LocalSymbolId, owner: LocalNodeId<T>) {
        let owner = owner.into_any().into_global(self.module_id);
        self.get_symbol_mut(symbol_id).primary_declaration = Some(owner);
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

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<T: Node>(&self, node_id: LocalNodeId<T>) -> &Scope {
        let scope_id = self.scope_by_node_id[node_id.id as usize];
        self.get_scope_by_id(scope_id)
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

    /// Get the scope for a node by its id.
    #[inline]
    pub fn get_scope_by_node_id(&self, node_id: u32) -> LocalScopeId {
        self.scope_by_node_id[node_id as usize]
    }
}
