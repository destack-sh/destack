use destack_dir::{
    Expression, InferTable, LocalNodeId, LocalNodeIdAny, LocalTypeId, SymbolTable,
    TypeRelationObligation, TypeRelationObligationDiagnostic, TypeRelationObligationOperands,
    TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::analyze::common::InferTablesContext;
use crate::{AnalyzeOptions, AnalyzeResult, Assignability, Compiler};

/// Policy for immediate unassignable diagnostics before solve convergence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnassignableRelationFailureMode {
    /// Propagate the unassignable relation as an immediate error result.
    PropagateError,
    /// Report the diagnostic and continue inference.
    ReportAndContinue,
}

#[allow(clippy::too_many_arguments)]
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
    pub(crate) fn push_relation_obligation_for_captured_types(
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
    pub(crate) fn push_relation_obligation_for_expression_operands(
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
    pub(crate) fn enforce_assignability_or_defer_diagnostic(
        &self,
        tables: &mut InferTablesContext<'_>,
        node_id: LocalNodeIdAny,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
        _options: &AnalyzeOptions,
        failure_mode: UnassignableRelationFailureMode,
    ) -> AnalyzeResult<()> {
        // defer relation diagnostics until all inference variables are solved
        if self.type_relation_requires_infer_convergence(
            tables.module,
            tables.profile,
            target_type_id,
            source_type_id,
            tables.symbols,
            tables.types,
        ) {
            self.push_relation_obligation_for_captured_types(
                tables.module,
                node_id,
                target_type_id,
                source_type_id,
                TypeRelationObligationDiagnostic::UnassignableType,
                tables.infer,
            );
            return Ok(());
        }

        // report immediately when the relation is fully concrete
        let assignability = self.is_type_assignable(
            &mut tables.type_tables_reborrow(),
            target_type_id,
            source_type_id,
        );
        if assignability != Assignability::NotAssignable {
            return Ok(());
        }

        match failure_mode {
            // hard failure paths return the diagnostic
            UnassignableRelationFailureMode::PropagateError => {
                if let Some(error) = self.unassignable_type_error_for_types(
                    tables.module,
                    tables.profile,
                    node_id,
                    target_type_id,
                    source_type_id,
                    tables.types,
                ) {
                    return Err(error);
                }
            }
            // soft failure paths emit the diagnostic and continue
            UnassignableRelationFailureMode::ReportAndContinue => {
                self.emit_unassignable_type_for_types(
                    tables.module,
                    tables.profile,
                    node_id,
                    target_type_id,
                    source_type_id,
                    tables.types,
                );
            }
        }

        Ok(())
    }
}
