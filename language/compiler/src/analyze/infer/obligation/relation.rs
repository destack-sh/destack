use destack_dir::{
    Expression, InferTable, LocalNodeId, LocalNodeIdAny, LocalTypeId, SymbolTable,
    TypeRelationObligation, TypeRelationObligationDiagnostic, TypeRelationObligationOperands,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::{AnalyzeError, AnalyzeOptions, AnalyzeResult, Assignability, Compiler};

/// Policy for immediate unassignable diagnostics before solve convergence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnassignableRelationFailureMode {
    /// Propagate the unassignable relation as an immediate error result.
    PropagateError,
    /// Report the diagnostic and continue inference.
    ReportAndContinue,
}

impl Compiler {
    /// Return true when one relation depends on unsolved inference state.
    pub(crate) fn type_relation_requires_infer_convergence(
        &self,
        module: &Module,
        profile: ProfileId,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> bool {
        // check whether the target depends on solver state
        if self.type_requires_infer_convergence(module, profile, target_type_id, symbols, types) {
            return true;
        }

        // check whether the source depends on solver state
        if self.type_requires_infer_convergence(module, profile, source_type_id, symbols, types) {
            return true;
        }

        false
    }

    /// Record one post solve relation obligation using captured type ids.
    pub(crate) fn push_type_relation_obligation_for_captured_types(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
        diagnostic: TypeRelationObligationDiagnostic,
        infer: &mut InferTable,
    ) {
        infer.push_type_relation_obligation(TypeRelationObligation {
            source_node_id: node_id.into_global(module.id),
            operands: TypeRelationObligationOperands::CapturedTypes {
                target_type_id,
                source_type_id,
            },
            diagnostic,
        });
    }

    /// Record one post solve relation obligation using expression operands.
    pub(crate) fn push_type_relation_obligation_for_expression_operands(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        target_expression_id: LocalNodeId<Expression>,
        source_expression_id: LocalNodeId<Expression>,
        diagnostic: TypeRelationObligationDiagnostic,
        infer: &mut InferTable,
    ) {
        infer.push_type_relation_obligation(TypeRelationObligation {
            source_node_id: node_id.into_global(module.id),
            operands: TypeRelationObligationOperands::ExpressionOperands {
                target_expression_id: target_expression_id.into_global_any(module.id),
                source_expression_id: source_expression_id.into_global_any(module.id),
            },
            diagnostic,
        });
    }

    /// Enforce one assignability relation or defer its diagnostic to post solve reporting.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn enforce_assignability_or_defer_unassignable_diagnostic(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
        failure_mode: UnassignableRelationFailureMode,
    ) -> AnalyzeResult<()> {
        // defer relation diagnostics until all inference variables are solved
        if self.type_relation_requires_infer_convergence(
            module,
            profile,
            target_type_id,
            source_type_id,
            symbols,
            types,
        ) {
            self.push_type_relation_obligation_for_captured_types(
                module,
                node_id,
                target_type_id,
                source_type_id,
                TypeRelationObligationDiagnostic::UnassignableType,
                infer,
            );
            return Ok(());
        }

        // report immediately when the relation is fully concrete
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
            return Ok(());
        }

        match failure_mode {
            // hard failure paths return the diagnostic
            UnassignableRelationFailureMode::PropagateError => {
                if let Some(error) = self.unassignable_type_error_for_types(
                    module,
                    profile,
                    node_id,
                    target_type_id,
                    source_type_id,
                    types,
                ) {
                    return Err(error);
                }
            }
            // soft failure paths emit the diagnostic and continue
            UnassignableRelationFailureMode::ReportAndContinue => {
                self.emit_unassignable_type_for_types(
                    module,
                    profile,
                    node_id,
                    target_type_id,
                    source_type_id,
                    types,
                );
            }
        }

        Ok(())
    }

    /// Report post solve type relation obligations after infer convergence.
    pub(crate) fn report_type_relation_obligation_errors_after_infer_convergence(
        &self,
        module: &Module,
        profile: ProfileId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        // collect deferred relation obligations from infer state
        let obligations = infer.take_type_relation_obligations();

        // replay each relation obligation with converged operand types
        for obligation in obligations {
            // skip obligations that belong to other modules
            if obligation.source_node_id.module_id != module.id {
                continue;
            }

            // resolve effective operands from captured facts or expression ids
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
