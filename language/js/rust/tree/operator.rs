#[derive(Debug, Copy, Clone, PartialEq)]
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
    BitwiseNot,
    /// `!`
    LogicalNot,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinaryOperator {
    // multiplicative
    /// `**`
    Exponent,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `%`
    Remainder,

    // additive
    /// `+`
    Add,
    /// `-`
    Subtract,

    // bitwise shift
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    ShiftRightUnsigned,

    // relational
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,
    /// `in`
    In,
    /// `instanceof`
    InstanceOf,

    // equality
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `===`
    EqualStrict,
    /// `!==`
    NotEqualStrict,

    // bitwise
    /// `&`
    BitwiseAnd,
    /// `^`
    BitwiseXor,
    /// `|`
    BitwiseOr,

    // logical
    /// `&&`
    LogicalAnd,
    /// `||`
    LogicalOr,
    /// `??`
    NullishCoalesce,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AssignOperator {
    /// `=`
    Assign,

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
    ShiftRightUnsignedAssign,

    /// `&=`
    BitwiseAndAssign,
    /// `^=`
    BitwiseXorAssign,
    /// `|=`
    BitwiseOrAssign,

    /// `&&=`
    LogicalAndAssign,
    /// `||=`
    LogicalOrAssign,
    /// `??=`
    NullishCoalesceAssign,
}
