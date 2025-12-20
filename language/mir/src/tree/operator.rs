use std::fmt;
use std::str::FromStr;

// nocheckin: deduplicate MIR operators vs intrinsics?
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
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            // integer arithmetic
            BinaryOperator::Add => "iadd",
            BinaryOperator::Subtract => "isub",
            BinaryOperator::Multiply => "imul",
            BinaryOperator::SignedDivide => "sdiv",
            BinaryOperator::UnsignedDivide => "udiv",
            BinaryOperator::SignedRemainder => "srem",
            BinaryOperator::UnsignedRemainder => "urem",
            // float arithmetic
            BinaryOperator::FloatAdd => "fadd",
            BinaryOperator::FloatSubtract => "fsub",
            BinaryOperator::FloatMultiply => "fmul",
            BinaryOperator::FloatDivide => "fdiv",
            // bitwise
            BinaryOperator::And => "band",
            BinaryOperator::Or => "bor",
            BinaryOperator::Xor => "bxor",
            BinaryOperator::ShiftLeft => "ishl",
            BinaryOperator::ArithmeticShiftRight => "sshr",
            BinaryOperator::LogicalShiftRight => "ushr",
            // integer comparison
            BinaryOperator::Equal => "icmp_eq",
            BinaryOperator::NotEqual => "icmp_ne",
            BinaryOperator::SignedLessThan => "icmp_slt",
            BinaryOperator::SignedLessEqual => "icmp_sle",
            BinaryOperator::SignedGreaterThan => "icmp_sgt",
            BinaryOperator::SignedGreaterEqual => "icmp_sge",
            BinaryOperator::UnsignedLessThan => "icmp_ult",
            BinaryOperator::UnsignedLessEqual => "icmp_ule",
            BinaryOperator::UnsignedGreaterThan => "icmp_ugt",
            BinaryOperator::UnsignedGreaterEqual => "icmp_uge",
            // float comparison
            BinaryOperator::FloatEqual => "fcmp_eq",
            BinaryOperator::FloatNotEqual => "fcmp_ne",
            BinaryOperator::FloatLessThan => "fcmp_lt",
            BinaryOperator::FloatLessEqual => "fcmp_le",
            BinaryOperator::FloatGreaterThan => "fcmp_gt",
            BinaryOperator::FloatGreaterEqual => "fcmp_ge",
        }
    }

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

    /// Whether this is an integer arithmetic or bitwise operator.
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            BinaryOperator::Add
                | BinaryOperator::Subtract
                | BinaryOperator::Multiply
                | BinaryOperator::SignedDivide
                | BinaryOperator::UnsignedDivide
                | BinaryOperator::SignedRemainder
                | BinaryOperator::UnsignedRemainder
                | BinaryOperator::And
                | BinaryOperator::Or
                | BinaryOperator::Xor
                | BinaryOperator::ShiftLeft
                | BinaryOperator::ArithmeticShiftRight
                | BinaryOperator::LogicalShiftRight
        )
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for BinaryOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // integer arithmetic
            "iadd" => Ok(BinaryOperator::Add),
            "isub" => Ok(BinaryOperator::Subtract),
            "imul" => Ok(BinaryOperator::Multiply),
            "sdiv" => Ok(BinaryOperator::SignedDivide),
            "udiv" => Ok(BinaryOperator::UnsignedDivide),
            "srem" => Ok(BinaryOperator::SignedRemainder),
            "urem" => Ok(BinaryOperator::UnsignedRemainder),
            // float arithmetic
            "fadd" => Ok(BinaryOperator::FloatAdd),
            "fsub" => Ok(BinaryOperator::FloatSubtract),
            "fmul" => Ok(BinaryOperator::FloatMultiply),
            "fdiv" => Ok(BinaryOperator::FloatDivide),
            // bitwise
            "band" => Ok(BinaryOperator::And),
            "bor" => Ok(BinaryOperator::Or),
            "bxor" => Ok(BinaryOperator::Xor),
            "ishl" => Ok(BinaryOperator::ShiftLeft),
            "sshr" => Ok(BinaryOperator::ArithmeticShiftRight),
            "ushr" => Ok(BinaryOperator::LogicalShiftRight),
            // integer comparison
            "icmp_eq" => Ok(BinaryOperator::Equal),
            "icmp_ne" => Ok(BinaryOperator::NotEqual),
            "icmp_slt" => Ok(BinaryOperator::SignedLessThan),
            "icmp_sle" => Ok(BinaryOperator::SignedLessEqual),
            "icmp_sgt" => Ok(BinaryOperator::SignedGreaterThan),
            "icmp_sge" => Ok(BinaryOperator::SignedGreaterEqual),
            "icmp_ult" => Ok(BinaryOperator::UnsignedLessThan),
            "icmp_ule" => Ok(BinaryOperator::UnsignedLessEqual),
            "icmp_ugt" => Ok(BinaryOperator::UnsignedGreaterThan),
            "icmp_uge" => Ok(BinaryOperator::UnsignedGreaterEqual),
            // float comparison
            "fcmp_eq" => Ok(BinaryOperator::FloatEqual),
            "fcmp_ne" => Ok(BinaryOperator::FloatNotEqual),
            "fcmp_lt" => Ok(BinaryOperator::FloatLessThan),
            "fcmp_le" => Ok(BinaryOperator::FloatLessEqual),
            "fcmp_gt" => Ok(BinaryOperator::FloatGreaterThan),
            "fcmp_ge" => Ok(BinaryOperator::FloatGreaterEqual),
            _ => Err(()),
        }
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

impl UnaryOperator {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            UnaryOperator::Negate => "ineg",
            UnaryOperator::FloatNegate => "fneg",
            UnaryOperator::Not => "bnot",
        }
    }
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl FromStr for UnaryOperator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ineg" => Ok(UnaryOperator::Negate),
            "fneg" => Ok(UnaryOperator::FloatNegate),
            "bnot" => Ok(UnaryOperator::Not),
            _ => Err(()),
        }
    }
}
