use dyst_dir::{
    LocalScopeId, LocalScopeMark, LocalSymbolId, Module, ScopeKind, SymbolKey, SymbolKind,
    SymbolSpace, SymbolTable,
};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
impl Compiler {
    /// Bind a new named item.
    pub(super) fn bind_named_item(
        &self,
        _module: &Module,
        space: SymbolSpace,
        key: SymbolKey,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(SymbolKind::Item, space, Some(key), scope)
    }

    /// Bind a new anonymous item.
    pub(super) fn bind_anonymous_item(
        &self,
        _module: &Module,
        space: SymbolSpace,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(SymbolKind::Item, space, None, scope)
    }

    /// Bind a new named item with an owned scope. Symbols belongs to outer scope.
    pub(super) fn bind_named_item_with_scope(
        &self,
        _module: &Module,
        space: SymbolSpace,
        key: SymbolKey,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) = symbols.insert_symbol(SymbolKind::Item, space, Some(key), scope);
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Bind a new anonymous item with an owned scope. Symbol belongs to outer scope.
    pub(super) fn bind_anonymous_item_with_scope(
        &self,
        _module: &Module,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) =
            symbols.insert_symbol(SymbolKind::Item, SymbolSpace::Value, None, scope);
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Bind a new named local.
    pub(super) fn bind_named_local(
        &self,
        _module: &Module,
        space: SymbolSpace,
        key: SymbolKey,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(SymbolKind::Local, space, Some(key), scope)
    }

    /// Bind a new anonymous local.
    pub(super) fn bind_anonymous_local(
        &self,
        _module: &Module,
        space: SymbolSpace,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(SymbolKind::Local, space, None, scope)
    }

    /// Bind a new named local with an owned scope. Symbol belongs to outer scope.
    pub(super) fn bind_named_local_with_scope(
        &self,
        _module: &Module,
        space: SymbolSpace,
        key: SymbolKey,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) = symbols.insert_symbol(SymbolKind::Local, space, Some(key), scope);
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Bind a new anonymous local with an owned scope. Symbol belongs to outer scope.
    pub(super) fn bind_anonymous_local_with_scope(
        &self,
        _module: &Module,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) =
            symbols.insert_symbol(SymbolKind::Local, SymbolSpace::Value, None, scope);
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }
}
