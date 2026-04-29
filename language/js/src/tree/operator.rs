use serde::{Deserialize, Serialize};
/// Unary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    /// `++`
    PostIncrement,
    /// `--`
    PostDecrement,
    /// `++`
    PreIncrement,
    /// `--`
    PreDecrement,
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
    /// Whether the unary operator is a prefix operator.
    #[inline]
    pub fn is_prefix(&self) -> bool {
        matches!(
            self,
            Self::PreIncrement
                | Self::PreDecrement
                | Self::Not
                | Self::Plus
                | Self::Negate
                | Self::ElementwiseNot
                | Self::Typeof
                | Self::Void
        )
    }

    /// Whether the unary operator is a postfix operator.
    #[inline]
    pub fn is_postfix(&self) -> bool {
        !self.is_prefix()
    }
}

/// Binary operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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

/// Assignment operator.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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
