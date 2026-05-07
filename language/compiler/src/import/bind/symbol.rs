use destack_dir::{
    DeclarationForm, ExportKind, LocalScopeId, LocalScopeMark, LocalSymbolId, ScopeKind, StaticKey,
    SymbolBinding, SymbolKind, SymbolSpace, SymbolTable,
};

use crate::Compiler;

use destack_artifact::Ast;
use destack_workspace::Module;

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
impl Compiler {
    /// Find the nearest ancestor scope of a given kind, starting from the given scope.
    pub(super) fn find_nearest_scope_of_kind(
        &self,
        symbols: &SymbolTable,
        scope: (LocalScopeId, LocalScopeMark),
        kind: ScopeKind,
    ) -> Option<(LocalScopeId, LocalScopeMark)> {
        let mut current_scope_id = scope.0;
        loop {
            let current_scope = symbols.get_scope_by_id(current_scope_id);
            if current_scope.kind == kind {
                return Some((current_scope_id, LocalScopeMark::end()));
            }
            match current_scope.parent {
                Some((parent_id, _)) => current_scope_id = parent_id,
                None => return None,
            }
        }
    }

    /// Bind a new named item.
    #[inline]
    pub(super) fn bind_named_item(
        &self,
        _module: &Module,
        _ast: &Ast,
        space: SymbolSpace,
        key: StaticKey,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(
            SymbolKind::Item,
            DeclarationForm::Void,
            space,
            SymbolBinding::Runtime,
            Some(key),
            scope,
            export,
        )
    }

    /// Bind a new named item with an owned scope. Symbols belongs to outer scope.
    #[inline]
    pub(super) fn bind_named_item_with_scope(
        &self,
        _module: &Module,
        _ast: &Ast,
        space: SymbolSpace,
        key: StaticKey,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Item,
            DeclarationForm::Void,
            space,
            SymbolBinding::Runtime,
            Some(key),
            scope,
            export,
        );
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Bind a new anonymous item.
    #[inline]
    pub(super) fn bind_anonymous_item(
        &self,
        _module: &Module,
        _ast: &Ast,
        space: SymbolSpace,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(
            SymbolKind::Item,
            DeclarationForm::Void,
            space,
            SymbolBinding::Runtime,
            None,
            scope,
            export,
        )
    }

    /// Bind a new anonymous item with an owned scope. Symbol belongs to outer scope.
    #[inline]
    pub(super) fn bind_anonymous_item_with_scope(
        &self,
        _module: &Module,
        _ast: &Ast,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Item,
            DeclarationForm::Void,
            SymbolSpace::Value,
            SymbolBinding::Runtime,
            None,
            scope,
            export,
        );
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Bind a new named local.
    #[inline]
    pub(super) fn bind_named_local(
        &self,
        _module: &Module,
        _ast: &Ast,
        space: SymbolSpace,
        key: StaticKey,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(
            SymbolKind::Local,
            DeclarationForm::Void,
            space,
            SymbolBinding::Runtime,
            Some(key),
            scope,
            None,
        )
    }

    /// Bind a new named local with an owned scope. Symbol belongs to outer scope.
    #[inline]
    pub(super) fn bind_named_local_with_scope(
        &self,
        _module: &Module,
        _ast: &Ast,
        space: SymbolSpace,
        key: StaticKey,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Local,
            DeclarationForm::Void,
            space,
            SymbolBinding::Runtime,
            Some(key),
            scope,
            None,
        );
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Bind a new anonymous local.
    #[inline]
    pub(super) fn bind_anonymous_local(
        &self,
        _module: &Module,
        _ast: &Ast,
        space: SymbolSpace,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        symbols.insert_symbol(
            SymbolKind::Local,
            DeclarationForm::Void,
            space,
            SymbolBinding::Runtime,
            None,
            scope,
            None,
        )
    }

    /// Bind a new anonymous local with an owned scope. Symbol belongs to outer scope.
    #[inline]
    pub(super) fn bind_anonymous_local_with_scope(
        &self,
        _module: &Module,
        _ast: &Ast,
        kind: ScopeKind,
        scope: (LocalScopeId, LocalScopeMark),
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeId) {
        let (symbol_id, _) = symbols.insert_symbol(
            SymbolKind::Local,
            DeclarationForm::Void,
            SymbolSpace::Value,
            SymbolBinding::Runtime,
            None,
            scope,
            None,
        );
        let scope_id = symbols.insert_scope(kind, Some(scope), Some(symbol_id));
        (symbol_id, scope_id)
    }

    /// Bind a new named item or local (if export is set, it's an item, otherwise a local).
    #[inline]
    pub(super) fn bind_named_symbol(
        &self,
        module: &Module,
        ast: &Ast,
        space: SymbolSpace,
        key: StaticKey,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        if let Some(export) = export {
            self.bind_named_item(module, ast, space, key, scope, Some(export), symbols)
        } else {
            self.bind_named_local(module, ast, space, key, scope, symbols)
        }
    }

    /// Bind a new named item or local with explicit binding type.
    #[inline]
    pub(super) fn bind_named_symbol_with_binding(
        &self,
        _module: &Module,
        _ast: &Ast,
        space: SymbolSpace,
        key: StaticKey,
        binding: SymbolBinding,
        scope: (LocalScopeId, LocalScopeMark),
        export: Option<ExportKind>,
        symbols: &mut SymbolTable,
    ) -> (LocalSymbolId, LocalScopeMark) {
        let kind = if export.is_some() {
            SymbolKind::Item
        } else {
            SymbolKind::Local
        };
        symbols.insert_symbol(
            kind,
            DeclarationForm::Void,
            space,
            binding,
            Some(key),
            scope,
            export,
        )
    }
}
