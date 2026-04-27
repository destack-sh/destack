use crate::resolve::binding::cache::ResolveScopeIndexCache;
use destack_dir::{
    BindingCategory, Declaration, Expression, LocalNodeId, LocalScopeId, LocalScopeMark,
    LocalSymbolId, Name, NodeType, Scope, StaticKey, SymbolSpace, SymbolSpaceOrder, SymbolTable,
    SymbolType, Tree,
};
use destack_workspace::Module;

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return the namespace scope used for module binding lookups in global declarations.
    pub(crate) fn module_binding_scope_for_global_expression(
        &self,
        tree: &Tree,
        expression_id: LocalNodeId<Expression>,
    ) -> Option<LocalScopeId> {
        // track the nearest global declaration in the parent chain
        let mut node = expression_id.into_any();
        let mut saw_global = false;

        // walk up the parent chain until the root
        loop {
            let parent = tree.get_parent(node.id)?;

            // inspect declaration ancestors
            if parent.ty == NodeType::Declaration {
                let declaration_id: LocalNodeId<Declaration> = LocalNodeId::new(parent.id);
                let declaration = tree.get(declaration_id);

                // remember when we are inside a global declaration
                if matches!(declaration, Declaration::Global(_)) {
                    saw_global = true;
                }

                // resolve module binding scopes that contain a global declaration
                if saw_global
                    && let Declaration::Namespace(declaration) = declaration
                    && matches!(declaration.name, Name::String(_))
                {
                    return Some(declaration.scope);
                }
            }

            node = parent;
        }
    }

    /// Find the best matching symbol for a key within a single scope.
    pub(crate) fn find_symbol_in_scope(
        &self,
        scope: &Scope,
        key: StaticKey,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        limit: Option<usize>,
    ) -> (Option<LocalSymbolId>, Option<LocalSymbolId>) {
        // limit to the visible symbol range
        let limit = limit.unwrap_or(scope.named_symbols.len());
        let limit = limit.min(scope.named_symbols.len());
        let named_symbols = &scope.named_symbols[..limit];

        // collect the nearest symbol per space
        let mut type_symbol = None;
        let mut value_symbol = None;
        let mut type_value_symbol = None;
        let mut fallback = None;
        for (candidate_key, symbol_id) in named_symbols.iter().rev() {
            if *candidate_key != key {
                continue;
            }
            let symbol = symbols.get_symbol(*symbol_id);
            if !symbol.is_active() {
                continue;
            }
            match symbol.space {
                SymbolSpace::Type => {
                    if type_symbol.is_none() {
                        type_symbol = Some(*symbol_id);
                    }
                }
                SymbolSpace::Value => {
                    if value_symbol.is_none() {
                        value_symbol = Some(*symbol_id);
                    }
                }
                SymbolSpace::TypeValue => {
                    if type_value_symbol.is_none() {
                        type_value_symbol = Some(*symbol_id);
                    }
                }
                SymbolSpace::Label => {}
            }
            if fallback.is_none() {
                fallback = Some(*symbol_id);
            }
        }

        // choose the preferred symbol from the space order
        let preferred = space_order.spaces().iter().find_map(|space| match space {
            SymbolSpace::Type => type_symbol.or(type_value_symbol),
            SymbolSpace::Value => value_symbol.or(type_value_symbol),
            SymbolSpace::TypeValue => type_value_symbol,
            SymbolSpace::Label => None,
        });

        (preferred, fallback)
    }

    /// Find the best matching symbol for a key within a single scope using an index cache.
    pub(crate) fn find_symbol_in_scope_cached(
        &self,
        scope_id: LocalScopeId,
        scope: &Scope,
        key: StaticKey,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        mark: LocalScopeMark,
        scope_cache: Option<&mut ResolveScopeIndexCache>,
    ) -> (Option<LocalSymbolId>, Option<LocalSymbolId>) {
        if let Some(scope_cache) = scope_cache {
            let index = scope_cache.scope_index(scope_id, scope, symbols);
            return index.lookup(key, mark, space_order);
        }

        let limit = mark.0 as usize;
        self.find_symbol_in_scope(scope, key, space_order, symbols, Some(limit))
    }

    /// Return true when a symbol participates in runtime hoisting lookup.
    fn symbol_is_runtime_hoisted(&self, symbol_id: LocalSymbolId, symbols: &SymbolTable) -> bool {
        let symbol = symbols.get_symbol(symbol_id);
        if !symbol.is_active() {
            return false;
        }

        symbol.binding_category == BindingCategory::FunctionScoped
            || symbol.ty == SymbolType::Function
    }

    /// Find a hoisted value symbol that appears after the current scope mark.
    ///
    /// This models JS/TS hoisting for `var` and function declarations.
    pub(super) fn find_hoisted_symbol_in_scope(
        &self,
        module: &Module,
        scope: &Scope,
        key: StaticKey,
        space_order: SymbolSpaceOrder,
        symbols: &SymbolTable,
        mark: LocalScopeMark,
    ) -> Option<LocalSymbolId> {
        // hoisting only applies to value lookups
        if !space_order.spaces().contains(&SymbolSpace::Value) {
            return None;
        }

        // js/ts allow var and function references before declaration during binding lookup
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return None;
        }

        let limit = (mark.0 as usize).min(scope.named_symbols.len());
        if limit == scope.named_symbols.len() {
            return None;
        }

        let mut value_symbol = None;
        let mut type_value_symbol = None;
        for (candidate_key, symbol_id) in scope.named_symbols[limit..].iter().rev() {
            if *candidate_key != key {
                continue;
            }

            if !self.symbol_is_runtime_hoisted(*symbol_id, symbols) {
                continue;
            }

            let symbol = symbols.get_symbol(*symbol_id);
            match symbol.space {
                SymbolSpace::Value => {
                    if value_symbol.is_none() {
                        value_symbol = Some(*symbol_id);
                    }
                }
                SymbolSpace::TypeValue => {
                    if type_value_symbol.is_none() {
                        type_value_symbol = Some(*symbol_id);
                    }
                }
                SymbolSpace::Type | SymbolSpace::Label => {}
            }
        }

        // choose by space preference, similar to regular lookup
        space_order.spaces().iter().find_map(|space| match space {
            SymbolSpace::Value => value_symbol.or(type_value_symbol),
            SymbolSpace::TypeValue => type_value_symbol,
            SymbolSpace::Type | SymbolSpace::Label => None,
        })
    }

    /// Return true when unresolved same-scope references should consider forward bindings.
    pub(super) fn allow_forward_binding_lookup(
        &self,
        module: &Module,
        space_order: SymbolSpaceOrder,
    ) -> bool {
        // JS/TS bind lexical names for the whole scope, with TDZ at runtime
        if !(module.language_type.is_javascript() || module.language_type.is_typescript()) {
            return false;
        }

        !space_order.spaces().is_empty()
    }
}
