use crate::analyze::common::{TypeContext, TypeRewriteCache};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{GlobalSymbolId, InferTable, LocalTypeId, StaticArgument, StaticExpression};
use std::collections::HashMap;

impl Compiler {
    /// Rewrite inferred-type overlays using solved instance substitutions.
    pub(in crate::analyze::solve) fn rewrite_inferred_type_overlays_for_instance_substitutions(
        &self,
        ctx: &mut TypeContext<'_>,
        infer: &mut InferTable,
    ) -> AnalyzeResult<()> {
        // apply substitutions to inferred overlays using explicit instance obligations
        let mut node_attachments = infer
            .iter_instance_commit_obligation_nodes()
            .collect::<Vec<_>>();
        node_attachments.sort_by_key(|(node_id, _)| *node_id);
        for (node_id, obligation_id) in node_attachments {
            // skip attachments that do not belong to this module
            if node_id.module_id != ctx.module.id {
                continue;
            }

            // skip nodes without inferred overlay types
            let Some(inferred_type_id) = infer.inferred_type_for_node(node_id) else {
                continue;
            };
            let Some(obligation) = infer.instance_commit_obligation(obligation_id) else {
                return Err(AnalyzeError::Internal {
                    message: "missing instance commit obligation for inferred node".to_string(),
                });
            };

            // build type substitutions from the committed obligation environment
            let mut substitutions = HashMap::<GlobalSymbolId, LocalTypeId>::new();
            for (parameter_symbol, argument) in obligation
                .generic_parameter_symbols
                .iter()
                .zip(obligation.generic_arguments.iter())
            {
                let StaticArgument::Evaluated {
                    value: StaticExpression::Type { ty },
                    ..
                } = argument
                else {
                    continue;
                };
                substitutions.insert(*parameter_symbol, ctx.types.unwrap_value_type_id(*ty));
            }
            if substitutions.is_empty() {
                continue;
            }

            // instantiate inferred type with committed substitutions
            let mut materialize_cache = TypeRewriteCache::new();
            let mut substitution_cache = HashMap::new();
            let mapped_type_id = self.instantiate_type_with_substitutions(
                &mut ctx.reborrow(),
                node_id.local_id,
                None,
                inferred_type_id,
                &substitutions,
                &mut materialize_cache,
                &mut substitution_cache,
            );

            // publish mapped type ids when instantiation changed the result
            if mapped_type_id != inferred_type_id {
                infer.set_inferred_type_for_node(node_id, mapped_type_id);
            }
        }

        Ok(())
    }
}
