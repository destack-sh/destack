use destack_fir::prelude::*;
use destack_fir::write;

use crate::{AssignOperator, BinaryOperator, TypeBinaryOperator, TypeUnaryOperator, UnaryOperator};

use crate::{CodegenJsFormatContext, CodegenJsFormatter};

impl<'ast> Format<CodegenJsFormatContext<'ast>> for UnaryOperator {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            UnaryOperator::PostIncrement => token("++"),
            UnaryOperator::PostDecrement => token("--"),
            UnaryOperator::PreIncrement => token("++"),
            UnaryOperator::PreDecrement => token("--"),
            UnaryOperator::Not => token("!"),
            UnaryOperator::Negate => token("-"),
            UnaryOperator::Plus => token("+"),
            UnaryOperator::ElementwiseNot => token("~"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<CodegenJsFormatContext<'ast>> for BinaryOperator {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
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

impl<'ast> Format<CodegenJsFormatContext<'ast>> for TypeUnaryOperator {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            TypeUnaryOperator::Type => token("type"),
            TypeUnaryOperator::Readonly => token("readonly"),
            TypeUnaryOperator::Typeof => token("typeof"),
            TypeUnaryOperator::Keyof => token("keyof"),
            TypeUnaryOperator::Infer => token("infer"),
            TypeUnaryOperator::AsConst => token("as const"),
            TypeUnaryOperator::Asserts => token("asserts"),
            TypeUnaryOperator::Not => token("!"),
            TypeUnaryOperator::Maybe => token("?"),
            TypeUnaryOperator::Must => token("!"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<CodegenJsFormatContext<'ast>> for TypeBinaryOperator {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            TypeBinaryOperator::Cast => token("as"),
            TypeBinaryOperator::Is => token("is"),
            TypeBinaryOperator::In => token("in"),
            TypeBinaryOperator::InstanceOf => token("instanceof"),
            TypeBinaryOperator::Satisfies => token("satisfies"),
            TypeBinaryOperator::Extends => token("extends"),
            TypeBinaryOperator::Implements => token("implements"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<CodegenJsFormatContext<'ast>> for AssignOperator {
    fn format(&self, f: &mut CodegenJsFormatter<'ast, '_>) -> FormatResult<()> {
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
