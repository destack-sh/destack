use destack_dir as dir;

use crate::ConstValue;

/// Resolve the target symbol for a reference expression.
pub fn expression_target_symbol(
    tree: &dir::NodeTree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<dir::GlobalSymbolId> {
    // unwrap parenthesized expressions first
    let expression = tree.get(expression_id);
    if let dir::Expression::Parenthesized { expression } = expression {
        return expression_target_symbol(tree, *expression);
    }

    // return the reference target symbol when present
    expression.target_symbol()
}

/// Convert a constant value into an i64 when possible.
pub fn const_i64(value: &ConstValue) -> Option<i64> {
    match value {
        ConstValue::Integer(value) => Some(*value),
        ConstValue::Bigint(value) => Some(*value),
        ConstValue::Float(value) => {
            if value.is_finite() && value.fract() == 0.0 {
                Some(*value as i64)
            } else {
                None
            }
        }
        ConstValue::Boolean(value) => Some(i64::from(*value)),
        ConstValue::Null | ConstValue::Undefined => None,
    }
}

/// Flip a comparison operator when the operands are swapped.
pub fn flip_operator(operator: dir::BinaryOperator) -> Option<dir::BinaryOperator> {
    match operator {
        dir::BinaryOperator::GreaterThan => Some(dir::BinaryOperator::LessThan),
        dir::BinaryOperator::GreaterThanOrEqual => Some(dir::BinaryOperator::LessThanOrEqual),
        dir::BinaryOperator::LessThan => Some(dir::BinaryOperator::GreaterThan),
        dir::BinaryOperator::LessThanOrEqual => Some(dir::BinaryOperator::GreaterThanOrEqual),
        dir::BinaryOperator::Equal
        | dir::BinaryOperator::EqualStrict
        | dir::BinaryOperator::NotEqual
        | dir::BinaryOperator::NotEqualStrict => Some(operator),
        _ => None,
    }
}
