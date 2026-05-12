use crate::Compiler;
use destack_ast as ast;
use destack_dir::{AssignOperator, UnaryOperator};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Bind a unary operator to a DIR unary operator.
    pub(super) fn bind_unary_operator(&self, unary_operator: ast::UnaryOperator) -> UnaryOperator {
        match unary_operator {
            ast::UnaryOperator::PostIncrement => UnaryOperator::PostIncrement,
            ast::UnaryOperator::PostDecrement => UnaryOperator::PostDecrement,
            ast::UnaryOperator::PreIncrement => UnaryOperator::PreIncrement,
            ast::UnaryOperator::PreDecrement => UnaryOperator::PreDecrement,
            ast::UnaryOperator::Not => UnaryOperator::Not,
            ast::UnaryOperator::Negate => UnaryOperator::Negate,
            ast::UnaryOperator::Plus => UnaryOperator::Plus,
            ast::UnaryOperator::ElementwiseNot => UnaryOperator::ElementwiseNot,
            ast::UnaryOperator::Typeof => UnaryOperator::Typeof,
            ast::UnaryOperator::Void => UnaryOperator::Void,
            ast::UnaryOperator::Dereference => UnaryOperator::Dereference,
            ast::UnaryOperator::Spread => UnaryOperator::Spread,
        }
    }

    /// Bind a binary operator to a DIR binary operator.
    pub(super) fn bind_binary_operator(
        &self,
        binary_operator: ast::BinaryOperator,
    ) -> destack_dir::BinaryOperator {
        match binary_operator {
            // multiplication
            ast::BinaryOperator::Multiply => destack_dir::BinaryOperator::Multiply,
            ast::BinaryOperator::Exponent => destack_dir::BinaryOperator::Exponent,
            ast::BinaryOperator::Divide => destack_dir::BinaryOperator::Divide,
            ast::BinaryOperator::Remainder => destack_dir::BinaryOperator::Remainder,

            // addition
            ast::BinaryOperator::Add => destack_dir::BinaryOperator::Add,
            ast::BinaryOperator::Subtract => destack_dir::BinaryOperator::Subtract,

            // shift
            ast::BinaryOperator::ShiftLeft => destack_dir::BinaryOperator::ShiftLeft,
            ast::BinaryOperator::ShiftRight => destack_dir::BinaryOperator::ShiftRight,
            ast::BinaryOperator::UnsignedShiftRight => {
                destack_dir::BinaryOperator::UnsignedShiftRight
            }

            // elementwise
            ast::BinaryOperator::ElementwiseAnd => destack_dir::BinaryOperator::ElementwiseAnd,
            ast::BinaryOperator::ElementwiseXor => destack_dir::BinaryOperator::ElementwiseXor,
            ast::BinaryOperator::ElementwiseOr => destack_dir::BinaryOperator::ElementwiseOr,

            // comparison
            ast::BinaryOperator::Equal => destack_dir::BinaryOperator::Equal,
            ast::BinaryOperator::NotEqual => destack_dir::BinaryOperator::NotEqual,
            ast::BinaryOperator::EqualStrict => destack_dir::BinaryOperator::EqualStrict,
            ast::BinaryOperator::NotEqualStrict => destack_dir::BinaryOperator::NotEqualStrict,
            ast::BinaryOperator::LessThan => destack_dir::BinaryOperator::LessThan,
            ast::BinaryOperator::LessThanOrEqual => destack_dir::BinaryOperator::LessThanOrEqual,
            ast::BinaryOperator::GreaterThan => destack_dir::BinaryOperator::GreaterThan,
            ast::BinaryOperator::GreaterThanOrEqual => {
                destack_dir::BinaryOperator::GreaterThanOrEqual
            }

            // logical
            ast::BinaryOperator::And => destack_dir::BinaryOperator::And,
            ast::BinaryOperator::Or => destack_dir::BinaryOperator::Or,
            ast::BinaryOperator::Coalesce => destack_dir::BinaryOperator::Coalesce,

            // container
            ast::BinaryOperator::In => destack_dir::BinaryOperator::In,
        }
    }

    /// Bind an assign operator to a DIR assignment operator.
    pub(super) fn bind_assign_operator(
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
            ast::AssignOperator::ExponentAssign => AssignOperator::ExponentAssign,
            ast::AssignOperator::DivideAssign => AssignOperator::DivideAssign,
            ast::AssignOperator::RemainderAssign => AssignOperator::RemainderAssign,

            // assignment addition
            ast::AssignOperator::AddAssign => AssignOperator::AddAssign,
            ast::AssignOperator::SubtractAssign => AssignOperator::SubtractAssign,

            // assignment shift
            ast::AssignOperator::ShiftLeftAssign => AssignOperator::ShiftLeftAssign,
            ast::AssignOperator::ShiftRightAssign => AssignOperator::ShiftRightAssign,
            ast::AssignOperator::UnsignedShiftRightAssign => {
                AssignOperator::UnsignedShiftRightAssign
            }

            // assignment elementwise
            ast::AssignOperator::ElementwiseAndAssign => AssignOperator::ElementwiseAndAssign,
            ast::AssignOperator::ElementwiseXorAssign => AssignOperator::ElementwiseXorAssign,
            ast::AssignOperator::ElementwiseOrAssign => AssignOperator::ElementwiseOrAssign,

            // assignment logical
            ast::AssignOperator::AndAssign => AssignOperator::AndAssign,
            ast::AssignOperator::OrAssign => AssignOperator::OrAssign,
            ast::AssignOperator::CoalesceAssign => AssignOperator::CoalesceAssign,
        };

        Some(assign_operator)
    }
}
