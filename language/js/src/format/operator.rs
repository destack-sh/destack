use tspp_fir::prelude::*;
use tspp_fir::write;

use crate::{AssignOperator, BinaryOperator, UnaryOperator};

use crate::{Context, Formatter};

impl<'ast> Format<'ast, Context<'ast>> for UnaryOperator {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            UnaryOperator::PostIncrement => token("++"),
            UnaryOperator::PostDecrement => token("--"),
            UnaryOperator::PreIncrement => token("++"),
            UnaryOperator::PreDecrement => token("--"),
            UnaryOperator::Not => token("!"),
            UnaryOperator::Negate => token("-"),
            UnaryOperator::Plus => token("+"),
            UnaryOperator::ElementwiseNot => token("~"),
            UnaryOperator::Typeof => token("typeof"),
            UnaryOperator::Void => token("void"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for BinaryOperator {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            // multiplication
            BinaryOperator::Multiply => token("*"),
            BinaryOperator::Exponent => token("**"),
            BinaryOperator::Divide => token("/"),
            BinaryOperator::Remainder => token("%"),

            // addition
            BinaryOperator::Add => token("+"),
            BinaryOperator::Subtract => token("-"),

            // shift
            BinaryOperator::ShiftLeft => token("<<"),
            BinaryOperator::ShiftRight => token(">>"),
            BinaryOperator::UnsignedShiftRight => token(">>>"),

            // elementwise
            BinaryOperator::ElementwiseAnd => token("&"),
            BinaryOperator::ElementwiseXor => token("^"),
            BinaryOperator::ElementwiseOr => token("|"),

            // comparison
            BinaryOperator::Equal => token("=="),
            BinaryOperator::NotEqual => token("!="),
            BinaryOperator::EqualStrict => token("==="),
            BinaryOperator::NotEqualStrict => token("!=="),
            BinaryOperator::LessThan => token("<"),
            BinaryOperator::LessThanOrEqual => token("<="),
            BinaryOperator::GreaterThan => token(">"),
            BinaryOperator::GreaterThanOrEqual => token(">="),

            // boolean
            BinaryOperator::And => token("&&"),
            BinaryOperator::Or => token("||"),
            BinaryOperator::Coalesce => token("??"),

            // container
            BinaryOperator::In => token("in"),
            BinaryOperator::InstanceOf => token("instanceof"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<'ast, Context<'ast>> for AssignOperator {
    fn format(&self, f: &mut Formatter<'ast, '_>) -> FormatResult<()> {
        let token = token(match self {
            // addition
            AssignOperator::AddAssign => "+=",
            AssignOperator::SubtractAssign => "-=",

            // multiplication
            AssignOperator::MultiplyAssign => "*=",
            AssignOperator::ExponentAssign => "**=",
            AssignOperator::DivideAssign => "/=",
            AssignOperator::RemainderAssign => "%=",

            // shift
            AssignOperator::ShiftLeftAssign => "<<=",
            AssignOperator::ShiftRightAssign => ">>=",
            AssignOperator::UnsignedShiftRightAssign => ">>>=",

            // elementwise
            AssignOperator::ElementwiseAndAssign => "&=",
            AssignOperator::ElementwiseOrAssign => "|=",
            AssignOperator::ElementwiseXorAssign => "^=",

            // boolean
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
            AssignOperator::CoalesceAssign => "??=",
        });
        write!(f, [token])
    }
}
