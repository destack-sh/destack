/// Unary operator.
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
    ElementwiseNot,
    /// `!`
    Not,
}

/// Binary operator.
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

    // shift
    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    ShiftRightUnsigned,

    // comparison
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `===`
    EqualStrict,
    /// `!==`
    NotEqualStrict,

    // elementwise
    /// `&`
    ElementwiseAnd,
    /// `^`
    ElementwiseXor,
    /// `|`
    ElementwiseOr,

    // logical
    /// `&&`
    And,
    /// `||`
    Or,
    /// `??`
    Coalesce,

    // container
    /// `in`
    In,
    /// `of`
    Of,
    /// `instanceof`
    InstanceOf,
}

/// Assignment operator.
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
