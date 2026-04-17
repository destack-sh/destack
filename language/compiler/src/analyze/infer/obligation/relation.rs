use destack_dir::{
    Expression, InferTable, LocalNodeId, LocalNodeIdAny, LocalTypeId, TypeRelationObligation,
    TypeRelationObligationDiagnostic, TypeRelationObligationOperands,
};
use destack_workspace::Module;

use crate::analyze::common::{InferContext, TypeView};
use crate::{AnalyzeResult, Assignability, Compiler};

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
        ctx: TypeView<'_>,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
    ) -> bool {
        // check whether the target depends on solver state
        if self.type_requires_infer_convergence(ctx, target_type_id) {
            return true;
        }

        // check whether the source depends on solver state
        if self.type_requires_infer_convergence(ctx, source_type_id) {
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

    /// Record one post solve relation obligation using a captured target type and one source expression.
    pub(crate) fn push_relation_obligation_for_target_type_and_source_expression(
        &self,
        module: &Module,
        node_id: LocalNodeIdAny,
        target_type_id: LocalTypeId,
        source_expression_id: LocalNodeId<Expression>,
        diagnostic: TypeRelationObligationDiagnostic,
        infer: &mut InferTable,
    ) {
        infer.push_type_relation_obligation(TypeRelationObligation {
            source_node_id: node_id.into_global(module.id),
            operands: TypeRelationObligationOperands::CapturedTargetTypeAndSourceExpression {
                target_type_id,
                source_expression_id: source_expression_id.into_global_any(module.id),
            },
            diagnostic,
        });
    }

    /// Enforce one assignability relation or defer its diagnostic to post solve reporting.
    pub(crate) fn enforce_assignability_or_defer_diagnostic(
        &self,
        ctx: &mut InferContext<'_>,
        node_id: LocalNodeIdAny,
        target_type_id: LocalTypeId,
        source_type_id: LocalTypeId,
        failure_mode: UnassignableRelationFailureMode,
    ) -> AnalyzeResult<()> {
        // defer relation diagnostics until all inference variables are solved
        if self.type_relation_requires_infer_convergence(
            ctx.type_view(),
            target_type_id,
            source_type_id,
        ) {
            self.push_relation_obligation_for_captured_types(
                ctx.module,
                node_id,
                target_type_id,
                source_type_id,
                TypeRelationObligationDiagnostic::UnassignableType,
                ctx.infer,
            );
            return Ok(());
        }

        // report immediately when the relation is fully concrete
        let assignability = self.is_type_assignable(
            &mut ctx.type_context_reborrow(),
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
                    ctx.module_type_view(),
                    node_id,
                    target_type_id,
                    source_type_id,
                ) {
                    return Err(error);
                }
            }
            // soft failure paths emit the diagnostic and continue
            UnassignableRelationFailureMode::ReportAndContinue => {
                self.emit_unassignable_type_for_types(
                    ctx.module_type_view(),
                    node_id,
                    target_type_id,
                    source_type_id,
                );
            }
        }

        Ok(())
    }
}
