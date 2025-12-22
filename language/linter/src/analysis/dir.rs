use std::collections::HashMap;

use destack_dir as dir;

use crate::ConstValue;

#[derive(Debug, Default)]
/// Cache shared analysis results for DIR lint rules.
pub struct LintDirAnalysisCache {
    /// Cached constant values for DIR expressions.
    const_values: HashMap<u32, Option<ConstValue>>,
}

impl LintDirAnalysisCache {
    /// Return a cached constant value for an expression.
    pub fn const_value(
        &mut self,
        tree: &dir::NodeTree,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<ConstValue> {
        if let Some(value) = self.const_values.get(&id.id) {
            return *value;
        }

        let value = evaluate_const_value(tree, id);
        self.const_values.insert(id.id, value);
        value
    }
}

/// Evaluate a DIR expression to a constant value when possible.
fn evaluate_const_value(
    tree: &dir::NodeTree,
    id: dir::LocalNodeId<dir::Expression>,
) -> Option<ConstValue> {
    let expression = tree.get(id);
    match expression {
        dir::Expression::ScalarLiteral { value } => match value {
            dir::ScalarLiteral::Boolean(value) => Some(ConstValue::Boolean(*value)),
            dir::ScalarLiteral::Integer(value) => Some(ConstValue::Integer(*value)),
            dir::ScalarLiteral::Bigint(value) => Some(ConstValue::Bigint(*value)),
            dir::ScalarLiteral::Float(value) => Some(ConstValue::Float(*value)),
            _ => None,
        },
        dir::Expression::TypeLiteral {
            value: dir::TypeLiteral::Null,
        } => Some(ConstValue::Null),
        dir::Expression::TypeLiteral {
            value: dir::TypeLiteral::Undefined,
        } => Some(ConstValue::Undefined),
        dir::Expression::Unary { operator, right } => {
            let value = evaluate_const_value(tree, *right)?;
            match operator {
                dir::UnaryOperator::Not => Some(ConstValue::Boolean(!value.to_bool())),
                dir::UnaryOperator::Plus => Some(value),
                dir::UnaryOperator::Negate | dir::UnaryOperator::WrappingNegate => {
                    negate_const_value(value)
                }
                dir::UnaryOperator::ElementwiseNot => bit_not_const_value(value),
                _ => None,
            }
        }
        dir::Expression::Statement { statement } => evaluate_const_value(tree, *statement),
        _ => None,
    }
}

/// Apply unary negation to a constant value.
fn negate_const_value(value: ConstValue) -> Option<ConstValue> {
    match value {
        ConstValue::Integer(value) => Some(ConstValue::Integer(-value)),
        ConstValue::Bigint(value) => Some(ConstValue::Bigint(-value)),
        ConstValue::Float(value) => Some(ConstValue::Float(-value)),
        ConstValue::Boolean(value) => Some(ConstValue::Integer(-(value as i64))),
        ConstValue::Null => Some(ConstValue::Integer(0)),
        ConstValue::Undefined => None,
    }
}

/// Apply bitwise not to a constant value.
fn bit_not_const_value(value: ConstValue) -> Option<ConstValue> {
    match value {
        ConstValue::Integer(value) => Some(ConstValue::Integer(!value)),
        ConstValue::Bigint(value) => Some(ConstValue::Bigint(!value)),
        ConstValue::Boolean(value) => Some(ConstValue::Integer(!(value as i64))),
        ConstValue::Null => Some(ConstValue::Integer(!0)),
        _ => None,
    }
}
