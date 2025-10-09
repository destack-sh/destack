use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{AssignOperator, UnaryOperator};

impl<'a> Compiler<'a> {
    /// Lower a unary operator to a DIR unary operator.
    pub fn lower_unary_operator(&self, unary_operator: ast::UnaryOperator) -> UnaryOperator {
        match unary_operator {
            ast::UnaryOperator::Not => UnaryOperator::Not,
            ast::UnaryOperator::Negate => UnaryOperator::Negate,
            ast::UnaryOperator::WrappingNegate => UnaryOperator::WrappingNegate,
            ast::UnaryOperator::ElementwiseNot => UnaryOperator::ElementwiseNot,
            ast::UnaryOperator::Dereference => UnaryOperator::Dereference,
            ast::UnaryOperator::Virtual => UnaryOperator::Virtual,
            ast::UnaryOperator::Spread => UnaryOperator::Spread,
        }
    }

    /// Lower a binary operator to a DIR binary operator.
    pub fn lower_binary_operator(
        &self,
        binary_operator: ast::BinaryOperator,
    ) -> dyst_dir::BinaryOperator {
        match binary_operator {
            // multiplication
            ast::BinaryOperator::Multiply => dyst_dir::BinaryOperator::Multiply,
            ast::BinaryOperator::WrappingMultiply => dyst_dir::BinaryOperator::WrappingMultiply,
            ast::BinaryOperator::SaturatingMultiply => dyst_dir::BinaryOperator::SaturatingMultiply,
            ast::BinaryOperator::Divide => dyst_dir::BinaryOperator::Divide,
            ast::BinaryOperator::Remainder => dyst_dir::BinaryOperator::Remainder,

            // addition
            ast::BinaryOperator::Add => dyst_dir::BinaryOperator::Add,
            ast::BinaryOperator::WrappingAdd => dyst_dir::BinaryOperator::WrappingAdd,
            ast::BinaryOperator::SaturatingAdd => dyst_dir::BinaryOperator::SaturatingAdd,
            ast::BinaryOperator::Subtract => dyst_dir::BinaryOperator::Subtract,
            ast::BinaryOperator::WrappingSubtract => dyst_dir::BinaryOperator::WrappingSubtract,
            ast::BinaryOperator::SaturatingSubtract => dyst_dir::BinaryOperator::SaturatingSubtract,

            // shift
            ast::BinaryOperator::ShiftLeft => dyst_dir::BinaryOperator::ShiftLeft,
            ast::BinaryOperator::SaturatingShiftLeft => {
                dyst_dir::BinaryOperator::SaturatingShiftLeft
            }
            ast::BinaryOperator::ShiftRight => dyst_dir::BinaryOperator::ShiftRight,

            // elementwise
            ast::BinaryOperator::ElementwiseAnd => dyst_dir::BinaryOperator::ElementwiseAnd,
            ast::BinaryOperator::ElementwiseXor => dyst_dir::BinaryOperator::ElementwiseXor,
            ast::BinaryOperator::ElementwiseOr => dyst_dir::BinaryOperator::ElementwiseOr,

            // comparison
            ast::BinaryOperator::Equal => dyst_dir::BinaryOperator::Equal,
            ast::BinaryOperator::NotEqual => dyst_dir::BinaryOperator::NotEqual,
            ast::BinaryOperator::LessThan => dyst_dir::BinaryOperator::LessThan,
            ast::BinaryOperator::LessThanOrEqual => dyst_dir::BinaryOperator::LessThanOrEqual,
            ast::BinaryOperator::GreaterThan => dyst_dir::BinaryOperator::GreaterThan,
            ast::BinaryOperator::GreaterThanOrEqual => dyst_dir::BinaryOperator::GreaterThanOrEqual,

            // logical
            ast::BinaryOperator::And => dyst_dir::BinaryOperator::And,
            ast::BinaryOperator::Or => dyst_dir::BinaryOperator::Or,
            ast::BinaryOperator::Coalesce => dyst_dir::BinaryOperator::Coalesce,
            ast::BinaryOperator::Cast => dyst_dir::BinaryOperator::Cast,
        }
    }

    /// Lower an assign operator to a DIR assignment operator.
    pub fn lower_assign_operator(
        &self,
        assign_operator: ast::AssignOperator,
    ) -> Option<AssignOperator> {
        if assign_operator == ast::AssignOperator::Assign {
            return None;
        }

        let assign_operator = match assign_operator {
            // assignment
            ast::AssignOperator::Assign => unreachable!(),

            // assignment multiplication
            ast::AssignOperator::MultiplyAssign => AssignOperator::MultiplyAssign,
            ast::AssignOperator::WrappingMultiplyAssign => AssignOperator::WrappingMultiplyAssign,
            ast::AssignOperator::SaturatingMultiplyAssign => {
                AssignOperator::SaturatingMultiplyAssign
            }
            ast::AssignOperator::DivideAssign => AssignOperator::DivideAssign,
            ast::AssignOperator::RemainderAssign => AssignOperator::RemainderAssign,

            // assignment addition
            ast::AssignOperator::AddAssign => AssignOperator::AddAssign,
            ast::AssignOperator::WrappingAddAssign => AssignOperator::WrappingAddAssign,
            ast::AssignOperator::SaturatingAddAssign => AssignOperator::SaturatingAddAssign,
            ast::AssignOperator::SubtractAssign => AssignOperator::SubtractAssign,
            ast::AssignOperator::WrappingSubtractAssign => AssignOperator::WrappingSubtractAssign,
            ast::AssignOperator::SaturatingSubtractAssign => {
                AssignOperator::SaturatingSubtractAssign
            }

            // assignment shift
            ast::AssignOperator::ShiftLeftAssign => AssignOperator::ShiftLeftAssign,
            ast::AssignOperator::SaturatingShiftLeftAssign => {
                AssignOperator::SaturatingShiftLeftAssign
            }
            ast::AssignOperator::ShiftRightAssign => AssignOperator::ShiftRightAssign,

            // assignment elementwise
            ast::AssignOperator::ElementwiseAndAssign => AssignOperator::ElementwiseAndAssign,
            ast::AssignOperator::ElementwiseXorAssign => AssignOperator::ElementwiseXorAssign,
            ast::AssignOperator::ElementwiseOrAssign => AssignOperator::ElementwiseOrAssign,

            // assignment logical
            ast::AssignOperator::AndAssign => AssignOperator::AndAssign,
            ast::AssignOperator::OrAssign => AssignOperator::OrAssign,
        };

        Some(assign_operator)
    }
}
