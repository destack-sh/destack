use crate::{
    Arena, LocalNodeId, LocalScopeId, LocalSymbolId, ModuleId, Node, NodeTreeImpl, Scope,
    ScopeKind, Symbol, SymbolKey, SymbolSpace,
};
use std::collections::HashMap;
use std::fmt::Debug;

/// A SymbolTable is a side table for a node. NOT THREAD-SAFE.
#[derive(Debug, Clone)]
pub struct SymbolTable {
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

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    /// Create a new SymbolTable.
    pub fn new() -> Self {
        Self {
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
        module_id: Option<ModuleId>,
    ) -> LocalSymbolId {
        let symbol_id = LocalSymbolId::new(self.next_symbol_id);
        self.next_symbol_id += 1;
        let symbol = Symbol {
            id: symbol_id,
            space,
            key,
            scope,
            module_id,
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
        module_id: Option<ModuleId>,
    ) -> LocalScopeId {
        let scope_id = LocalScopeId::new(self.next_scope_id);
        self.next_scope_id += 1;
        let scope = Scope {
            id: scope_id,
            kind,
            owner,
            parent_id: parent,
            module_id,
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
        module_id: Option<ModuleId>,
    ) -> (LocalSymbolId, LocalScopeId) {
        let symbol_id = self.create_symbol(space, key, parent, module_id);
        let scope_id = self.create_scope(kind, Some(parent), Some(symbol_id), module_id);
        (symbol_id, scope_id)
    }

    /// Set the symbol for a node.
    #[inline]
    pub fn set_symbol(&mut self, node_id: u32, symbol_id: LocalSymbolId) {
        self.symbol_by_node_id[node_id as usize] = Some(symbol_id);
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
    pub fn get_scope<T>(&self, node_id: LocalNodeId<T>) -> &Scope
    where
        T: Node,
        Self: NodeTreeImpl<T>,
    {
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
