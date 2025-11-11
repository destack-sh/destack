use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{Expression, NodeId, TypeUnaryOperator, UnaryOperator};

use crate::{Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a DIR type unary operator to a JavaScript type unary operator.
    pub fn transpile_type_unary_operator(
        &self,
        operator: dir::TypeUnaryOperator,
    ) -> TypeUnaryOperator {
        match operator {
            dir::TypeUnaryOperator::Type => TypeUnaryOperator::Type,
            dir::TypeUnaryOperator::Readonly => TypeUnaryOperator::Readonly,
            dir::TypeUnaryOperator::Not => TypeUnaryOperator::Not,
            dir::TypeUnaryOperator::Maybe => TypeUnaryOperator::Maybe,
            dir::TypeUnaryOperator::Must => TypeUnaryOperator::Must,
            dir::TypeUnaryOperator::Typeof => TypeUnaryOperator::Typeof,
            dir::TypeUnaryOperator::Keyof => TypeUnaryOperator::Keyof,
            dir::TypeUnaryOperator::Infer => TypeUnaryOperator::Infer,
            dir::TypeUnaryOperator::AsConst => TypeUnaryOperator::AsConst,
            dir::TypeUnaryOperator::Asserts => TypeUnaryOperator::Asserts,
        }
    }

    /// Transpile a DIR type binary expression to a JavaScript type binary expression.
    pub fn transpile_type_binary_expression(
        &self,
        module: &'a Module,
        expression_id: dir::NodeId<dir::Expression>,
        left_id: dir::NodeId<dir::Expression>,
        operator: dir::TypeBinaryOperator,
        right_id: dir::NodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> NodeId<Expression> {
        todo!("lower_type_binary_expression {operator:?}");
    }

    /// Transpile a DIR unary expression to a JavaScript unary expression.
    pub fn transpile_unary_expression(
        &self,
        module: &'a Module,
        expression_id: dir::NodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right_id: dir::NodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> NodeId<Expression> {
        let right_id = self.transpile_expression(module, right_id, unit);
        match operator {
            dir::UnaryOperator::PostIncrement => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::PostIncrement,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::PostDecrement => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::PostDecrement,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::PreIncrement => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::PreIncrement,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::PreDecrement => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::PreDecrement,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::Not => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::Not,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::Plus => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::Plus,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::Negate => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::Negate,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::WrappingNegate => {
                panic!("nocheckin: proper TranspileErrors/diagnostics")
            }
            dir::UnaryOperator::ElementwiseNot => {
                let expression = Expression::Unary {
                    operator: UnaryOperator::ElementwiseNot,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::Dereference => {
                // NOTE: nothing to do here?
                right_id
            }
            dir::UnaryOperator::Spread => {
                panic!("nocheckin: proper TranspileErrors/diagnostics")
            }
        }
    }

    /// Transpile a DIR binary expression to a JavaScript binary expression.
    pub fn transpile_binary_expression(
        &self,
        module: &'a Module,
        expression_id: dir::NodeId<dir::Expression>,
        left_id: dir::NodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right_id: dir::NodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> NodeId<Expression> {
        todo!("lower_binary_expression {operator:?}");
    }
}
