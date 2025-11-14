use dyst_dir::{self as dir, Module};
use dyst_javascript_ast::{
    BinaryOperator, Expression, NodeId, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator,
};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl<'a> Transpiler<'a> {
    /// Transpile a DIR type unary operator to a JavaScript type unary operator.
    pub fn transpile_type_unary_expression(
        &self,
        module: &'a Module,
        expression_id: dir::NodeId<dir::Expression>,
        operator: dir::TypeUnaryOperator,
        right_id: dir::NodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Expression>> {
        let right_id = self.transpile_expression(module, right_id, unit)?;

        // transpile a trivial unary expression to a JavaScript unary expression
        let mut unary = |operator: TypeUnaryOperator| -> NodeId<Expression> {
            let expression = Expression::TypeUnary {
                operator,
                right: right_id,
            };
            unit.ast
                .insert_from_dir(expression, module.id, expression_id)
        };

        let expression_id = match operator {
            dir::TypeUnaryOperator::Newtype => {
                return Err(TranspileError::UnsupportedExpression {
                    node: expression_id,
                });
            }
            dir::TypeUnaryOperator::Type => unary(TypeUnaryOperator::Type),
            dir::TypeUnaryOperator::Readonly => unary(TypeUnaryOperator::Readonly),
            dir::TypeUnaryOperator::Not => unary(TypeUnaryOperator::Not),
            dir::TypeUnaryOperator::Maybe => unary(TypeUnaryOperator::Maybe),
            dir::TypeUnaryOperator::Must => unary(TypeUnaryOperator::Must),
            dir::TypeUnaryOperator::Typeof => unary(TypeUnaryOperator::Typeof),
            dir::TypeUnaryOperator::Keyof => unary(TypeUnaryOperator::Keyof),
            dir::TypeUnaryOperator::Infer => unary(TypeUnaryOperator::Infer),
            dir::TypeUnaryOperator::AsConst => unary(TypeUnaryOperator::AsConst),
            dir::TypeUnaryOperator::Asserts => unary(TypeUnaryOperator::Asserts),
        };

        Ok(expression_id)
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
    ) -> TranspileResult<NodeId<Expression>> {
        let left_id = self.transpile_expression(module, left_id, unit)?;
        let right_id = self.transpile_expression(module, right_id, unit)?;
        let operator = match operator {
            dir::TypeBinaryOperator::Cast => TypeBinaryOperator::Cast,
            dir::TypeBinaryOperator::In => TypeBinaryOperator::In,
            dir::TypeBinaryOperator::Is => TypeBinaryOperator::Is,
            dir::TypeBinaryOperator::InstanceOf => TypeBinaryOperator::InstanceOf,
            dir::TypeBinaryOperator::Satisfies => TypeBinaryOperator::Satisfies,
            dir::TypeBinaryOperator::Extends => TypeBinaryOperator::Extends,
            dir::TypeBinaryOperator::Implements => TypeBinaryOperator::Implements,
        };
        let expression = Expression::TypeBinary {
            left: left_id,
            operator,
            right: right_id,
        };
        let expression_id = unit
            .ast
            .insert_from_dir(expression, module.id, expression_id);
        Ok(expression_id)
    }

    /// Transpile a DIR unary expression to a JavaScript unary expression.
    pub fn transpile_unary_expression(
        &self,
        module: &'a Module,
        expression_id: dir::NodeId<dir::Expression>,
        operator: dir::UnaryOperator,
        right_id: dir::NodeId<dir::Expression>,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<NodeId<Expression>> {
        let right_id = self.transpile_expression(module, right_id, unit)?;

        // transpile a trivial unary expression to a JavaScript unary expression
        let mut unary = |operator: UnaryOperator| -> NodeId<Expression> {
            let expression = Expression::Unary {
                operator,
                right: right_id,
            };
            unit.ast
                .insert_from_dir(expression, module.id, expression_id)
        };

        let expression_id = match operator {
            dir::UnaryOperator::PostIncrement => unary(UnaryOperator::PostIncrement),
            dir::UnaryOperator::PostDecrement => unary(UnaryOperator::PostDecrement),
            dir::UnaryOperator::PreIncrement => unary(UnaryOperator::PreIncrement),
            dir::UnaryOperator::PreDecrement => unary(UnaryOperator::PreDecrement),
            dir::UnaryOperator::Not => unary(UnaryOperator::Not),
            dir::UnaryOperator::Plus => unary(UnaryOperator::Plus),
            dir::UnaryOperator::Negate => unary(UnaryOperator::Negate),
            dir::UnaryOperator::WrappingNegate => {
                // NOTE #Broken: transpile UnaryOperator.WrappingNegate
                let expression = Expression::Unary {
                    operator: UnaryOperator::Negate,
                    right: right_id,
                };
                unit.ast
                    .insert_from_dir(expression, module.id, expression_id)
            }
            dir::UnaryOperator::ElementwiseNot => unary(UnaryOperator::ElementwiseNot),
            dir::UnaryOperator::Dereference => {
                // nothing to do here
                right_id
            }
            dir::UnaryOperator::Spread => {
                return Err(TranspileError::UnsupportedExpression {
                    node: expression_id,
                });
            }
        };

        Ok(expression_id)
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
    ) -> TranspileResult<NodeId<Expression>> {
        let left_id = self.transpile_expression(module, left_id, unit)?;
        let right_id = self.transpile_expression(module, right_id, unit)?;

        let mut binary = |operator: BinaryOperator| -> NodeId<Expression> {
            let expression = Expression::Binary {
                left: left_id,
                operator,
                right: right_id,
            };
            unit.ast
                .insert_from_dir(expression, module.id, expression_id)
        };

        let expression_id = match operator {
            // multiplication
            dir::BinaryOperator::Multiply => binary(BinaryOperator::Multiply),
            dir::BinaryOperator::Exponent => binary(BinaryOperator::Exponent),
            dir::BinaryOperator::Divide => binary(BinaryOperator::Divide),
            dir::BinaryOperator::Remainder => binary(BinaryOperator::Remainder),

            // addition
            dir::BinaryOperator::Add => binary(BinaryOperator::Add),
            dir::BinaryOperator::Subtract => binary(BinaryOperator::Subtract),

            // shift
            dir::BinaryOperator::ShiftLeft => binary(BinaryOperator::ShiftLeft),
            dir::BinaryOperator::ShiftRight => binary(BinaryOperator::ShiftRight),
            dir::BinaryOperator::UnsignedShiftRight => binary(BinaryOperator::UnsignedShiftRight),

            // elementwise
            dir::BinaryOperator::ElementwiseAnd => binary(BinaryOperator::ElementwiseAnd),
            dir::BinaryOperator::ElementwiseXor => binary(BinaryOperator::ElementwiseXor),
            dir::BinaryOperator::ElementwiseOr => binary(BinaryOperator::ElementwiseOr),

            // comparison
            dir::BinaryOperator::Equal => binary(BinaryOperator::Equal),
            dir::BinaryOperator::NotEqual => binary(BinaryOperator::NotEqual),
            dir::BinaryOperator::EqualStrict => binary(BinaryOperator::EqualStrict),
            dir::BinaryOperator::NotEqualStrict => binary(BinaryOperator::NotEqualStrict),
            dir::BinaryOperator::LessThan => binary(BinaryOperator::LessThan),
            dir::BinaryOperator::LessThanOrEqual => binary(BinaryOperator::LessThanOrEqual),
            dir::BinaryOperator::GreaterThan => binary(BinaryOperator::GreaterThan),
            dir::BinaryOperator::GreaterThanOrEqual => binary(BinaryOperator::GreaterThanOrEqual),

            // boolean
            dir::BinaryOperator::And => binary(BinaryOperator::And),
            dir::BinaryOperator::Or => binary(BinaryOperator::Or),
            dir::BinaryOperator::Coalesce => binary(BinaryOperator::Coalesce),

            // container
            dir::BinaryOperator::In => binary(BinaryOperator::In),
            dir::BinaryOperator::InstanceOf => binary(BinaryOperator::InstanceOf),

            _ => {
                return Err(TranspileError::UnsupportedExpression {
                    node: expression_id,
                });
            }
        };

        Ok(expression_id)
    }
}
