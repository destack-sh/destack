use crate::Compiler;
use destack_dir::{
    BinaryOperator, Expression, LocalNodeId, NodeTree, ScalarLiteral, StaticExpression,
    UnaryOperator,
};

impl Compiler {
    /// Evaluate an expression into a static value expression.
    pub(crate) fn evaluate_static_expression_value(
        &self,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
    ) -> Option<StaticExpression> {
        let expression = tree.get(expression_id);
        match expression {
            Expression::ScalarLiteral { value } => Some(StaticExpression::ScalarLiteral {
                value: value.clone(),
            }),
            Expression::TypeLiteral { value } => Some(StaticExpression::TypeLiteral {
                value: value.clone(),
            }),
            Expression::Type { value } => Some(StaticExpression::Type { ty: *value }),
            Expression::Parenthesized { expression } => {
                self.evaluate_static_expression_value(*expression, tree)
            }
            Expression::Unary { operator, right } => {
                let right_value = self.evaluate_static_expression_value(*right, tree)?;
                let StaticExpression::ScalarLiteral { value } = right_value else {
                    return None;
                };
                let value = evaluate_unary_scalar(*operator, &value)?;
                Some(StaticExpression::ScalarLiteral { value })
            }
            Expression::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.evaluate_static_expression_value(*left, tree)?;
                let right_value = self.evaluate_static_expression_value(*right, tree)?;
                let StaticExpression::ScalarLiteral { value: left_value } = left_value else {
                    return None;
                };
                let StaticExpression::ScalarLiteral { value: right_value } = right_value else {
                    return None;
                };
                let value = evaluate_binary_scalar(*operator, &left_value, &right_value)?;
                Some(StaticExpression::ScalarLiteral { value })
            }
            Expression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                let start_value = self.evaluate_static_expression_value(*start, tree)?;
                let end_value = self.evaluate_static_expression_value(*end, tree)?;
                Some(StaticExpression::RangeExpression {
                    start: Box::new(start_value),
                    end: Box::new(end_value),
                    is_inclusive: *is_inclusive,
                })
            }
            Expression::ArrayExpression { elements } => {
                let mut values = Vec::with_capacity(elements.len());
                for element_id in elements {
                    let element = tree.get(*element_id);
                    let value = self.evaluate_static_expression_value(element.value(), tree)?;
                    values.push(value);
                }
                Some(StaticExpression::ArrayExpression { elements: values })
            }
            Expression::TupleExpression { elements } => {
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

/// Evaluate a unary operator on scalar literals.
fn evaluate_unary_scalar(operator: UnaryOperator, right: &ScalarLiteral) -> Option<ScalarLiteral> {
    match operator {
        UnaryOperator::Not => match right {
            ScalarLiteral::Boolean(value) => Some(ScalarLiteral::Boolean(!value)),
            _ => None,
        },
        UnaryOperator::Plus => Some(right.clone()),
        UnaryOperator::Negate => match right {
            ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(-value)),
            ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(-value)),
            ScalarLiteral::Float(value) => Some(ScalarLiteral::Float(-value)),
            _ => None,
        },
        UnaryOperator::WrappingNegate => match right {
            ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(value.wrapping_neg())),
            ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(value.wrapping_neg())),
            _ => None,
        },
        UnaryOperator::ElementwiseNot => match right {
            ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(!value)),
            ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(!value)),
            _ => None,
        },
        _ => None,
    }
}

/// Evaluate a binary operator on scalar literals.
fn evaluate_binary_scalar(
    operator: BinaryOperator,
    left: &ScalarLiteral,
    right: &ScalarLiteral,
) -> Option<ScalarLiteral> {
    match operator {
        BinaryOperator::Add => evaluate_numeric_binary(left, right, |a, b| a + b, |a, b| a + b),
        BinaryOperator::Subtract => {
            evaluate_numeric_binary(left, right, |a, b| a - b, |a, b| a - b)
        }
        BinaryOperator::Multiply => {
            evaluate_numeric_binary(left, right, |a, b| a * b, |a, b| a * b)
        }
        BinaryOperator::Divide => evaluate_numeric_binary(left, right, |a, b| a / b, |a, b| a / b),
        BinaryOperator::Remainder => {
            evaluate_numeric_binary(left, right, |a, b| a % b, |a, b| a % b)
        }
        BinaryOperator::WrappingAdd => evaluate_integer_binary(left, right, i64::wrapping_add),
        BinaryOperator::WrappingSubtract => evaluate_integer_binary(left, right, i64::wrapping_sub),
        BinaryOperator::WrappingMultiply => evaluate_integer_binary(left, right, i64::wrapping_mul),
        BinaryOperator::SaturatingAdd => evaluate_integer_binary(left, right, i64::saturating_add),
        BinaryOperator::SaturatingSubtract => {
            evaluate_integer_binary(left, right, i64::saturating_sub)
        }
        BinaryOperator::SaturatingMultiply => {
            evaluate_integer_binary(left, right, i64::saturating_mul)
        }
        BinaryOperator::ShiftLeft => evaluate_shift_binary(left, right, |a, b| a << b),
        BinaryOperator::ShiftRight => evaluate_shift_binary(left, right, |a, b| a >> b),
        BinaryOperator::ElementwiseAnd => evaluate_integer_binary(left, right, |a, b| a & b),
        BinaryOperator::ElementwiseOr => evaluate_integer_binary(left, right, |a, b| a | b),
        BinaryOperator::ElementwiseXor => evaluate_integer_binary(left, right, |a, b| a ^ b),
        BinaryOperator::Equal | BinaryOperator::EqualStrict => {
            Some(ScalarLiteral::Boolean(left == right))
        }
        BinaryOperator::NotEqual | BinaryOperator::NotEqualStrict => {
            Some(ScalarLiteral::Boolean(left != right))
        }
        BinaryOperator::LessThan => evaluate_compare_binary(left, right, |a, b| a < b),
        BinaryOperator::LessThanOrEqual => evaluate_compare_binary(left, right, |a, b| a <= b),
        BinaryOperator::GreaterThan => evaluate_compare_binary(left, right, |a, b| a > b),
        BinaryOperator::GreaterThanOrEqual => evaluate_compare_binary(left, right, |a, b| a >= b),
        BinaryOperator::And => match (left, right) {
            (ScalarLiteral::Boolean(a), ScalarLiteral::Boolean(b)) => {
                Some(ScalarLiteral::Boolean(*a && *b))
            }
            _ => None,
        },
        BinaryOperator::Or => match (left, right) {
            (ScalarLiteral::Boolean(a), ScalarLiteral::Boolean(b)) => {
                Some(ScalarLiteral::Boolean(*a || *b))
            }
            _ => None,
        },
        _ => None,
    }
}

/// Evaluate a numeric binary operator across integer and float literals.
fn evaluate_numeric_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
) -> Option<ScalarLiteral> {
    match (left, right) {
        (ScalarLiteral::Integer(a), ScalarLiteral::Integer(b)) => {
            Some(ScalarLiteral::Integer(int_op(*a, *b)))
        }
        (ScalarLiteral::Bigint(a), ScalarLiteral::Bigint(b)) => {
            Some(ScalarLiteral::Bigint(int_op(*a, *b)))
        }
        (ScalarLiteral::Float(a), ScalarLiteral::Float(b)) => {
            Some(ScalarLiteral::Float(float_op(*a, *b)))
        }
        (ScalarLiteral::Integer(a), ScalarLiteral::Float(b)) => {
            Some(ScalarLiteral::Float(float_op(*a as f64, *b)))
        }
        (ScalarLiteral::Float(a), ScalarLiteral::Integer(b)) => {
            Some(ScalarLiteral::Float(float_op(*a, *b as f64)))
        }
        _ => None,
    }
}

/// Evaluate a binary operator on integer-like literals.
fn evaluate_integer_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(i64, i64) -> i64,
) -> Option<ScalarLiteral> {
    match (left, right) {
        (ScalarLiteral::Integer(a), ScalarLiteral::Integer(b)) => {
            Some(ScalarLiteral::Integer(op(*a, *b)))
        }
        (ScalarLiteral::Bigint(a), ScalarLiteral::Bigint(b)) => {
            Some(ScalarLiteral::Bigint(op(*a, *b)))
        }
        _ => None,
    }
}

/// Evaluate a shift operator on integer-like literals.
fn evaluate_shift_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(i64, u32) -> i64,
) -> Option<ScalarLiteral> {
    let shift = match right {
        ScalarLiteral::Integer(value) => (*value).try_into().ok(),
        ScalarLiteral::Bigint(value) => (*value).try_into().ok(),
        _ => None,
    }?;

    match left {
        ScalarLiteral::Integer(value) => Some(ScalarLiteral::Integer(op(*value, shift))),
        ScalarLiteral::Bigint(value) => Some(ScalarLiteral::Bigint(op(*value, shift))),
        _ => None,
    }
}

/// Evaluate a comparison operator on scalar numeric literals.
fn evaluate_compare_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(f64, f64) -> bool,
) -> Option<ScalarLiteral> {
    let (left, right) = match (left, right) {
        (ScalarLiteral::Integer(a), ScalarLiteral::Integer(b)) => (*a as f64, *b as f64),
        (ScalarLiteral::Bigint(a), ScalarLiteral::Bigint(b)) => (*a as f64, *b as f64),
        (ScalarLiteral::Float(a), ScalarLiteral::Float(b)) => (*a, *b),
        (ScalarLiteral::Integer(a), ScalarLiteral::Float(b)) => (*a as f64, *b),
        (ScalarLiteral::Float(a), ScalarLiteral::Integer(b)) => (*a, *b as f64),
        _ => return None,
    };

    Some(ScalarLiteral::Boolean(op(left, right)))
}
