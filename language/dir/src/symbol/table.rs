use destack_source::{AdaptImage, ModuleId};
use serde::{Deserialize, Serialize};

use crate::{
    Arena, BindingCategory, ExportMode, LocalMergeGroupId, LocalNodeId, LocalScopeId,
    LocalScopeMark, LocalSymbolId, Node, NodeTree, Scope, ScopeKind, StaticKey, Symbol,
    SymbolBinding, SymbolDecorators, SymbolKind, SymbolOrigin, SymbolSpace, SymbolType,
};
use std::fmt::Debug;

/// A SymbolTable is a side table for mapping symbols and scopes. NOT THREAD-SAFE.
#[derive(Debug, Clone, Serialize, Deserialize, AdaptImage)]
pub struct SymbolTable {
    /// The module id of the symbol table.
    pub module_id: ModuleId,

    /// The next symbol id to allocate.
    pub(crate) next_symbol_id: u32,
    /// The next scope id to allocate.
    pub(crate) next_scope_id: u32,
    /// The next merge group id to allocate.
    pub(crate) next_merge_group_id: u32,

    /// The symbols in the table.
    pub(crate) symbols: Arena<Symbol>,
    /// The scopes in the table.
    pub(crate) scopes: Arena<Scope>,
    /// The merge groups in the table.
    pub(crate) merge_groups: Arena<Vec<LocalSymbolId>>,
}

#[allow(clippy::too_many_arguments)]
impl SymbolTable {
    /// Create a new SymbolTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            next_symbol_id: 0,
            next_scope_id: 0,
            next_merge_group_id: 0,
            symbols: Arena::new(),
            scopes: Arena::new(),
            merge_groups: Arena::new(),
        }
    }
    /// Get the symbols.
    #[inline]
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    /// Get a symbol by its raw id.
    #[inline]
    pub fn get_symbol_by_id(&self, symbol_id: u32) -> &Symbol {
        self.symbols.get(symbol_id)
    }

    /// Retype a symbol id and update stored references.
    pub fn retype_symbol_id(
        &mut self,
        symbol_id: LocalSymbolId,
        symbol_type: SymbolType,
    ) -> LocalSymbolId {
        let typed_id = LocalSymbolId::new_typed(symbol_id.id, symbol_type);

        // update the symbol entry type
        let (scope_id, merge_group) = {
            let symbol_entry = self.symbols.get_mut(symbol_id.id);
            symbol_entry.ty = symbol_type;
            (symbol_entry.scope.0, symbol_entry.merge_group)
        };

        // update named symbol ids in the scope
        let scope = self.scopes.get_mut(scope_id.0);
        for (_, scope_symbol_id) in scope.named_symbols.iter_mut() {
            if scope_symbol_id.id == symbol_id.id {
                *scope_symbol_id = typed_id;
            }
        }

        // update anonymous symbol ids in the scope
        for scope_symbol_id in scope.anonymous_symbols.iter_mut() {
            if scope_symbol_id.id == symbol_id.id {
                *scope_symbol_id = typed_id;
            }
        }

        // update scope ownership when it matches
        if let Some(owner_id) = scope.owner_id
            && owner_id.id == symbol_id.id
        {
            scope.owner_id = Some(typed_id);
        }

        // update merge group entries
        if let Some(group_id) = merge_group {
            let group = self.merge_groups.get_mut(group_id.0);
            for merge_symbol_id in group.iter_mut() {
                if merge_symbol_id.id == symbol_id.id {
                    *merge_symbol_id = typed_id;
                }
            }
        }

        typed_id
    }

    /// Iterate active symbol ids.
    pub fn active_symbol_ids(&self) -> impl Iterator<Item = LocalSymbolId> + '_ {
        (0..self.symbol_count()).filter_map(|symbol_id| {
            let symbol = self.get_symbol_by_id(symbol_id);
            if !symbol.is_active {
                return None;
            }
            Some(LocalSymbolId::new_typed(symbol_id, symbol.ty))
        })
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
        binding: SymbolBinding,
        key: Option<StaticKey>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportMode>,
    ) -> (LocalSymbolId, LocalScopeMark) {
        let symbol_id = LocalSymbolId::new_typed(self.next_symbol_id, ty);
        self.next_symbol_id += 1;
        let symbol = Symbol {
            kind,
            ty,
            space,
            binding,
            binding_mutability: None,
            binding_category: BindingCategory::Unclassified,
            origin: SymbolOrigin::Primary,
            key,
            scope,
            module_id: self.module_id,
            export,
            primary_declaration: None,
            secondary_declarations: None,
            merge_group: None,
            target_symbol: None,
            canonical_symbol: None,
            decorators: SymbolDecorators::default(),
            is_active: true,
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

    /// Get an active symbol by its id.
    pub fn get_active_symbol(&self, symbol_id: LocalSymbolId) -> Option<&Symbol> {
        let symbol = self.get_symbol(symbol_id);
        if symbol.is_active {
            return Some(symbol);
        }
        None
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

    /// Iterate active named symbols in a scope.
    pub fn active_named_symbols<'a>(
        &'a self,
        scope: &'a Scope,
    ) -> impl Iterator<Item = (StaticKey, LocalSymbolId)> + 'a {
        scope
            .named_symbols
            .iter()
            .copied()
            .filter(move |(_, symbol_id)| self.get_symbol(*symbol_id).is_active)
    }

    /// Iterate active named symbols in a scope up to a mark.
    pub fn active_named_symbols_up_to<'a>(
        &'a self,
        scope: &'a Scope,
        mark: LocalScopeMark,
    ) -> impl Iterator<Item = (StaticKey, LocalSymbolId)> + 'a {
        let limit = mark.0 as usize;
        scope
            .named_symbols
            .iter()
            .take(limit)
            .copied()
            .filter(move |(_, symbol_id)| self.get_symbol(*symbol_id).is_active)
    }

    /// Iterate active anonymous symbols in a scope.
    pub fn active_anonymous_symbols<'a>(
        &'a self,
        scope: &'a Scope,
    ) -> impl Iterator<Item = LocalSymbolId> + 'a {
        scope
            .anonymous_symbols
            .iter()
            .copied()
            .filter(move |symbol_id| self.get_symbol(*symbol_id).is_active)
    }

    /// Find an active symbol in a scope by key.
    pub fn find_active_symbol(&self, scope: &Scope, key: StaticKey) -> Option<LocalSymbolId> {
        for (candidate_key, symbol_id) in scope.named_symbols.iter().rev() {
            if *candidate_key != key {
                continue;
            }
            let symbol = self.get_symbol(*symbol_id);
            if symbol.is_active {
                return Some(*symbol_id);
            }
        }
        None
    }

    /// Find an active symbol in a scope by key up to a mark.
    pub fn find_active_symbol_up_to(
        &self,
        scope: &Scope,
        key: StaticKey,
        mark: LocalScopeMark,
    ) -> Option<LocalSymbolId> {
        let limit = mark.0 as usize;
        for (candidate_key, symbol_id) in scope.named_symbols.iter().take(limit).rev() {
            if *candidate_key != key {
                continue;
            }
            let symbol = self.get_symbol(*symbol_id);
            if symbol.is_active {
                return Some(*symbol_id);
            }
        }
        None
    }

    /// Create a merge group from a list of symbols.
    pub fn create_merge_group(&mut self, symbols: Vec<LocalSymbolId>) -> LocalMergeGroupId {
        let group_id = LocalMergeGroupId::new(self.next_merge_group_id);
        self.next_merge_group_id += 1;

        // attach symbols to the merge group
        for symbol_id in &symbols {
            self.symbols.get_mut(symbol_id.id).merge_group = Some(group_id);
        }

        self.merge_groups.allocate(symbols);
        group_id
    }

    /// Add a symbol to an existing merge group.
    pub fn add_to_merge_group(&mut self, group_id: LocalMergeGroupId, symbol_id: LocalSymbolId) {
        // attach the symbol to the group
        self.symbols.get_mut(symbol_id.id).merge_group = Some(group_id);

        // record the symbol in the group list
        self.merge_groups.get_mut(group_id.0).push(symbol_id);
    }

    /// Get the symbols in a merge group.
    #[inline]
    pub fn merge_group_symbols(&self, group_id: LocalMergeGroupId) -> &[LocalSymbolId] {
        self.merge_groups.get(group_id.0)
    }
}
