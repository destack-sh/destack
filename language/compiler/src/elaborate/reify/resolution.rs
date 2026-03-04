use destack_dir as dir;
use dir::{
    Expression, IfCondition, IfKind, LocalNodeId, LocalTypeId, NodeType, Resolution,
    ResolutionCandidate,
};

use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Reify dynamic resolutions into explicit type checks.
    ///
    /// Transforms `Resolution::Dynamic` into if-else chains with `is` type checks
    /// and `Resolution::Static` branches. After this pass, only `Builtin` and
    /// `Static` resolutions remain in the DIR.
    pub(super) fn reify_resolution(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> ElaborateResult<()> {
        // get the resolution for this expression
        let Some(resolution_id) = state
            .types
            .get_resolution_for_node(expression_id.into_global_any(state.ctx.module_id))
        else {
            return Ok(());
        };
        let resolution = state.types.get_resolution(resolution_id).clone();

        // only transform Dynamic resolutions
        let Resolution::Dynamic {
            receiver,
            candidates,
        } = resolution
        else {
            return Ok(());
        };

        // need at least 2 candidates to split
        if candidates.len() < 2 {
            return Ok(());
        }

        // need a receiver type to dispatch on
        let Some(_receiver_type_id) = receiver else {
            return Ok(());
        };

        // get the receiver expression from the original expression
        let expression = state.tree.get(expression_id).clone();
        let Some(receiver_expression_id) = self.receiver_expression_for(state, &expression) else {
            return Ok(());
        };

        // get the scope for creating synthetic nodes
        let scope = state.tree.get_scope(expression_id);

        // build the if-else chain from bottom up
        // start with the last candidate as the final else branch
        let mut else_branch = self.build_static_resolution_branch(
            state,
            expression_id,
            &expression,
            &candidates[candidates.len() - 1],
            scope,
        )?;

        // build if-else chain for remaining candidates (in reverse order)
        for candidate in candidates.iter().rev().skip(1) {
            // get the type to check against from the dispatch key
            let Some(check_type_id) = self.type_for_dispatch_candidate(candidate) else {
                continue;
            };

            // build: if (receiver is CheckType) { staticBranch } else { elseBranch }
            let then_branch = self.build_static_resolution_branch(
                state,
                expression_id,
                &expression,
                candidate,
                scope,
            )?;

            // build the is check: receiver is Type
            let is_check = self.build_is_type_check(
                state,
                expression_id,
                receiver_expression_id,
                check_type_id,
                scope,
            )?;

            // build the if expression
            let if_id = state.tree.reserve_from(
                NodeType::Expression,
                expression_id.into_any(),
                scope,
                None,
            );
            else_branch = state.tree.insert(
                if_id,
                Expression::If {
                    kind: IfKind::If,
                    condition: IfCondition::Expression {
                        condition: is_check,
                    },
                    then_expression: then_branch,
                    else_expression: Some(else_branch),
                },
            );

            // set the type for the if expression (same as original expression)
            if let Some(expr_type_id) = state.types.get_declared_or_inferred_type_id(
                expression_id.into_global_any(state.ctx.module_id),
            ) {
                state
                    .types
                    .set_inferred_type(if_id.into_global(state.ctx.module_id), expr_type_id);
            }
        }

        // replace the original expression with the if-else chain
        let final_expression = state.tree.get(else_branch).clone();
        state.tree.replace(expression_id, final_expression);

        Ok(())
    }

    /// Get the receiver expression from an expression that has a resolution.
    fn receiver_expression_for(
        &self,
        state: &ElaborateState<'_>,
        expression: &Expression,
    ) -> Option<LocalNodeId<Expression>> {
        match expression {
            // binary operators: receiver is the left operand
            Expression::Binary { left, .. } => Some(*left),
            // unary operators: receiver is the operand
            Expression::Unary { right, .. } => Some(*right),
            // call expressions: receiver is from the left (could be member access)
            Expression::Call { left, .. } => {
                // unwrap parenthesized callee
                let mut callee_id = *left;
                loop {
                    let Expression::Parenthesized { expression } = state.tree.get(callee_id) else {
                        break;
                    };
                    callee_id = *expression;
                }

                // use the receiver for member calls
                if let Expression::Member { left, .. } = state.tree.get(callee_id) {
                    Some(*left)
                } else {
                    Some(callee_id)
                }
            }
            // member access: receiver is the left
            Expression::Member { left, .. } => Some(*left),
            // index access: receiver is the left
            Expression::Index { left, .. } => Some(*left),
            _ => None,
        }
    }

    /// Get the type to check from a resolution candidate's dispatch key.
    fn type_for_dispatch_candidate(&self, candidate: &ResolutionCandidate) -> Option<LocalTypeId> {
        // the dispatch key contains the type(s) for runtime selection
        let key = candidate.key.as_ref()?;

        // for single dispatch, use the single type
        // for multiple dispatch, we'd need more complex logic
        let types = key.types();
        types.first().copied()
    }

    /// Build a branch with static resolution for a candidate.
    fn build_static_resolution_branch(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        expression: &Expression,
        candidate: &ResolutionCandidate,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // clone the original expression
        let cloned_id = self.clone_expression_with_analysis(state, origin_id, expression, scope);

        // get the receiver type for the static resolution
        let receiver_type = self.type_for_dispatch_candidate(candidate);

        // create a static resolution for this branch
        let static_resolution = Resolution::Static {
            receiver: receiver_type,
            candidate: candidate.clone(),
        };
        let resolution_id = state.types.insert_resolution(static_resolution);
        state.types.set_resolution_for_node(
            cloned_id.into_global_any(state.ctx.module_id),
            resolution_id,
        );

        // reify overloaded operators inside each static branch clone
        self.reify_operator_expression(state, cloned_id)?;

        // reify branch local call argument casts after static dispatch split
        let cloned_expression = state.tree.get(cloned_id).clone();
        if let Expression::Call {
            left,
            static_arguments,
            dynamic_arguments,
        } = cloned_expression
        {
            let dynamic_arguments = self.clone_arguments_with_analysis(
                state,
                cloned_id,
                &dynamic_arguments,
                state.tree.get_scope(cloned_id),
            );
            state.tree.replace(
                cloned_id,
                Expression::Call {
                    left,
                    static_arguments,
                    dynamic_arguments: dynamic_arguments.clone(),
                },
            );
            self.reify_implicit_casts_in_call(state, cloned_id, &dynamic_arguments)?;
        }

        Ok(cloned_id)
    }

    /// Build an `is` type check expression: `value is Type`.
    fn build_is_type_check(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        check_type_id: LocalTypeId,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // create the type expression for the right side of `is`
        let type_expr_id =
            self.insert_type_expression_for_type_id(state, origin_id, check_type_id, scope);

        // build `value is Type`
        let is_check =
            self.insert_is_type_check_expression(state, origin_id, value_id, type_expr_id, scope);

        Ok(is_check)
    }
}
