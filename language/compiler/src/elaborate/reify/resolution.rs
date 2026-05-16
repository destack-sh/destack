use destack_dir as dir;
use dir::{
    DispatchResolution, DispatchTarget, Expression, IfCondition, IfForm, LocalNodeId, LocalTypeId,
    NodeType, Resolution,
};

use crate::elaborate::ElaborateState;
use crate::{Compiler, ElaborateResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Clone one dynamic-resolution expression so each synthetic branch owns its own shell.
    fn clone_resolution_expression(
        &self,
        state: &mut ElaborateState<'_>,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
        scope: dir::LocalScope,
    ) -> LocalNodeId<Expression> {
        let cloned_expression = match expression {
            Expression::Member { left, name } => {
                let left_expression = state.tree.get(*left).clone();
                let left = self.clone_resolution_expression(state, *left, &left_expression, scope);

                Expression::Member { left, name: *name }
            }

            Expression::Call {
                left,
                generic_arguments,
                arguments,
            } => {
                let left_expression = state.tree.get(*left).clone();
                let left = self.clone_resolution_expression(state, *left, &left_expression, scope);
                let arguments = arguments
                    .iter()
                    .map(|argument_id| self.clone_resolution_argument(state, *argument_id, scope))
                    .collect();

                Expression::Call {
                    left,
                    generic_arguments: generic_arguments.clone(),
                    arguments,
                }
            }

            Expression::Parenthesized { expression } => {
                let inner_expression = state.tree.get(*expression).clone();
                let expression =
                    self.clone_resolution_expression(state, *expression, &inner_expression, scope);

                Expression::Parenthesized { expression }
            }

            Expression::Unary { operator, right } => {
                let right_expression = state.tree.get(*right).clone();
                let right =
                    self.clone_resolution_expression(state, *right, &right_expression, scope);

                Expression::Unary {
                    operator: *operator,
                    right,
                }
            }

            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_expression = state.tree.get(*left).clone();
                let left = self.clone_resolution_expression(state, *left, &left_expression, scope);
                let right_expression = state.tree.get(*right).clone();
                let right =
                    self.clone_resolution_expression(state, *right, &right_expression, scope);

                Expression::Binary {
                    left,
                    operator: *operator,
                    right,
                }
            }

            Expression::Index { left, right } => {
                let left_expression = state.tree.get(*left).clone();
                let left = self.clone_resolution_expression(state, *left, &left_expression, scope);
                let right = right.map(|right| {
                    let right_expression = state.tree.get(right).clone();
                    self.clone_resolution_expression(state, right, &right_expression, scope)
                });

                Expression::Index { left, right }
            }

            _ => expression.clone(),
        };

        let cloned_id = state.tree.reserve_from(
            NodeType::Expression,
            expression_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Reified),
        );
        let cloned_id = state.tree.insert_as_owner(cloned_id, cloned_expression);
        state.types_tail.copy_node_relations(
            expression_id.into_global_any(state.module_id),
            cloned_id.into_global_any(state.module_id),
        );

        cloned_id
    }

    /// Clone one call argument for a dynamic-resolution branch.
    fn clone_resolution_argument(
        &self,
        state: &mut ElaborateState<'_>,
        argument_id: LocalNodeId<dir::Argument>,
        scope: dir::LocalScope,
    ) -> LocalNodeId<dir::Argument> {
        let argument = state.tree.get(argument_id).clone();
        let value = argument.value();
        let value_expression = state.tree.get(value).clone();
        let value = self.clone_resolution_expression(state, value, &value_expression, scope);

        let cloned_id = state.tree.reserve_from(
            NodeType::Argument,
            argument_id.into_any(),
            scope,
            None,
            Some(dir::ProvenanceReason::Reified),
        );

        let cloned_argument = match argument {
            dir::Argument::Named { name, value: _ } => dir::Argument::Named { name, value },
            dir::Argument::Labeled { label, value: _ } => dir::Argument::Labeled { label, value },
            dir::Argument::Positional { value: _ } => dir::Argument::Positional { value },
            dir::Argument::Spread { label, value: _ } => dir::Argument::Spread { label, value },
            dir::Argument::Error { value: _ } => dir::Argument::Error { value },
        };

        let cloned_id = state.tree.insert_as_owner(cloned_id, cloned_argument);
        state.types_tail.copy_node_relations(
            argument_id.into_global_any(state.module_id),
            cloned_id.into_global_any(state.module_id),
        );

        cloned_id
    }

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
        let Some(resolution) = state.type_table().resolution(expression_id.into_global_any(state.module_id))
            .cloned()
        else {
            return Ok(());
        };

        // only transform Dynamic resolutions
        let Resolution::Dispatch(DispatchResolution::Dynamic {
            receiver,
            targets: candidates,
        }) = resolution
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
            let is_check = self.build_resolution_type_guard(
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
                Some(dir::ProvenanceReason::Reified),
            );
            else_branch = state.tree.insert_as_owner(
                if_id,
                Expression::If {
                    form: IfForm::If,
                    condition: IfCondition::Expression {
                        condition: is_check,
                    },
                    then_expression: then_branch,
                    else_expression: Some(else_branch),
                },
            );

            // set the type for the if expression (same as original expression)
            if let Some(expr_type_id) = state.type_table().get_declared_or_inferred_type_id(expression_id.into_global_any(state.module_id))
            {
                state
            .types_tail
            .set_inferred_type(if_id.into_global(state.module_id), expr_type_id);
            }
        }

        // replace the original expression with the if-else chain
        state.tree.replace_from(expression_id, else_branch);

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
    fn type_for_dispatch_candidate(&self, candidate: &DispatchTarget) -> Option<LocalTypeId> {
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
        candidate: &DispatchTarget,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // clone the original expression
        let cloned_id = self.clone_resolution_expression(state, origin_id, expression, scope);

        // get the receiver type for the static resolution
        let receiver_type = self.type_for_dispatch_candidate(candidate);

        // create a static resolution for this branch
        let static_resolution = Resolution::Dispatch(DispatchResolution::Static {
            receiver: receiver_type,
            target: candidate.clone(),
        });
        state.types_tail.set_resolution(
            cloned_id.into_global_any(state.module_id),
            static_resolution,
        );

        // reify overloaded operators inside each static branch clone
        self.reify_operator_expression(state, cloned_id)?;

        // reify branch local call argument casts after static dispatch split
        let cloned_expression = state.tree.get(cloned_id).clone();
        if let Expression::Call { arguments, .. } = cloned_expression {
            self.reify_implicit_casts_in_call(state, cloned_id, &arguments)?;
        }

        Ok(cloned_id)
    }

    /// Build one runtime type guard: `value is Type`.
    fn build_resolution_type_guard(
        &self,
        state: &mut ElaborateState<'_>,
        origin_id: LocalNodeId<Expression>,
        value_id: LocalNodeId<Expression>,
        check_type_id: LocalTypeId,
        scope: dir::LocalScope,
    ) -> ElaborateResult<LocalNodeId<Expression>> {
        // each generated type guard needs its own receiver node
        let value_expression = state.tree.get(value_id).clone();
        let value_id = self.clone_resolution_expression(state, value_id, &value_expression, scope);

        // create the target type syntax
        let type_expr_id =
            self.insert_type_expression_for_type_id(state, origin_id, check_type_id, scope);

        // build `value is Type`
        let is_check =
            self.insert_is_type_check_expression(state, origin_id, value_id, type_expr_id, scope);

        Ok(is_check)
    }
}
