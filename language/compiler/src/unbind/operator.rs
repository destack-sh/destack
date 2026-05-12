use destack_ast::{self as ast};
use destack_dir::{self as dir};

use super::UnbindContext;
use crate::Compiler;

impl Compiler {
    /// Unbind a DIR unary operator to an AST unary operator.
    pub(super) fn unbind_unary_operator(
        &self,
        _context: &mut UnbindContext,
        operator: dir::UnaryOperator,
    ) -> ast::UnaryOperator {
        match operator {
            dir::UnaryOperator::PostIncrement => ast::UnaryOperator::PostIncrement,
            dir::UnaryOperator::PostDecrement => ast::UnaryOperator::PostDecrement,
            dir::UnaryOperator::PreIncrement => ast::UnaryOperator::PreIncrement,
            dir::UnaryOperator::PreDecrement => ast::UnaryOperator::PreDecrement,
            dir::UnaryOperator::Not => ast::UnaryOperator::Not,
            dir::UnaryOperator::Negate => ast::UnaryOperator::Negate,
            dir::UnaryOperator::Plus => ast::UnaryOperator::Plus,
            dir::UnaryOperator::ElementwiseNot => ast::UnaryOperator::ElementwiseNot,
            dir::UnaryOperator::Dereference => ast::UnaryOperator::Dereference,
            dir::UnaryOperator::Spread => ast::UnaryOperator::Spread,
            dir::UnaryOperator::Typeof => ast::UnaryOperator::Typeof,
            dir::UnaryOperator::Void => ast::UnaryOperator::Void,
        }
    }

    /// Unbind a DIR binary operator to an AST binary operator.
    pub(super) fn unbind_binary_operator(
        &self,
        _context: &mut UnbindContext,
        operator: dir::BinaryOperator,
    ) -> ast::BinaryOperator {
        match operator {
            // multiplication
            dir::BinaryOperator::Multiply => ast::BinaryOperator::Multiply,
            dir::BinaryOperator::Exponent => ast::BinaryOperator::Exponent,
            dir::BinaryOperator::Divide => ast::BinaryOperator::Divide,
            dir::BinaryOperator::Remainder => ast::BinaryOperator::Remainder,

            // addition
            dir::BinaryOperator::Add => ast::BinaryOperator::Add,
            dir::BinaryOperator::Subtract => ast::BinaryOperator::Subtract,

            // shift
            dir::BinaryOperator::ShiftLeft => ast::BinaryOperator::ShiftLeft,
            dir::BinaryOperator::ShiftRight => ast::BinaryOperator::ShiftRight,
            dir::BinaryOperator::UnsignedShiftRight => ast::BinaryOperator::UnsignedShiftRight,

            // elementwise
            dir::BinaryOperator::ElementwiseAnd => ast::BinaryOperator::ElementwiseAnd,
            dir::BinaryOperator::ElementwiseXor => ast::BinaryOperator::ElementwiseXor,
            dir::BinaryOperator::ElementwiseOr => ast::BinaryOperator::ElementwiseOr,

            // comparison
            dir::BinaryOperator::Equal => ast::BinaryOperator::Equal,
            dir::BinaryOperator::NotEqual => ast::BinaryOperator::NotEqual,
            dir::BinaryOperator::EqualStrict => ast::BinaryOperator::EqualStrict,
            dir::BinaryOperator::NotEqualStrict => ast::BinaryOperator::NotEqualStrict,
            dir::BinaryOperator::LessThan => ast::BinaryOperator::LessThan,
            dir::BinaryOperator::LessThanOrEqual => ast::BinaryOperator::LessThanOrEqual,
            dir::BinaryOperator::GreaterThan => ast::BinaryOperator::GreaterThan,
            dir::BinaryOperator::GreaterThanOrEqual => ast::BinaryOperator::GreaterThanOrEqual,

            // logical
            dir::BinaryOperator::And => ast::BinaryOperator::And,
            dir::BinaryOperator::Or => ast::BinaryOperator::Or,
            dir::BinaryOperator::Coalesce => ast::BinaryOperator::Coalesce,

            // container
            dir::BinaryOperator::In => ast::BinaryOperator::In,
        }
    }

    /// Unbind a DIR assign operator to an AST assign operator.
    pub(super) fn unbind_assign_operator(
        &self,
        _context: &mut UnbindContext,
        operator: dir::AssignOperator,
    ) -> ast::AssignOperator {
        match operator {
            // multiplication
            dir::AssignOperator::MultiplyAssign => ast::AssignOperator::MultiplyAssign,
            dir::AssignOperator::ExponentAssign => ast::AssignOperator::ExponentAssign,
            dir::AssignOperator::DivideAssign => ast::AssignOperator::DivideAssign,
            dir::AssignOperator::RemainderAssign => ast::AssignOperator::RemainderAssign,

            // addition
            dir::AssignOperator::AddAssign => ast::AssignOperator::AddAssign,
            dir::AssignOperator::SubtractAssign => ast::AssignOperator::SubtractAssign,

            // shift
            dir::AssignOperator::ShiftLeftAssign => ast::AssignOperator::ShiftLeftAssign,
            dir::AssignOperator::ShiftRightAssign => ast::AssignOperator::ShiftRightAssign,
            dir::AssignOperator::UnsignedShiftRightAssign => {
                ast::AssignOperator::UnsignedShiftRightAssign
            }

            // elementwise
            dir::AssignOperator::ElementwiseAndAssign => ast::AssignOperator::ElementwiseAndAssign,
            dir::AssignOperator::ElementwiseXorAssign => ast::AssignOperator::ElementwiseXorAssign,
            dir::AssignOperator::ElementwiseOrAssign => ast::AssignOperator::ElementwiseOrAssign,

            // logical
            dir::AssignOperator::AndAssign => ast::AssignOperator::AndAssign,
            dir::AssignOperator::OrAssign => ast::AssignOperator::OrAssign,
            dir::AssignOperator::CoalesceAssign => ast::AssignOperator::CoalesceAssign,
        }
    }
}
