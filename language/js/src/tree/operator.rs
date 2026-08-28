use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::Precedence;

/// Unary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum UnaryOperator {
    /// `+`
    Plus,
    /// `-`
    Negate,
    /// `~`
    ElementwiseNot,
    /// `!`
    Not,
    /// `typeof`
    Typeof,
    /// `void`
    Void,
}

impl UnaryOperator {
    /// Return the ECMAScript token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Negate => "-",
            Self::ElementwiseNot => "~",
            Self::Not => "!",
            Self::Typeof => "typeof",
            Self::Void => "void",
        }
    }

    /// Return whether the operator uses a keyword token.
    pub const fn is_keyword(self) -> bool {
        matches!(self, Self::Typeof | Self::Void)
    }
}

/// One update operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum UpdateOperator {
    /// `++`
    Increment,
    /// `--`
    Decrement,
}

impl UpdateOperator {
    /// Return the ECMAScript token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Increment => "++",
            Self::Decrement => "--",
        }
    }
}

/// The position of an update operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum UpdatePosition {
    /// Prefix update like `++value`.
    Prefix,
    /// Postfix update like `value++`.
    Postfix,
}

/// Binary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum BinaryOperator {
    // multiplication
    /// `*`
    Multiply,
    /// `**`
    Exponent,
    /// `/`
    Divide,
    /// `%`
    Remainder,

    // addition
    /// `+`
    Add,
    /// `-`
    Subtract,

    // shift
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    UnsignedShiftRight,

    // elementwise
    /// `&`
    ElementwiseAnd,
    /// `^`
    ElementwiseXor,
    /// `|`
    ElementwiseOr,

    // comparison
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `===`
    EqualStrict,
    /// `!==`
    NotEqualStrict,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,

    // boolean
    /// `&&`
    And,
    /// `||`
    Or,
    /// `??`
    Coalesce,

    // container
    /// `in`
    In,
    /// `instanceof`
    InstanceOf,
}

impl BinaryOperator {
    /// Return the ECMAScript token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Multiply => "*",
            Self::Exponent => "**",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::Add => "+",
            Self::Subtract => "-",
            Self::ShiftLeft => "<<",
            Self::ShiftRight => ">>",
            Self::UnsignedShiftRight => ">>>",
            Self::ElementwiseAnd => "&",
            Self::ElementwiseXor => "^",
            Self::ElementwiseOr => "|",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::EqualStrict => "===",
            Self::NotEqualStrict => "!==",
            Self::LessThan => "<",
            Self::LessThanOrEqual => "<=",
            Self::GreaterThan => ">",
            Self::GreaterThanOrEqual => ">=",
            Self::And => "&&",
            Self::Or => "||",
            Self::Coalesce => "??",
            Self::In => "in",
            Self::InstanceOf => "instanceof",
        }
    }

    /// Return whether the operator uses a keyword token.
    pub const fn is_keyword(self) -> bool {
        matches!(self, Self::In | Self::InstanceOf)
    }

    /// Return the local precedence for this binary operator.
    pub(crate) fn precedence(self) -> Precedence {
        match self {
            Self::Multiply | Self::Divide | Self::Remainder => Precedence::Multiply,
            Self::Exponent => Precedence::Exponent,
            Self::Add | Self::Subtract => Precedence::Add,
            Self::ShiftLeft | Self::ShiftRight | Self::UnsignedShiftRight => Precedence::Shift,
            Self::ElementwiseAnd => Precedence::BitwiseAnd,
            Self::ElementwiseXor => Precedence::BitwiseXor,
            Self::ElementwiseOr => Precedence::BitwiseOr,
            Self::Equal | Self::NotEqual | Self::EqualStrict | Self::NotEqualStrict => {
                Precedence::Equality
            }
            Self::LessThan
            | Self::LessThanOrEqual
            | Self::GreaterThan
            | Self::GreaterThanOrEqual
            | Self::In
            | Self::InstanceOf => Precedence::Compare,
            Self::And => Precedence::LogicalAnd,
            Self::Or => Precedence::LogicalOr,
            Self::Coalesce => Precedence::Coalesce,
        }
    }
}

/// One assignment operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum AssignOperator {
    /// `+=`
    AddAssign,
    /// `-=`
    SubtractAssign,
    /// `*=`
    MultiplyAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,
    /// `**=`
    ExponentAssign,

    /// `<<=`
    ShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,
    /// `>>>=`
    UnsignedShiftRightAssign,

    /// `&=`
    ElementwiseAndAssign,
    /// `^=`
    ElementwiseXorAssign,
    /// `|=`
    ElementwiseOrAssign,

    /// `&&=`
    AndAssign,
    /// `||=`
    OrAssign,
    /// `??=`
    CoalesceAssign,
}

impl AssignOperator {
    /// Return the ECMAScript token.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AddAssign => "+=",
            Self::SubtractAssign => "-=",
            Self::MultiplyAssign => "*=",
            Self::DivideAssign => "/=",
            Self::RemainderAssign => "%=",
            Self::ExponentAssign => "**=",
            Self::ShiftLeftAssign => "<<=",
            Self::ShiftRightAssign => ">>=",
            Self::UnsignedShiftRightAssign => ">>>=",
            Self::ElementwiseAndAssign => "&=",
            Self::ElementwiseXorAssign => "^=",
            Self::ElementwiseOrAssign => "|=",
            Self::AndAssign => "&&=",
            Self::OrAssign => "||=",
            Self::CoalesceAssign => "??=",
        }
    }
}
