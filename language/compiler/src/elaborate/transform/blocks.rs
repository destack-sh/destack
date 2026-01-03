use destack_dir::{Expression, LocalNodeId, NodeTree};

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Unwrap single-expression blocks in if/else branches.
    /// This enables ternary optimization for `if (cond) { a } else { b }`.
    pub(super) fn unwrap_single_expression_blocks(
        &self,
        tree: &mut NodeTree,
    ) -> ElaborateResult<()> {
        let if_ids: Vec<_> = tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| matches!(tree.get(*id), Expression::If { .. }))
            .collect();

        for if_id in if_ids {
            let Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
            } = tree.get(if_id).clone()
            else {
                continue;
            };

            // try to unwrap then_expression if it's a single-expression block
            let new_then = self.try_unwrap_block(then_expression, tree);

            // try to unwrap else_expression if it's a single-expression block
            let new_else = else_expression.map(|e| self.try_unwrap_block(e, tree));

            // only update if something changed
            if new_then != then_expression || new_else != else_expression {
                tree.replace(
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
    pub(super) fn transform_drop_parenthesized(&self, tree: &mut NodeTree) -> ElaborateResult<()> {
        let expression_ids: Vec<_> = tree.iter_node_ids_of_type::<Expression>();

        for expression_id in expression_ids {
            let Expression::Parenthesized { expression } = tree.get(expression_id).clone() else {
                continue;
            };

            let inner = tree.get(expression).clone();
            tree.replace(expression_id, inner);
        }

        Ok(())
    }

    /// Try to unwrap a single-expression block to its inner expression.
    /// Returns the inner expression if the block contains exactly one expression
    /// that is not a statement. Otherwise returns the original expression.
    fn try_unwrap_block(
        &self,
        expr_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> LocalNodeId<Expression> {
        let Expression::Block { block } = tree.get(expr_id) else {
            return expr_id;
        };

        let block_node = tree.get(*block);
        if block_node.expressions.len() != 1 {
            return expr_id;
        }

        let inner_expr_id = block_node.expressions[0];
        let inner_expr = tree.get(inner_expr_id);

        // don't unwrap if the inner expression is a statement (has side effects)
        // or if it's a let/declaration
        match inner_expr {
            Expression::Statement { .. } | Expression::Let { .. } | Expression::Using { .. } => {
                expr_id
            }
            _ => inner_expr_id,
        }
    }
}
