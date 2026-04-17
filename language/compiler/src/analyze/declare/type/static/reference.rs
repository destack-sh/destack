use crate::analyze::common::{AnalyzeIndex, TypeContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GenericParameterKind, GlobalSymbolId, LocalNodeId, LocalTypeId, SymbolSpace,
    TypeExpression,
};
use std::collections::HashMap;

impl Compiler {
    pub(crate) fn generic_parameter_expression_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, GenericParameterKind)>> {
        // only treat references as static parameters in Destack modules
        if !ctx.module.language_type.is_destack() {
            return Ok(None);
        }

        // unwrap one embedded type expression when the value is spelled in type space
        let expression_id = self.unwrap_parenthesized_expression(expression_id, ctx.tree);
        if let Expression::Type { value, .. } = ctx.tree.get(expression_id) {
            return self.generic_parameter_type_reference(&mut ctx.reborrow(), *value);
        }

        // resolve the referenced symbol first
        let (Expression::LocalReference { target_symbol, .. }
        | Expression::ModuleReference { target_symbol, .. }
        | Expression::GlobalReference { target_symbol, .. }) = ctx.tree.get(expression_id)
        else {
            return Ok(None);
        };

        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            target_symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                let owner_options = ctx
                    .compiler_context
                    .analyze_context_options_for_module(view.module.id);
                let mut ctx = TypeContext::new(
                    ctx.compiler_context,
                    view.module,
                    ctx.profile,
                    &owner_options,
                    view.tree,
                    view.symbols,
                    ctx.types,
                    AnalyzeIndex::default(),
                );
                self.generic_parameter_reference_in_symbols(&mut ctx.reborrow(), *target_symbol)
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the static parameter symbol and kind for a reference type expression.
    pub(crate) fn generic_parameter_type_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<Option<(GlobalSymbolId, GenericParameterKind)>> {
        // only treat references as static parameters in Destack modules
        if !ctx.module.language_type.is_destack() {
            return Ok(None);
        }

        // unwrap explicit comptime wrappers to reach the reference
        let (expression_id, _) = self.unwrap_as_comptime_type_expression(expression_id, ctx.tree);

        // resolve the referenced symbol first
        let (TypeExpression::LocalReference { target_symbol, .. }
        | TypeExpression::ModuleReference { target_symbol, .. }
        | TypeExpression::GlobalReference { target_symbol, .. }) = ctx.tree.get(expression_id)
        else {
            return Ok(None);
        };

        self.with_module_tree_symbol_view_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            target_symbol.module_id,
            ctx.tree,
            ctx.symbols,
            destack_artifact::ArtifactKey::dir_declared,
            |view| {
                let owner_options = ctx
                    .compiler_context
                    .analyze_context_options_for_module(view.module.id);
                let mut ctx = TypeContext::new(
                    ctx.compiler_context,
                    view.module,
                    ctx.profile,
                    &owner_options,
                    view.tree,
                    view.symbols,
                    ctx.types,
                    AnalyzeIndex::default(),
                );
                self.generic_parameter_reference_in_symbols(&mut ctx.reborrow(), *target_symbol)
            },
        )
        .map_err(AnalyzeError::from)
    }

    /// Resolve the static parameter kind for a reference type expression.
    pub(crate) fn generic_parameter_type_reference_kind(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<Option<GenericParameterKind>> {
        Ok(self
            .generic_parameter_type_reference(&mut ctx.reborrow(), expression_id)?
            .map(|(_, kind)| kind))
    }

    /// Resolve the static parameter symbol and kind for a symbol within a symbol table.
    pub(crate) fn generic_parameter_reference_in_symbols(
        &self,
        ctx: &mut TypeContext<'_>,
        target_symbol: GlobalSymbolId,
    ) -> Option<(GlobalSymbolId, GenericParameterKind)> {
        // resolve direct static parameter references
        if self.symbol_is_static_parameter(ctx.symbol_type_view(), target_symbol) {
            let kind = self.generic_parameter_kind_for_symbol(&mut ctx.reborrow(), target_symbol);
            return Some((target_symbol, kind));
        }

        // fall back to a same-scope static parameter with the same key
        let symbol_entry = ctx.symbols.get_symbol(target_symbol.local_id);
        if symbol_entry.space != SymbolSpace::Type {
            return None;
        }
        let key = symbol_entry.key?;
        let mut scope_cursor = Some(symbol_entry.scope);
        while let Some((scope_id, mark)) = scope_cursor {
            let scope = ctx.symbols.get_scope_by_id(scope_id);
            let limit = mark.0 as usize;
            for (candidate_key, candidate_symbol_id) in scope.named_symbols.iter().take(limit).rev()
            {
                if *candidate_key != key {
                    continue;
                }
                let candidate_symbol = ctx.symbols.get_symbol(*candidate_symbol_id);
                if !candidate_symbol.is_active || !candidate_symbol.is_static_parameter() {
                    continue;
                }
                let candidate_global = candidate_symbol_id.into_global(ctx.module.id);
                let kind =
                    self.generic_parameter_kind_for_symbol(&mut ctx.reborrow(), candidate_global);
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
