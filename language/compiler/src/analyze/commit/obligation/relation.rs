use destack_dir::{
    LocalTypeId, SymbolTable, TypeRelationObligation, TypeRelationObligationDiagnostic,
    TypeRelationObligationOperands, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler};

impl Compiler {
    /// Replay post-solve type relation obligations from explicit records.
    pub(in crate::analyze::commit) fn replay_type_relation_obligations_from_records(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        obligations: &[TypeRelationObligation],
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        // replay each relation obligation with converged operand types
        for obligation in obligations {
            // skip obligations that belong to other modules
            if obligation.source_node_id.module_id != module.id {
                continue;
            }

            // resolve effective operands from captured inputs or expression ids
            let (target_type_id, source_type_id) =
                self.resolve_type_relation_obligation_operands(module, types, &obligation)?;

            if self.type_relation_requires_infer_convergence(
                module,
                profile,
                target_type_id,
                source_type_id,
                symbols,
                types,
            ) {
                // drop unresolved relations only when blocked by a primary error
                if self.type_relation_operands_have_primary_error(
                    target_type_id,
                    source_type_id,
                    types,
                ) {
                    continue;
                }

                return Err(AnalyzeError::Internal {
                    message: "unresolved type relation obligation after infer convergence"
                        .to_string(),
                });
            }

            // skip cascading diagnostics once one side already failed
            if self.type_relation_operands_have_primary_error(target_type_id, source_type_id, types)
            {
                continue;
            }

            // evaluate assignability with converged operands
            let assignability = self.is_type_assignable(
                module,
                profile,
                symbols,
                target_type_id,
                source_type_id,
                types,
                options,
            );
            if assignability != Assignability::NotAssignable {
                continue;
            }

            // emit the diagnostic selected by the obligation
            match obligation.diagnostic {
                TypeRelationObligationDiagnostic::UnassignableType => {
                    self.emit_unassignable_type_for_types(
                        module,
                        profile,
                        obligation.source_node_id.local_id,
                        target_type_id,
                        source_type_id,
                        types,
                    );
                }
                TypeRelationObligationDiagnostic::UnsatisfiedType => {
                    self.report_unsatisfied_type_for_types(
                        module,
                        profile,
                        obligation.source_node_id.local_id,
                        target_type_id,
                        source_type_id,
                        types,
                    );
                }
            }
        }

        Ok(())
    }

    /// Return true when at least one relation operand already carries a primary error.
    fn type_relation_operands_have_primary_error(
        &self,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        // inspect the target side first
        let mut target_visited = std::collections::HashSet::new();
        if self.type_contains_error(target_type_id, types, &mut target_visited) {
            return true;
        }

        // then inspect the source side
        let mut source_visited = std::collections::HashSet::new();
        self.type_contains_error(source_type_id, types, &mut source_visited)
    }

    /// Resolve operand type ids for one relation obligation after convergence.
    fn resolve_type_relation_obligation_operands(
        &self,
        module: &Module,
        types: &TypeTable,
        obligation: &TypeRelationObligation,
    ) -> AnalyzeResult<(LocalTypeId, LocalTypeId)> {
        // resolve captured type operands directly
        match &obligation.operands {
            TypeRelationObligationOperands::CapturedTypes {
                target_type_id,
                source_type_id,
            } => Ok((*target_type_id, *source_type_id)),
            TypeRelationObligationOperands::ExpressionOperands {
                target_expression_id,
                source_expression_id,
            } => {
                // reject cross module expression operands
                if target_expression_id.module_id != module.id
                    || source_expression_id.module_id != module.id
                {
                    return Err(AnalyzeError::Internal {
                        message: "cross-module type relation expression operands are not supported"
                            .to_string(),
                    });
                }

                let Some(source_type_id) = types.get_inferred_type_id(*source_expression_id) else {
                    return Err(AnalyzeError::Internal {
                        message: "missing inferred source operand for type relation obligation"
                            .to_string(),
                    });
                };
                let Some(target_type_id) = types.get_inferred_type_id(*target_expression_id) else {
                    return Err(AnalyzeError::Internal {
                        message: "missing inferred target operand for type relation obligation"
                            .to_string(),
                    });
                };

                // normalize value wrappers for replayed relation checks
                let target_type_id = types.unwrap_value_type_id(target_type_id);
                let source_type_id = types.unwrap_value_type_id(source_type_id);
                Ok((target_type_id, source_type_id))
            }
        }
    }
}
