use crate::{DestackFormatContext, DestackFormatter};
use destack_ast::{AssignOperator, BinaryOperator, UnaryOperator};
use destack_fir::format::{Buffer, Format, FormatResult};
use destack_fir::prelude::token;
use destack_fir::write;

/// Format unary operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for UnaryOperator {
    /// Write the token form of the unary operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            UnaryOperator::PostIncrement => token("++"),
            UnaryOperator::PostDecrement => token("--"),
            UnaryOperator::PreIncrement => token("++"),
            UnaryOperator::PreDecrement => token("--"),
            UnaryOperator::Not => token("!"),
            UnaryOperator::Negate => token("-"),
            UnaryOperator::Plus => token("+"),
            UnaryOperator::WrappingNegate => token("-%"),
            UnaryOperator::ElementwiseNot => token("~"),
            UnaryOperator::Typeof => token("typeof"),
            UnaryOperator::Void => token("void"),
            UnaryOperator::Dereference => token("*"),
            UnaryOperator::Spread => token("..."),
        };
        write!(f, [token])
    }
}

/// Format binary operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for BinaryOperator {
    /// Write the token form of the binary operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            BinaryOperator::Multiply => token("*"),
            BinaryOperator::WrappingMultiply => token("*%"),
            BinaryOperator::SaturatingMultiply => token("*|"),
            BinaryOperator::Exponent => token("**"),
            BinaryOperator::WrappingExponent => token("**%"),
            BinaryOperator::SaturatingExponent => token("**|"),
            BinaryOperator::Divide => token("/"),
            BinaryOperator::Remainder => token("%"),
            BinaryOperator::Add => token("+"),
            BinaryOperator::WrappingAdd => token("+%"),
            BinaryOperator::SaturatingAdd => token("+|"),
            BinaryOperator::Subtract => token("-"),
            BinaryOperator::WrappingSubtract => token("-%"),
            BinaryOperator::SaturatingSubtract => token("-|"),
            BinaryOperator::ShiftLeft => token("<<"),
            BinaryOperator::SaturatingShiftLeft => token("<<|"),
            BinaryOperator::ShiftRight => token(">>"),
            BinaryOperator::UnsignedShiftRight => token(">>>"),
            BinaryOperator::ElementwiseAnd => token("&"),
            BinaryOperator::ElementwiseXor => token("^"),
            BinaryOperator::ElementwiseOr => token("|"),
            BinaryOperator::Equal => token("=="),
            BinaryOperator::NotEqual => token("!="),
            BinaryOperator::EqualStrict => token("==="),
            BinaryOperator::NotEqualStrict => token("!=="),
            BinaryOperator::LessThan => token("<"),
            BinaryOperator::LessThanOrEqual => token("<="),
            BinaryOperator::GreaterThan => token(">"),
            BinaryOperator::GreaterThanOrEqual => token(">="),
            BinaryOperator::And => token("&&"),
            BinaryOperator::Or => token("||"),
            BinaryOperator::Coalesce => token("??"),
            BinaryOperator::In => token("in"),
        };
        write!(f, [token])
    }
}

/// Format assignment operators as source tokens.
impl<'ast> Format<DestackFormatContext<'ast>> for AssignOperator {
    /// Write the token form of the assignment operator.
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let token = token(match self {
            AssignOperator::Assign => "=",
            AssignOperator::AddAssign => "+=",
            AssignOperator::WrappingAddAssign => "+%=",
            AssignOperator::SaturatingAddAssign => "+|=",
            AssignOperator::SubtractAssign => "-=",
            AssignOperator::WrappingSubtractAssign => "-%=",
            AssignOperator::SaturatingSubtractAssign => "-|=",
            AssignOperator::MultiplyAssign => "*=",
            AssignOperator::WrappingMultiplyAssign => "*%=",
            AssignOperator::SaturatingMultiplyAssign => "*|=",
            AssignOperator::ExponentAssign => "**=",
            AssignOperator::WrappingExponentAssign => "**%=",
            AssignOperator::SaturatingExponentAssign => "**|=",
            AssignOperator::DivideAssign => "/=",
            AssignOperator::RemainderAssign => "%=",
            AssignOperator::ShiftLeftAssign => "<<=",
            AssignOperator::SaturatingShiftLeftAssign => "<<|=",
            AssignOperator::ShiftRightAssign => ">>=",
            AssignOperator::UnsignedShiftRightAssign => ">>>=",
            AssignOperator::ElementwiseAndAssign => "&=",
            AssignOperator::ElementwiseOrAssign => "|=",
            AssignOperator::ElementwiseXorAssign => "^=",
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
            AssignOperator::CoalesceAssign => "??=",
        });
        write!(f, [token])
    }
}
