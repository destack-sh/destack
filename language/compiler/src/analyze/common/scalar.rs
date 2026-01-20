use destack_dir::{BinaryOperator, ScalarLiteral, UnaryOperator};

/// Evaluate a unary operator on scalar literals.
pub(crate) fn evaluate_unary_scalar(
    operator: UnaryOperator,
    right: &ScalarLiteral,
) -> Option<ScalarLiteral> {
    // dispatch unary operator behavior
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
pub(crate) fn evaluate_binary_scalar(
    operator: BinaryOperator,
    left: &ScalarLiteral,
    right: &ScalarLiteral,
) -> Option<ScalarLiteral> {
    // dispatch binary operator behavior
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
    // dispatch numeric literal combinations
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

/// Evaluate a binary operator on integer like literals.
fn evaluate_integer_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(i64, i64) -> i64,
) -> Option<ScalarLiteral> {
    // dispatch integer like literal combinations
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

/// Evaluate a shift operator on integer like literals.
fn evaluate_shift_binary(
    left: &ScalarLiteral,
    right: &ScalarLiteral,
    op: fn(i64, u32) -> i64,
) -> Option<ScalarLiteral> {
    // resolve the shift amount
    let shift = match right {
        ScalarLiteral::Integer(value) => (*value).try_into().ok(),
        ScalarLiteral::Bigint(value) => (*value).try_into().ok(),
        _ => None,
    }?;

    // apply the shift to integer like values
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
    // normalize operands to f64 for comparisons
    let (left, right) = match (left, right) {
        (ScalarLiteral::Integer(a), ScalarLiteral::Integer(b)) => (*a as f64, *b as f64),
        (ScalarLiteral::Bigint(a), ScalarLiteral::Bigint(b)) => (*a as f64, *b as f64),
        (ScalarLiteral::Float(a), ScalarLiteral::Float(b)) => (*a, *b),
        (ScalarLiteral::Integer(a), ScalarLiteral::Float(b)) => (*a as f64, *b),
        (ScalarLiteral::Float(a), ScalarLiteral::Integer(b)) => (*a, *b as f64),
        _ => return None,
    };

    // evaluate the comparison
    Some(ScalarLiteral::Boolean(op(left, right)))
}
