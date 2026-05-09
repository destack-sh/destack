use serde::{Deserialize, Serialize};

/// A UnaryOperator is a unary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
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
    /// `!`
    Not,
    /// `+`
    Plus,
    /// `-`
    Negate,
    /// `-%`
    WrappingNegate,
    /// `~`
    ElementwiseNot,
    /// `typeof`
    Typeof,
    /// `void`
    Void,
    /// `*`
    Dereference,
    /// `...`
    Spread,
}

/// A BinaryOperator is an infix binary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    // multiplication
    /// `*`
    Multiply,
    /// `*%`
    WrappingMultiply,
    /// `*|`
    SaturatingMultiply,
    /// `**`
    Exponent,
    /// `**%`
    WrappingExponent,
    /// `**|`
    SaturatingExponent,
    /// `/`
    Divide,
    /// `%`
    Remainder,

    // addition
    /// `+`
    Add,
    /// `+%`
    WrappingAdd,
    /// `+|`
    SaturatingAdd,
    /// `-`
    Subtract,
    /// `-%`
    WrappingSubtract,
    /// `-|`
    SaturatingSubtract,

    // shift
    /// `<<`
    ShiftLeft,
    /// `<<|`
    SaturatingShiftLeft,
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
}

/// An AssignOperator is an assignment type.
/// All assignment operators share one right-associative precedence.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssignOperator {
    // multiplication assignment
    /// `*=`
    MultiplyAssign,
    /// `*%=`
    WrappingMultiplyAssign,
    /// `*|=`
    SaturatingMultiplyAssign,
    /// `**=`
    ExponentAssign,
    /// `**%=`
    WrappingExponentAssign,
    /// `**|`
    SaturatingExponentAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,

    // addition assignment
    /// `+=`
    AddAssign,
    /// `+%=`
    WrappingAddAssign,
    /// `+|=`
    SaturatingAddAssign,
    /// `-=`
    SubtractAssign,
    /// `-%=`
    WrappingSubtractAssign,
    /// `-|=`
    SaturatingSubtractAssign,

    // shift assignment
    /// `<<=`
    ShiftLeftAssign,
    /// `<<|=`
    SaturatingShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,
    /// `>>>=`
    UnsignedShiftRightAssign,

    // elementwise assignment
    /// `&=`
    ElementwiseAndAssign,
    /// `^=`
    ElementwiseXorAssign,
    /// `|=`
    ElementwiseOrAssign,

    // logical assignment
    /// `&&=`
    AndAssign,
    /// `||=`
    OrAssign,
    /// `??=`
    CoalesceAssign,
}
