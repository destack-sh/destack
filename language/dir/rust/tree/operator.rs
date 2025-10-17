/// A UnaryOperator is a unary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
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
    /// `!`
    Not,
    /// `-`
    Negate,
    /// `-%`
    WrappingNegate,
    /// `~`
    ElementwiseNot,
    /// `*`
    Dereference,
    /// `..`
    Spread,
}

/// A BinaryOperator is an infix binary operator.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BinaryOperator {
    // multiplication
    /// `*`
    Multiply,
    /// `*%`
    WrappingMultiply,
    /// `*|`
    SaturatingMultiply,
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
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,

    // logical
    /// `&&`
    And,
    /// `||`
    Or,
    /// `??`
    Coalesce,

    // type
    /// `as`
    Cast,
    /// `in`
    In,
    /// `of`
    Of,
    /// `is`
    Is,
    /// `instanceof`
    Instanceof,
    /// `satisfies`
    Satisfies,
}

/// An AssignOperator is an assignment type.
/// Relative order matches precedence. Also see OperatorPrecedence.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AssignOperator {
    // assignment
    /// `=`
    Assign,

    // assignment multiplication
    /// `*=`
    MultiplyAssign,
    /// `*%=`
    WrappingMultiplyAssign,
    /// `*|=`
    SaturatingMultiplyAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,

    // assignment addition
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

    // assignment shift
    /// `<<=`
    ShiftLeftAssign,
    /// `<<|=`
    SaturatingShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,

    // assignment elementwise
    /// `&=`
    ElementwiseAndAssign,
    /// `^=`
    ElementwiseXorAssign,
    /// `|=`
    ElementwiseOrAssign,

    // assignment logical
    /// `&&=`
    AndAssign,
    /// `||=`
    OrAssign,
}
