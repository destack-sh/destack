use std::collections::HashSet;

use crate::analyze::RelationMode;
use crate::analyze::common::CommitContext;
use crate::{AnalyzeResult, Compiler, InferState};
use destack_dir::{Declarator, InferTable, NormalizationMode, Pattern};

impl Compiler {
    /// Commit direct-binding value types from authored annotations and solved initializers.
    pub(super) fn commit_binding_value_types(
        &self,
        ctx: &mut CommitContext<'_>,
        infer: &InferTable,
    ) -> AnalyzeResult<()> {
        let mut actions = Vec::new();

        // commit direct declarator annotations from exact syntax
        for (declarator_id, declarator) in ctx.tree.iter_nodes_of_type::<Declarator>() {
            if !self.is_node_active(ctx.tree, ctx.symbols, declarator_id.into_any()) {
                continue;
            }

            let Pattern::Binding {
                symbol, pattern, ..
            } = ctx.tree.get(declarator.pattern)
            else {
                continue;
            };
            if pattern.is_some() {
                continue;
            }

            let Some(annotation_id) = declarator.ty else {
                continue;
            };

            // resolve the authored annotation and normalize it once for the committed table
            let committed_type = self.resolve_declared_type_expression_value(
                &mut ctx.type_context_reborrow(),
                annotation_id,
                true,
                true,
                true,
                false,
                false,
            )?;
            let committed_type_id = ctx.types.insert_type_from(committed_type, annotation_id);
            let committed_type_id = self.normalize_type_with_relation(
                &mut ctx.type_context_reborrow(),
                committed_type_id,
                NormalizationMode::Assign,
                RelationMode::OBJECT_SHAPE,
            );
            actions.push((symbol.into_global(ctx.module.id), committed_type_id));
        }

        let mut committed_symbols = actions
            .iter()
            .map(|(symbol_id, _)| *symbol_id)
            .collect::<HashSet<_>>();

        // consume infer-recorded write intents in deterministic symbol or node order
        let mut intents = infer
            .iter_direct_binding_value_commit_intents()
            .filter(|(symbol_id, _, _)| symbol_id.module_id == ctx.module.id)
            .collect::<Vec<_>>();
        intents.sort_by_key(|(symbol_id, declarator_id, value_id)| {
            (symbol_id.local_id.id, declarator_id.id, value_id.id)
        });

        // commit one deterministic binding value type per direct initializer binding
        for (symbol_id, declarator_id, value_id) in intents {
            if committed_symbols.contains(&symbol_id) {
                continue;
            }

            if !self.is_node_active(ctx.tree, ctx.symbols, declarator_id.into_any()) {
                continue;
            }

            let declarator_node_id = declarator_id.into_global_any(ctx.module.id);
            if ctx.types.get_declared_type_id(declarator_node_id).is_some() {
                continue;
            }

            let inferred_node_id = value_id.into_global_any(ctx.module.id);
            let Some(inferred_type_id) = ctx.types.get_inferred_type_id(inferred_node_id) else {
                continue;
            };

            // materialize the initializer under the binding mutability contract
            let binding_mutability = ctx
                .symbols
                .get_symbol(symbol_id.local_id)
                .binding_mutability;
            let materialize_ctx = InferState::new(ctx.profile, *ctx.options)
                .with_binding_initializer(binding_mutability);
            let committed_type_id = self.materialize_declarator_initializer_type(
                &mut ctx.type_context_reborrow(),
                declarator_id,
                inferred_type_id,
                &materialize_ctx,
            );
            actions.push((symbol_id, committed_type_id));
            committed_symbols.insert(symbol_id);
        }

        // publish the committed value types in the collected deterministic order
        for (symbol_id, type_id) in actions {
            ctx.types.set_value_type(symbol_id, type_id);
        }

        Ok(())
    }
}
