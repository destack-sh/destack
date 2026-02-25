use crate::analyze::common::{AnalyzeDependencyStage, TypeTablesContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{Expression, GlobalSymbolId, LocalNodeId, LocalTypeId, StaticParameterKind};
use std::collections::HashMap;

impl Compiler {
    pub(crate) fn static_parameter_reference(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, StaticParameterKind)>> {
        // only treat references as static parameters in Destack modules
        if !tables.module.language_type.is_destack() {
            return Ok(None);
        }

        // unwrap explicit comptime wrappers to reach the reference
        let (expression_id, _) = self.unwrap_as_comptime_expression(expression_id, tables.tree);

        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = tables.tree.get(expression_id)
        else {
            return Ok(None);
        };

        self.with_module_tree_symbols_or_local_at_stage(
            tables.module,
            tables.profile,
            target_symbol.module_id,
            tables.tree,
            tables.symbols,
            AnalyzeDependencyStage::Declare,
            |owner_module, owner_tree, owner_symbols| {
                let owner_options = self.analyze_context_options_for_module(owner_module.id);
                let mut owner_tables = tables.reborrow_for_module_with_options(
                    owner_module,
                    &owner_options,
                    owner_tree,
                    owner_symbols,
                );
                self.static_parameter_reference_in_symbols(
                    &mut owner_tables.reborrow(),
                    *target_symbol,
                )
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the static parameter kind for a reference expression.

    pub(crate) fn static_parameter_reference_kind(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<StaticParameterKind>> {
        Ok(self
            .static_parameter_reference(&mut tables.reborrow(), expression_id)?
            .map(|(_, kind)| kind))
    }

    /// Resolve the static parameter symbol and kind for a symbol within a symbol table.

    pub(crate) fn static_parameter_reference_in_symbols(
        &self,
        tables: &mut TypeTablesContext<'_>,
        target_symbol: GlobalSymbolId,
    ) -> Option<(GlobalSymbolId, StaticParameterKind)> {
        // resolve direct static parameter references
        if self.symbol_is_static_parameter(
            tables.module,
            tables.profile,
            target_symbol,
            tables.symbols,
            tables.types,
        ) {
            let kind = self.static_parameter_kind_for_symbol(&mut tables.reborrow(), target_symbol);
            return Some((target_symbol, kind));
        }

        // fall back to a same-scope static parameter with the same key
        let symbol_entry = tables.symbols.get_symbol(target_symbol.local_id);
        let key = symbol_entry.key?;
        let mut scope_cursor = Some(symbol_entry.scope);
        while let Some((scope_id, mark)) = scope_cursor {
            let scope = tables.symbols.get_scope_by_id(scope_id);
            let limit = mark.0 as usize;
            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().take(limit).rev()
            {
                if *candidate_key != key {
                    continue;
                }
                let candidate_symbol = tables.symbols.get_symbol(*candidate_symbol_id);
                if !candidate_symbol.is_active || !candidate_symbol.is_static_parameter() {
                    continue;
                }
                let candidate_global = candidate_symbol_id.into_global(tables.module.id);
                let kind =
                    self.static_parameter_kind_for_symbol(&mut tables.reborrow(), candidate_global);
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
