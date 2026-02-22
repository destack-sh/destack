use super::super::member::MemberResolution;
use crate::analyze::common::RelationMode;
use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Compiler};
use destack_dir::{Expression, InferTable, NormalizationMode, Type, TypeTable};
use destack_workspace::{Module, ProfileId};
use std::collections::HashSet;

impl Compiler {
    /// Report deferred missing member obligations after infer convergence.
    pub(crate) fn report_missing_member_obligation_errors_after_infer_convergence(
        &self,
        module: &Module,
        profile: ProfileId,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        // collect deferred obligations from infer state
        let obligations = infer.take_missing_member_obligations();

        // skip when there are no obligations
        if obligations.is_empty() {
            return Ok(());
        }

        // read module owned semantic tables once
        let tree = module.dir(profile).tree.read();
        let symbols = module.dir(profile).symbols.read();
        let mut reported = HashSet::new();

        // replay each obligation after solver convergence
        for obligation in obligations {
            // skip obligations attached to other modules
            if obligation.expression_id.module_id != module.id
                || obligation.receiver_expression_id.module_id != module.id
            {
                continue;
            }

            // decode expression ids from global obligation payload
            let expression_id = obligation
                .expression_id
                .local_id
                .try_into_typed::<Expression>()
                .map_err(|_| AnalyzeError::Internal {
                    message: "missing member obligation expression is not an expression"
                        .to_string(),
                })?;
            let receiver_expression_id = obligation
                .receiver_expression_id
                .local_id
                .try_into_typed::<Expression>()
                .map_err(|_| AnalyzeError::Internal {
                    message: "missing member obligation receiver is not an expression".to_string(),
                })?;

            // recover and normalize the receiver type for replayed lookup
            let mut receiver_ty_id = types
                .get_inferred_type_id(obligation.receiver_expression_id)
                .unwrap_or(obligation.receiver_type_id);
            if self.type_is_solver_placeholder(receiver_ty_id, types)
                && let Some(receiver_symbol) = tree.get(receiver_expression_id).target_symbol()
                && let Some(symbol_type_id) =
                    types.get_type_id_for_symbol(&symbols, receiver_symbol)
            {
                receiver_ty_id = symbol_type_id;
            }
            let receiver_ty_id = self.materialize_infer_type_for_check(
                module,
                profile,
                &symbols,
                receiver_ty_id,
                infer,
                types,
                options,
            );
            let receiver_ty_id = self.normalize_apparent_type(
                module,
                profile,
                receiver_ty_id,
                &symbols,
                types,
                NormalizationMode::Assign,
                RelationMode::ASSIGN,
            );
            if self.type_is_solver_placeholder(receiver_ty_id, types) {
                continue;
            }

            // suppress cascades after primary receiver failure
            if self.type_blocks_cascading_diagnostic(receiver_ty_id, types) {
                continue;
            }

            // fail closed when replay still depends on unsolved state
            if self.type_relation_requires_infer_convergence(
                module,
                profile,
                receiver_ty_id,
                receiver_ty_id,
                &symbols,
                types,
            ) {
                if self.type_blocks_cascading_diagnostic(receiver_ty_id, types) {
                    continue;
                }

                return Err(AnalyzeError::Internal {
                    message: "unresolved missing member obligation after infer convergence"
                        .to_string(),
                });
            }

            // resolve member dispatch for the converged receiver
            let receiver_ty = types.get_type(receiver_ty_id).clone();
            let receiver_context = self.query_member_receiver_context_for_expression(
                module,
                receiver_expression_id,
                Some(receiver_ty_id),
                &receiver_ty,
                profile,
                &tree,
                &symbols,
                types,
            );
            let member_resolution = self.resolve_member_symbol_for_receiver(
                module,
                receiver_expression_id,
                &receiver_ty,
                &receiver_context,
                &obligation.member_key,
                profile,
                &tree,
                &symbols,
                types,
            )?;
            if !matches!(
                member_resolution,
                MemberResolution::None | MemberResolution::Unresolved
            ) {
                self.commit_member_resolution(
                    expression_id.into_global_any(module.id),
                    Some(receiver_ty_id),
                    &member_resolution,
                    None,
                    None,
                    true,
                    types,
                );
                continue;
            }

            // resolve index signature value access fallback
            let mut index_visited = Vec::new();
            let index_type_id = self.infer_index_signature_value_type_for_key(
                module,
                profile,
                expression_id.into_any(),
                &symbols,
                &receiver_ty,
                &obligation.member_key,
                types,
                &mut index_visited,
            );
            if index_type_id.is_some() {
                self.commit_member_resolution(
                    expression_id.into_global_any(module.id),
                    Some(receiver_ty_id),
                    &member_resolution,
                    None,
                    None,
                    true,
                    types,
                );
                continue;
            }

            // emit at most one diagnostic per expression id
            if !reported.insert(expression_id.id) {
                continue;
            }

            // report missing member diagnostics for unresolved lookups
            let allow_associated_contract_blocker = self
                .query_expression_is_projection_receiver_for_infer(
                    module,
                    profile,
                    receiver_expression_id,
                    &tree,
                    &symbols,
                    types,
                );
            let did_report = self.report_missing_member_diagnostic_for_receiver_type(
                module,
                profile,
                expression_id,
                receiver_ty_id,
                obligation.member_key.clone(),
                &symbols,
                types,
                allow_associated_contract_blocker,
            )?;
            if !did_report {
                continue;
            }

            // commit an error type on the expression after reporting
            let error_type_id = types.insert_type_from(Type::Error, expression_id);
            types.set_inferred_type(obligation.expression_id, error_type_id);
        }

        Ok(())
    }
}
