use destack_dir as dir;
use dir::{Expression, LocalNodeId};

use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Unwrap single-expression blocks in if/else branches.
    /// This enables ternary optimization for `if (cond) { a } else { b }`.
    pub(super) fn unwrap_single_expression_blocks(
        &self,
        state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        let if_ids: Vec<_> = state
            .tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| self.is_active_in_state(state, id.into_any()))
            .filter(|id| matches!(state.tree.get(*id), Expression::If { .. }))
            .collect();

        for if_id in if_ids {
            let Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } = state.tree.get(if_id).clone()
            else {
                continue;
            };

            // try to unwrap then_expression if it's a single-expression block
            let new_then = self.try_unwrap_block(state, then_expression);

            // try to unwrap else_expression if it's a single-expression block
            let new_else = else_expression.map(|e| self.try_unwrap_block(state, e));

            // only update if something changed
            if new_then != then_expression || new_else != else_expression {
                state.tree.replace(
                    if_id,
                    Expression::If {
                        kind,
                        condition,
                        then_expression: new_then,
                        else_expression: new_else,
                    },
                );
            }
        }

        Ok(())
    }

    /// Drop parenthesized expressions from canonical DIR.
    pub(super) fn transform_drop_parenthesized(
        &self,
        state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        let expression_ids: Vec<_> = state.tree.iter_node_ids_of_type::<Expression>();
        for expression_id in expression_ids {
            // skip inactive expressions
            if !self.is_active_in_state(state, expression_id.into_any()) {
                continue;
            }

            let Expression::Parenthesized { expression } = state.tree.get(expression_id).clone()
            else {
                continue;
            };

            state.tree.replace_from(expression_id, expression);
        }

        Ok(())
    }

    /// Try to unwrap a single tail-expression block to its inner expression.
    fn try_unwrap_block(
        &self,
        state: &ElaborateState<'_>,
        expr_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let Expression::Block(block) = state.tree.get(expr_id) else {
            return expr_id;
        };

        let block_node = state.tree.get(*block);
        if !block_node.leading_expressions.is_empty() {
            return expr_id;
        }

        let Some(inner_expr_id) = block_node.tail_expression else {
            return expr_id;
        };
        let inner_expr = state.tree.get(inner_expr_id);

        // don't unwrap bindings from block position
        match inner_expr {
            Expression::Let { .. } | Expression::LetElse { .. } | Expression::Using { .. } => {
                expr_id
            }
            _ => inner_expr_id,
        }
    }
}
