use std::fmt::Debug;

use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, ExportKind, GlobalNodeIdAny, LocalNodeId, LocalNodeIdAny, LocalScope, LocalScopeId,
    LocalScopeMark, LocalSymbolId, Node, Scope, ScopeKind, StaticKey, Symbol, SymbolBinding,
    SymbolForm, SymbolOrigin, SymbolRole, SymbolSpace,
};

/// Lexical scopes and symbols for one DIR module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingTable {
    /// The module id of the binding table.
    pub module_id: ModuleId,
    /// The first symbol id owned by this table segment.
    pub(crate) first_symbol_id: u32,
    /// The first scope id owned by this table segment.
    pub(crate) first_scope_id: u32,

    /// The symbols in the table.
    pub(crate) symbols: Arena<Symbol>,
    /// The scopes in the table.
    pub(crate) scopes: Arena<Scope>,
    /// Symbols keyed by their declaration node.
    pub(crate) symbol_by_declaration: IndexMap<GlobalNodeIdAny, LocalSymbolId>,
    /// Scopes keyed by their owner or member node.
    pub(crate) scope_by_node: IndexMap<GlobalNodeIdAny, LocalScope>,
    /// Replacements for visible symbols copied into this segment.
    pub(crate) replaced_symbol_by_id: IndexMap<LocalSymbolId, Symbol>,
    /// Replacements for visible scopes copied into this segment.
    pub(crate) replaced_scope_by_id: IndexMap<LocalScopeId, Scope>,
}

#[allow(clippy::too_many_arguments)]
impl BindingTable {
    /// Create a new BindingTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            first_symbol_id: 0,
            first_scope_id: 0,
            symbols: Arena::new(),
            scopes: Arena::new(),
            symbol_by_declaration: IndexMap::new(),
            scope_by_node: IndexMap::new(),
            replaced_symbol_by_id: IndexMap::new(),
            replaced_scope_by_id: IndexMap::new(),
        }
    }

    /// Create a new empty segment after an existing binding table segment.
    pub fn from_base(base: &Self) -> Self {
        Self {
            module_id: base.module_id,
            first_symbol_id: base.symbol_count(),
            first_scope_id: base.scope_count(),
            symbols: Arena::new(),
            scopes: Arena::new(),
            symbol_by_declaration: IndexMap::new(),
            scope_by_node: IndexMap::new(),
            replaced_symbol_by_id: IndexMap::new(),
            replaced_scope_by_id: IndexMap::new(),
        }
    }

    /// Get the symbols.
    #[inline]
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbol_ids()
            .map(|symbol_id| self.get_symbol(symbol_id))
    }

    /// Get a symbol by its raw id.
    #[inline]
    pub fn get_symbol_by_id(&self, symbol_id: u32) -> &Symbol {
        self.get_symbol(LocalSymbolId::new(symbol_id))
    }

    /// Iterate symbol ids.
    pub fn symbol_ids(&self) -> impl Iterator<Item = LocalSymbolId> + '_ {
        (self.first_symbol_id..self.symbol_count()).map(LocalSymbolId::new)
    }

    /// Get the number of symbols.
    #[inline]
    pub fn symbol_count(&self) -> u32 {
        self.first_symbol_id + self.symbols.len() as u32
    }

    /// Get the scopes.
    #[inline]
    pub fn scopes(&self) -> impl Iterator<Item = &Scope> {
        self.scope_ids()
            .map(|scope_id| self.get_scope_by_id(scope_id))
    }

    /// Iterate scope ids.
    pub fn scope_ids(&self) -> impl Iterator<Item = LocalScopeId> + '_ {
        (self.first_scope_id..self.scope_count()).map(LocalScopeId::new)
    }

    /// Get the number of scopes.
    #[inline]
    pub fn scope_count(&self) -> u32 {
        self.first_scope_id + self.scopes.len() as u32
    }

    /// Insert a new symbol.
    pub fn insert_symbol(
        &mut self,
        role: SymbolRole,
        form: SymbolForm,
        space: SymbolSpace,
        binding: SymbolBinding,
        key: Option<StaticKey>,
        scope: LocalScope,
        export: Option<ExportKind>,
    ) -> (LocalSymbolId, LocalScopeMark) {
        let symbol_id = LocalSymbolId::new(self.symbol_count());
        let symbol = Symbol {
            role,
            form,
            space,
            binding,
            binding_mutability: None,
            origin: SymbolOrigin::Module,
            key,
            scope,
            export_kind: export,
            declaration: None,
        };
        self.symbols.allocate(symbol);
        let mark = self.get_scope_by_id_mut(scope.id).append(key, symbol_id);
        (symbol_id, mark)
    }

    /// Attach a declaration node to a symbol.
    pub fn declare_symbol<T: Node>(&mut self, symbol_id: LocalSymbolId, node_id: LocalNodeId<T>) {
        let module_id = self.module_id;
        let declaration = node_id.into_global_any(module_id);

        self.get_symbol_mut(symbol_id).declaration = Some(declaration);
        self.symbol_by_declaration.insert(declaration, symbol_id);
    }

    /// Find the symbol declared by one node.
    #[inline]
    pub fn symbol_for_declaration(&self, declaration: GlobalNodeIdAny) -> Option<LocalSymbolId> {
        self.symbol_by_declaration.get(&declaration).copied()
    }

    /// Attach a lexical scope to one typed node.
    pub fn bind_scope<T: Node>(&mut self, node_id: LocalNodeId<T>, scope: LocalScope) {
        self.bind_scope_any(node_id.into_any(), scope);
    }

    /// Attach a lexical scope to one erased node.
    pub fn bind_scope_any(&mut self, node_id: LocalNodeIdAny, scope: LocalScope) {
        let node_id = node_id.into_global(self.module_id);

        self.scope_by_node.insert(node_id, scope);
    }

    /// Return the lexical scope attached to one global node.
    #[inline]
    pub fn scope_for_node(&self, node_id: GlobalNodeIdAny) -> Option<LocalScope> {
        self.scope_by_node.get(&node_id).copied()
    }

    /// Insert a new scope.
    pub fn insert_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<LocalScope>,
        owner: Option<LocalSymbolId>,
    ) -> LocalScopeId {
        let scope_id = LocalScopeId::new(self.scope_count());
        let scope = Scope {
            kind,
            owner,
            parent,
            bindings: Vec::new(),
            children: Vec::new(),
        };
        self.scopes.allocate(scope);
        if let Some(parent) = parent {
            self.get_scope_by_id_mut(parent.id).append_child(scope_id);
        }
        scope_id
    }

    /// Get a symbol by its id.
    #[inline]
    pub fn get_symbol(&self, symbol_id: LocalSymbolId) -> &Symbol {
        if let Some(symbol) = self.replaced_symbol_by_id.get(&symbol_id) {
            return symbol;
        }

        self.get_local_symbol(symbol_id)
            .unwrap_or_else(|| panic!("DIR symbol {symbol_id:?} is not allocated in this segment"))
    }

    /// Get the symbol mutable by its id.
    #[inline]
    pub fn get_symbol_mut(&mut self, symbol_id: LocalSymbolId) -> &mut Symbol {
        if self.contains_symbol_id(symbol_id) {
            let slot = self.symbol_slot(symbol_id);
            return self.symbols.get_mut(slot);
        }

        self.replaced_symbol_by_id
            .get_mut(&symbol_id)
            .unwrap_or_else(|| panic!("DIR symbol {symbol_id:?} is not mutable in this segment"))
    }

    /// Get the scope view for a scope id.
    #[inline]
    pub fn get_scope_mark(&self, scope_id: LocalScopeId) -> LocalScopeMark {
        self.get_scope_by_id(scope_id).mark()
    }

    /// Get the scope for a symbol id.
    #[inline]
    pub fn get_scope_by_symbol(&self, symbol_id: LocalSymbolId) -> &Scope {
        let symbol = self.get_symbol(symbol_id);
        self.get_scope_by_id(symbol.scope.id)
    }

    /// Get a scope by its id.
    #[inline]
    pub fn get_scope_by_id(&self, scope_id: LocalScopeId) -> &Scope {
        if let Some(scope) = self.replaced_scope_by_id.get(&scope_id) {
            return scope;
        }

        self.get_local_scope(scope_id)
            .unwrap_or_else(|| panic!("DIR scope {scope_id:?} is not allocated in this segment"))
    }

    /// Get the scope mutable by its id.
    #[inline]
    pub fn get_scope_by_id_mut(&mut self, scope_id: LocalScopeId) -> &mut Scope {
        if self.contains_scope_id(scope_id) {
            let slot = self.scope_slot(scope_id);
            return self.scopes.get_mut(slot);
        }

        self.replaced_scope_by_id
            .get_mut(&scope_id)
            .unwrap_or_else(|| panic!("DIR scope {scope_id:?} is not mutable in this segment"))
    }

    /// Iterate named symbols in a scope.
    pub fn named_symbols<'a>(
        &'a self,
        scope: &'a Scope,
    ) -> impl Iterator<Item = (StaticKey, LocalSymbolId)> + 'a {
        scope
            .bindings
            .iter()
            .filter_map(|binding| binding.key.map(|key| (key, binding.symbol)))
    }

    /// Iterate named symbols in a scope up to a mark.
    pub fn named_symbols_up_to<'a>(
        &'a self,
        scope: &'a Scope,
        mark: LocalScopeMark,
    ) -> impl Iterator<Item = (StaticKey, LocalSymbolId)> + 'a {
        let limit = mark.0 as usize;
        scope
            .bindings
            .iter()
            .take(limit)
            .filter_map(|binding| binding.key.map(|key| (key, binding.symbol)))
    }

    /// Iterate anonymous symbols in a scope.
    pub fn anonymous_symbols<'a>(
        &'a self,
        scope: &'a Scope,
    ) -> impl Iterator<Item = LocalSymbolId> + 'a {
        scope
            .bindings
            .iter()
            .filter_map(|binding| binding.key.is_none().then_some(binding.symbol))
    }

    /// Find a symbol in a scope by key.
    pub fn find_symbol(&self, scope: &Scope, key: StaticKey) -> Option<LocalSymbolId> {
        for binding in scope.bindings.iter().rev() {
            if binding.key != Some(key) {
                continue;
            }
            return Some(binding.symbol);
        }
        None
    }

    /// Find a symbol in a scope by key up to a mark.
    pub fn find_symbol_up_to(
        &self,
        scope: &Scope,
        key: StaticKey,
        mark: LocalScopeMark,
    ) -> Option<LocalSymbolId> {
        let limit = mark.0 as usize;
        for binding in scope.bindings.iter().take(limit).rev() {
            if binding.key != Some(key) {
                continue;
            }
            return Some(binding.symbol);
        }
        None
    }

    /// Get a symbol owned or replaced by this table segment.
    pub(crate) fn get_local_symbol(&self, symbol_id: LocalSymbolId) -> Option<&Symbol> {
        if let Some(symbol) = self.replaced_symbol_by_id.get(&symbol_id) {
            return Some(symbol);
        }

        self.contains_symbol_id(symbol_id)
            .then(|| self.symbols.get(self.symbol_slot(symbol_id)))
    }

    /// Get a scope owned or replaced by this table segment.
    pub(crate) fn get_local_scope(&self, scope_id: LocalScopeId) -> Option<&Scope> {
        if let Some(scope) = self.replaced_scope_by_id.get(&scope_id) {
            return Some(scope);
        }

        self.contains_scope_id(scope_id)
            .then(|| self.scopes.get(self.scope_slot(scope_id)))
    }

    /// Replace one visible symbol in this table segment.
    pub fn replace_symbol(&mut self, symbol_id: LocalSymbolId, symbol: Symbol) {
        if let Some(declaration) = symbol.declaration {
            self.symbol_by_declaration.insert(declaration, symbol_id);
        }

        self.replaced_symbol_by_id.insert(symbol_id, symbol);
    }

    /// Replace one visible scope in this table segment.
    pub fn replace_scope(&mut self, scope_id: LocalScopeId, scope: Scope) {
        self.replaced_scope_by_id.insert(scope_id, scope);
    }

    /// Return whether this segment contains the given symbol id.
    fn contains_symbol_id(&self, symbol_id: LocalSymbolId) -> bool {
        symbol_id.id >= self.first_symbol_id && symbol_id.id < self.symbol_count()
    }

    /// Return whether this segment contains the given scope id.
    fn contains_scope_id(&self, scope_id: LocalScopeId) -> bool {
        scope_id.0 >= self.first_scope_id && scope_id.0 < self.scope_count()
    }

    /// Return the local arena slot for one symbol id.
    fn symbol_slot(&self, symbol_id: LocalSymbolId) -> u32 {
        symbol_id.id - self.first_symbol_id
    }

    /// Return the local arena slot for one scope id.
    fn scope_slot(&self, scope_id: LocalScopeId) -> u32 {
        scope_id.0 - self.first_scope_id
    }
}
