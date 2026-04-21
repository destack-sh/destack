use destack_dir as dir;
use dir::{Expression, IfCondition, IfKind, LocalNodeId};

use crate::elaborate::common::ElaborateState;
use crate::{Compiler, ElaborateResult};

impl Compiler {
    /// Transform simple if-else expressions to ternary expressions.
    /// Only transforms if both branches are simple (non-block) expressions.
    pub(super) fn transform_if_to_ternary(
        &self,
        state: &mut ElaborateState<'_>,
    ) -> ElaborateResult<()> {
        let if_ids: Vec<_> = state
            .tree
            .iter_node_ids_of_type::<Expression>()
            .into_iter()
            .filter(|id| self.is_active_in_state(state, id.into_any()))
            .filter(|id| {
                matches!(
                    state.tree.get(*id),
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
            } = state.tree.get(if_id).clone()
            else {
                continue;
            };

            if matches!(condition, IfCondition::Let { .. }) {
                continue;
            }

            // check if both branches are simple (non-block, non-if)
            if !self.is_simple_expression(state, then_expression) {
                continue;
            }
            if !self.is_simple_expression(state, else_expr) {
                continue;
            }

            // transform to ternary
            state.tree.replace(
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
    fn is_simple_expression(
        &self,
        state: &ElaborateState<'_>,
        expr_id: LocalNodeId<Expression>,
    ) -> bool {
        match state.tree.get(expr_id) {
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
            Expression::Block(..)
            | Expression::If {
                kind: IfKind::If, ..
            }
            | Expression::Match { .. }
            | Expression::Loop { .. }
            | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Try { .. }
            | Expression::Let { .. }
            | Expression::LetElse { .. }
            | Expression::Using { .. } => false,

            // other expressions: be conservative
            _ => false,
        }
    }
}
