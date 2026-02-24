use crate::analyze::common::AnalyzeDependencyStage;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, NodeTree, StaticParameterKind,
    SymbolTable, TypeTable,
};
use destack_workspace::{Module, ProfileId};
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    pub(crate) fn static_parameter_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, StaticParameterKind)>> {
        // only treat references as static parameters in Destack modules
        if !module.language_type.is_destack() {
            return Ok(None);
        }

        // unwrap explicit comptime wrappers to reach the reference
        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tree);

        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tree.get(expression_id)
        else {
            return Ok(None);
        };

        self.with_module_tree_symbols_or_local_at_stage(
            module,
            profile,
            target_symbol.module_id,
            tree,
            symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                self.static_parameter_reference_in_symbols(
                    owner_module,
                    profile,
                    *target_symbol,
                    owner_tree,
                    owner_symbols,
                    types,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the static parameter kind for a reference expression.

    pub(crate) fn static_parameter_reference_kind(
        &self,
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<StaticParameterKind>> {
        Ok(self
            .static_parameter_reference(module, profile, expression_id, tree, symbols, types)?
            .map(|(_, kind)| kind))
    }

    /// Resolve the static parameter symbol and kind for a symbol within a symbol table.

    pub(crate) fn static_parameter_reference_in_symbols(
        &self,
        module: &Module,
        profile: ProfileId,
        target_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> Option<(GlobalSymbolId, StaticParameterKind)> {
        // resolve direct static parameter references
        if self.symbol_is_static_parameter(module, profile, target_symbol, symbols, types) {
            let kind = self.static_parameter_kind_for_symbol(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
                types,
            );
            return Some((target_symbol, kind));
        }

        // fall back to a same-scope static parameter with the same key
        let symbol_entry = symbols.get_symbol(target_symbol.local_id);
        let key = symbol_entry.key?;
        let mut scope_cursor = Some(symbol_entry.scope);
        while let Some((scope_id, mark)) = scope_cursor {
            let scope = symbols.get_scope_by_id(scope_id);
            let limit = mark.0 as usize;
            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().take(limit).rev()
            {
                if *candidate_key != key {
                    continue;
                }
                let candidate_symbol = symbols.get_symbol(*candidate_symbol_id);
                if !candidate_symbol.is_active || !candidate_symbol.is_static_parameter() {
                    continue;
                }
                let candidate_global = candidate_symbol_id.into_global(module.id);
                let kind = self.static_parameter_kind_for_symbol(
                    module,
                    profile,
                    candidate_global,
                    tree,
                    symbols,
                    types,
                );
                return Some((candidate_global, kind));
            }

            scope_cursor = scope.parent;
        }

        None
    }

    /// Look up one substitution type for a static parameter symbol.

    pub(crate) fn substitution_type_id_for_static_parameter(
        &self,
        symbol: GlobalSymbolId,
        substitutions: &HashMap<GlobalSymbolId, LocalTypeId>,
    ) -> Option<LocalTypeId> {
        if let Some(type_id) = substitutions.get(&symbol) {
            return Some(*type_id);
        }

        substitutions.iter().find_map(|(candidate, type_id)| {
            let matches_symbol = candidate.module_id == symbol.module_id
                && candidate.local_id.id == symbol.local_id.id;
            if matches_symbol { Some(*type_id) } else { None }
        })
    }
}
