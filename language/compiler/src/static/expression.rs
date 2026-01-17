use crate::{Compiler, evaluate_binary_scalar, evaluate_unary_scalar};
use destack_dir::{Expression, LocalNodeId, NodeTree, StaticExpression};

impl Compiler {
    /// Evaluate an expression into a static value expression.
    pub(crate) fn evaluate_static_expression_value(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<StaticExpression> {
        // read the expression node
        let expression = tree.get(expression_id);

        // resolve supported expression shapes
        match expression {
            Expression::ScalarLiteral { value } => Some(StaticExpression::ScalarLiteral {
                value: value.clone(),
            }),
            Expression::TypeLiteral { value } => Some(StaticExpression::TypeLiteral {
                value: value.clone(),
            }),
            Expression::Type { value } => Some(StaticExpression::Type { ty: *value }),
            Expression::Parenthesized { expression } => {
                // unwrap parenthesized expressions
                self.evaluate_static_expression_value(*expression, tree)
            }
            Expression::Unary { operator, right } => {
                // evaluate the operand
                let right_value = self.evaluate_static_expression_value(*right, tree)?;
                let StaticExpression::ScalarLiteral { value } = right_value else {
                    return None;
                };

                // apply the unary operator
                let value = evaluate_unary_scalar(*operator, &value)?;
                Some(StaticExpression::ScalarLiteral { value })
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                // evaluate both operands
                let left_value = self.evaluate_static_expression_value(*left, tree)?;
                let right_value = self.evaluate_static_expression_value(*right, tree)?;
                let StaticExpression::ScalarLiteral { value: left_value } = left_value else {
                    return None;
                };
                let StaticExpression::ScalarLiteral { value: right_value } = right_value else {
                    return None;
                };

                // apply the binary operator
                let value = evaluate_binary_scalar(*operator, &left_value, &right_value)?;
                Some(StaticExpression::ScalarLiteral { value })
            }
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                // evaluate range bounds
                let start_value = self.evaluate_static_expression_value(*start, tree)?;
                let end_value = self.evaluate_static_expression_value(*end, tree)?;

                // build the range literal
                Some(StaticExpression::RangeExpression {
                    start: Box::new(start_value),
                    end: Box::new(end_value),
                    is_inclusive: *is_inclusive,
                })
            }
            Expression::ArrayExpression { elements } => {
                // collect element values
                let mut values = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value(element.value(), tree)?;
                    values.push(value);
                }
                Some(StaticExpression::ArrayExpression { elements: values })
            }
            Expression::TupleExpression { elements } => {
                // collect element values
                let mut values = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value(element.value(), tree)?;
                    values.push(value);
                }
                Some(StaticExpression::TupleExpression { elements: values })
            }
            _ => None,
        }
    }
}
