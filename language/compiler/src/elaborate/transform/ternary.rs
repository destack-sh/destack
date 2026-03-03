use destack_dir::{Expression, IfCondition, IfKind, LocalNodeId, NodeTree, SymbolTable};

use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Transform simple if-else expressions to ternary expressions.
    /// Only transforms if both branches are simple (non-block) expressions.
    pub(super) fn transform_if_to_ternary(
        &self,
        tree: &mut NodeTree,
        symbols: &SymbolTable,
    ) -> ElaborateResult<()> {
        let if_ids: Vec<_> = tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| self.is_node_active(tree, symbols, id.into_any()))
            .filter(|id| {
                matches!(
                    tree.get(*id),
                    Expression::If {
                        kind: IfKind::If,
                        ..
                    }
                )
            })
            .collect();

        for if_id in if_ids {
            let Expression::If {
                kind: IfKind::If,
                condition,
                then_expression,
                else_expression: Some(else_expr),
            } = tree.get(if_id).clone()
            else {
                continue;
            };

            if matches!(condition, IfCondition::Let { .. }) {
                continue;
            }

            // check if both branches are simple (non-block, non-if)
            if !self.is_simple_expression(then_expression, tree) {
                continue;
            }
            if !self.is_simple_expression(else_expr, tree) {
                continue;
            }

            // transform to ternary
            tree.replace(
                if_id,
                Expression::If {
                    kind: IfKind::Ternary,
                    condition,
                    then_expression,
                    else_expression: Some(else_expr),
                },
            );
        }

        Ok(())
    }

    /// Check if an expression is "simple" enough for ternary optimization.
    /// Returns true for literals, references, and simple expressions.
    /// Returns false for blocks, if statements, match, loops, etc.
    fn is_simple_expression(&self, expr_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
        match tree.get(expr_id) {
            // simple expressions
            Expression::ScalarLiteral { .. }
            | Expression::TypeLiteral { .. }
            | Expression::LocalReference { .. }
            | Expression::ModuleReference { .. }
            | Expression::GlobalReference { .. }
            | Expression::Unary { .. }
            | Expression::Binary { .. }
            | Expression::Member { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::ArrayExpression { .. }
            | Expression::TupleExpression { .. }
            | Expression::ObjectExpression { .. }
            | Expression::TaggedScalarExpression { .. }
            | Expression::TaggedTupleExpression { .. }
            | Expression::TaggedObjectExpression { .. }
            | Expression::Parenthesized { .. }
            | Expression::TemplateExpression { .. } => true,

            // ternary is ok if nested ternaries are ok
            Expression::If {
                kind: IfKind::Ternary,
                ..
            } => true,

            // not simple
            Expression::Block { .. }
            | Expression::If {
                kind: IfKind::If, ..
            }
            | Expression::Match { .. }
            | Expression::Loop { .. }
            | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Try { .. }
            | Expression::Statement { .. }
            | Expression::Let { .. }
            | Expression::Using { .. } => false,

            // other expressions: be conservative
            _ => false,
        }
    }
}
