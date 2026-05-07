use std::fmt::Debug;

use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Arena, BindingCategory, DeclarationForm, ExportKind, LocalNodeId, LocalScopeId, LocalScopeMark,
    LocalSymbolId, Node, Scope, ScopeKind, StaticKey, Symbol, SymbolAttributes, SymbolBinding,
    SymbolKind, SymbolOrigin, SymbolSpace, Tree,
};

/// A symbol table maps local symbols and scopes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolTable {
    /// The module id of the symbol table.
    pub module_id: ModuleId,

    /// The symbols in the table.
    pub(crate) symbols: Arena<Symbol>,
    /// The scopes in the table.
    pub(crate) scopes: Arena<Scope>,
}

#[allow(clippy::too_many_arguments)]
impl SymbolTable {
    /// Create a new SymbolTable.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            symbols: Arena::new(),
            scopes: Arena::new(),
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

    /// Iterate active symbol ids.
    pub fn active_symbol_ids(&self) -> impl Iterator<Item = LocalSymbolId> + '_ {
        (0..self.symbol_count()).filter_map(|symbol_id| {
            let symbol = self.get_symbol_by_id(symbol_id);
            if !symbol.is_active {
                return None;
            }
            Some(LocalSymbolId::new(symbol_id))
        })
    }

    /// Get the number of symbols.
    #[inline]
    pub fn symbol_count(&self) -> u32 {
        self.symbols.len() as u32
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
        form: DeclarationForm,
        space: SymbolSpace,
        binding: SymbolBinding,
        key: Option<StaticKey>,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
    ) -> (LocalSymbolId, LocalScopeMark) {
        let symbol_id = LocalSymbolId::new(self.symbols.len() as u32);
        let symbol = Symbol {
            kind,
            form,
            space,
            binding,
            binding_mutability: None,
            binding_category: BindingCategory::Unclassified,
            origin: SymbolOrigin::Primary,
            key,
            scope,
            module_id: self.module_id,
            export,
            declaration: None,
            attributes: SymbolAttributes::default(),
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
        let scope_id = LocalScopeId::new(self.scopes.len() as u32);
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
        tree: &Tree,
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
}
