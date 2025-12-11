use destack_source::ModuleId;

use crate::{
    Arena, DependencyMode, LocalNodeId, LocalScopeId, LocalScopeMark, LocalSymbolId, Node,
    NodeTree, Scope, ScopeKind, StaticKey, Symbol, SymbolKind, SymbolSpace, SymbolType,
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
        }
    }

    /// Get the symbols.
    #[inline]
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    /// Get the number of symbols.
    #[inline]
    pub fn symbol_count(&self) -> u32 {
        self.next_symbol_id
    }

    /// Get the scopes.
    #[inline]
    pub fn scopes(&self) -> impl Iterator<Item = &Scope> {
        self.scopes.iter()
    }

    /// Insert a new symbol.
    pub fn insert_symbol(
        &mut self,
        kind: SymbolKind,
        ty: SymbolType,
        space: SymbolSpace,
        key: Option<StaticKey>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<DependencyMode>,
    ) -> (LocalSymbolId, LocalScopeMark) {
        let symbol_id = LocalSymbolId::new_typed(self.next_symbol_id, ty);
        self.next_symbol_id += 1;
        let symbol = Symbol {
            kind,
            ty,
            space,
            key,
            scope,
            module_id: self.module_id,
            export,
            primary_declaration: None,
            secondary_declarations: None,
            target_symbol: None,
            final_symbol: None,
        };
        self.symbols.allocate(symbol);
        let mark = self.scopes.get_mut(scope.0.0).append(key, symbol_id);
        (symbol_id, mark)
    }

    /// Insert a new scope.
    pub fn insert_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<(LocalScopeId, LocalScopeMark)>,
        owner: Option<LocalSymbolId>,
    ) -> LocalScopeId {
        let scope_id = LocalScopeId::new(self.next_scope_id);
        self.next_scope_id += 1;
        let scope = Scope {
            kind,
            owner_id: owner,
            parent,
            module_id: self.module_id,
            named_symbols: Vec::new(),
            anonymous_symbols: Vec::new(),
            children: Vec::new(),
        };
        self.scopes.allocate(scope);
        if let Some(parent) = parent {
            self.scopes.get_mut(parent.0.0).append_child(scope_id);
        }
        scope_id
    }

    /// Get a symbol by its id.
    #[inline]
    pub fn get_symbol(&self, symbol_id: LocalSymbolId) -> &Symbol {
        self.symbols.get(symbol_id.id)
    }

    /// Get the symbol mutable by its id.
    #[inline]
    pub fn get_symbol_mut(&mut self, symbol_id: LocalSymbolId) -> &mut Symbol {
        self.symbols.get_mut(symbol_id.id)
    }

    /// Get the scope view for a scope id.
    #[inline]
    pub fn get_scope_mark(&self, scope_id: LocalScopeId) -> LocalScopeMark {
        self.scopes.get(scope_id.0).mark()
    }

    /// Get the scope for a node id.
    #[inline]
    pub fn get_scope<'a, T: Node>(
        &'a self,
        node_id: LocalNodeId<T>,
        tree: &NodeTree,
    ) -> (LocalScopeId, &'a Scope, LocalScopeMark) {
        let (scope_id, mark) = tree.get_scope(node_id);
        let scope = self.scopes.get(scope_id.0);
        (scope_id, scope, mark)
    }

    /// Get the scope for a symbol id.
    #[inline]
    pub fn get_scope_by_symbol(&self, symbol_id: LocalSymbolId) -> &Scope {
        let symbol = self.get_symbol(symbol_id);
        self.scopes.get(symbol.scope.0.0)
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
}
