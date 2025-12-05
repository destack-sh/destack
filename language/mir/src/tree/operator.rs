/// Binary arithmetic/logic operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    // integer arithmetic
    /// Integer addition.
    Add,
    /// Integer subtraction.
    Subtract,
    /// Integer multiplication.
    Multiply,
    /// Signed integer division.
    SignedDivide,
    /// Unsigned integer division.
    UnsignedDivide,
    /// Signed integer remainder.
    SignedRemainder,
    /// Unsigned integer remainder.
    UnsignedRemainder,

    // floating point arithmetic
    /// Floating point addition.
    FloatAdd,
    /// Floating point subtraction.
    FloatSubtract,
    /// Floating point multiplication.
    FloatMultiply,
    /// Floating point division.
    FloatDivide,

    // bitwise
    /// Bitwise AND.
    And,
    /// Bitwise OR.
    Or,
    /// Bitwise XOR.
    Xor,
    /// Shift left.
    ShiftLeft,
    /// Arithmetic shift right (sign-extending).
    ArithmeticShiftRight,
    /// Logical shift right (zero-extending).
    LogicalShiftRight,

    // integer comparison
    /// Equal.
    Equal,
    /// Not equal.
    NotEqual,
    /// Signed less than.
    SignedLessThan,
    /// Signed less than or equal.
    SignedLessEqual,
    /// Signed greater than.
    SignedGreaterThan,
    /// Signed greater than or equal.
    SignedGreaterEqual,
    /// Unsigned less than.
    UnsignedLessThan,
    /// Unsigned less than or equal.
    UnsignedLessEqual,
    /// Unsigned greater than.
    UnsignedGreaterThan,
    /// Unsigned greater than or equal.
    UnsignedGreaterEqual,

    // floating point comparison (ordered)
    /// Floating point equal.
    FloatEqual,
    /// Floating point not equal.
    FloatNotEqual,
    /// Floating point less than.
    FloatLessThan,
    /// Floating point less than or equal.
    FloatLessEqual,
    /// Floating point greater than.
    FloatGreaterThan,
    /// Floating point greater than or equal.
    FloatGreaterEqual,
}

impl BinaryOperator {
    /// Whether this is a comparison operator.
    pub fn is_comparison(&self) -> bool {
        matches!(
            self,
            BinaryOperator::Equal
                | BinaryOperator::NotEqual
                | BinaryOperator::SignedLessThan
                | BinaryOperator::SignedLessEqual
                | BinaryOperator::SignedGreaterThan
                | BinaryOperator::SignedGreaterEqual
                | BinaryOperator::UnsignedLessThan
                | BinaryOperator::UnsignedLessEqual
                | BinaryOperator::UnsignedGreaterThan
                | BinaryOperator::UnsignedGreaterEqual
                | BinaryOperator::FloatEqual
                | BinaryOperator::FloatNotEqual
                | BinaryOperator::FloatLessThan
                | BinaryOperator::FloatLessEqual
                | BinaryOperator::FloatGreaterThan
                | BinaryOperator::FloatGreaterEqual
        )
    }

    /// Whether this is a floating point operator.
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            BinaryOperator::FloatAdd
                | BinaryOperator::FloatSubtract
                | BinaryOperator::FloatMultiply
                | BinaryOperator::FloatDivide
                | BinaryOperator::FloatEqual
                | BinaryOperator::FloatNotEqual
                | BinaryOperator::FloatLessThan
                | BinaryOperator::FloatLessEqual
                | BinaryOperator::FloatGreaterThan
                | BinaryOperator::FloatGreaterEqual
        )
    }
}

/// Unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    /// Integer negation.
    Negate,
    /// Floating point negation.
    FloatNegate,
    /// Bitwise NOT.
    Not,
}
