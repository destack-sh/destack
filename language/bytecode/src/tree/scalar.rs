use serde::{Deserialize, Serialize};
use tspp_core::{FloatFormat, float_from_bits, float_to_bits};
use tspp_serde::Reflect;

use crate::ValueType;

/// One scalar machine representation selected by an opcode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Scalar {
    /// A boolean value.
    Boolean = 0,
    /// An 8-bit signed integer.
    Int8 = 1,
    /// An 8-bit unsigned integer.
    Uint8 = 2,
    /// A 16-bit signed integer.
    Int16 = 3,
    /// A 16-bit unsigned integer.
    Uint16 = 4,
    /// A 32-bit signed integer.
    Int32 = 5,
    /// A 32-bit unsigned integer.
    Uint32 = 6,
    /// A 64-bit signed integer.
    Int64 = 7,
    /// A 64-bit unsigned integer.
    Uint64 = 8,
    /// An IEEE 754 binary32 value.
    Float32 = 9,
    /// An IEEE 754 binary64 value.
    Float64 = 10,
}

impl Scalar {
    /// The scalar count in each parameterized opcode range.
    pub(crate) const OPCODE_STRIDE: u16 = Self::Float64 as u16 + 1;
    /// The scalar count in each parameterized integer opcode range.
    pub(crate) const INTEGER_OPCODE_STRIDE: u16 = Self::Uint64 as u16;
    /// The scalar count in each parameterized floating-point opcode range.
    pub(crate) const FLOAT_OPCODE_STRIDE: u16 = Self::Float64 as u16 - Self::Float32 as u16 + 1;

    /// Return the scalar with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "boolean" => Some(Self::Boolean),
            "int8" => Some(Self::Int8),
            "uint8" => Some(Self::Uint8),
            "int16" => Some(Self::Int16),
            "uint16" => Some(Self::Uint16),
            "int32" => Some(Self::Int32),
            "uint32" => Some(Self::Uint32),
            "int64" => Some(Self::Int64),
            "uint64" => Some(Self::Uint64),
            "float32" => Some(Self::Float32),
            "float64" => Some(Self::Float64),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::Int8 => "int8",
            Self::Uint8 => "uint8",
            Self::Int16 => "int16",
            Self::Uint16 => "uint16",
            Self::Int32 => "int32",
            Self::Uint32 => "uint32",
            Self::Int64 => "int64",
            Self::Uint64 => "uint64",
            Self::Float32 => "float32",
            Self::Float64 => "float64",
        }
    }

    /// Return the stable scalar code.
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Decode one stable scalar code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Boolean),
            1 => Some(Self::Int8),
            2 => Some(Self::Uint8),
            3 => Some(Self::Int16),
            4 => Some(Self::Uint16),
            5 => Some(Self::Int32),
            6 => Some(Self::Uint32),
            7 => Some(Self::Int64),
            8 => Some(Self::Uint64),
            9 => Some(Self::Float32),
            10 => Some(Self::Float64),
            _ => None,
        }
    }

    /// Return the dense integer representation index.
    pub const fn integer_index(self) -> Option<u16> {
        match self.code() {
            1..=8 => Some(self.code() as u16 - 1),
            _ => None,
        }
    }

    /// Return whether this is an integer representation.
    pub const fn is_integer(self) -> bool {
        self.integer_index().is_some()
    }

    /// Return whether this is a signed integer representation.
    pub const fn is_signed_integer(self) -> bool {
        matches!(self, Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64)
    }

    /// Return whether this is an unsigned integer representation.
    pub const fn is_unsigned_integer(self) -> bool {
        matches!(
            self,
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64
        )
    }

    /// Return the unsigned integer representation with the same width.
    pub const fn unsigned(self) -> Option<Self> {
        match self {
            Self::Int8 | Self::Uint8 => Some(Self::Uint8),
            Self::Int16 | Self::Uint16 => Some(Self::Uint16),
            Self::Int32 | Self::Uint32 => Some(Self::Uint32),
            Self::Int64 | Self::Uint64 => Some(Self::Uint64),
            _ => None,
        }
    }

    /// Return the dense floating-point representation index.
    pub const fn float_index(self) -> Option<u16> {
        match self.code() {
            9..=10 => Some(self.code() as u16 - 9),
            _ => None,
        }
    }

    /// Return whether this is a floating-point representation.
    pub const fn is_float(self) -> bool {
        self.float_index().is_some()
    }

    /// Return the scalar bit width.
    pub const fn bit_width(self) -> u8 {
        match self {
            Self::Boolean | Self::Int8 | Self::Uint8 => 8,
            Self::Int16 | Self::Uint16 => 16,
            Self::Int32 | Self::Uint32 | Self::Float32 => 32,
            Self::Int64 | Self::Uint64 | Self::Float64 => 64,
        }
    }

    /// Encode memory bits into one canonical register word.
    #[inline(always)]
    pub const fn encode(self, bits: u64) -> u64 {
        match self {
            Self::Boolean => (bits != 0) as u64,
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64 => {
                let shift = u64::BITS as u8 - self.bit_width();

                ((bits << shift) as i64 >> shift) as u64
            }
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64 => {
                let shift = u64::BITS as u8 - self.bit_width();

                bits << shift >> shift
            }
            Self::Float32 => bits as u32 as u64,
            Self::Float64 => bits,
        }
    }

    /// Decode one integer register word into a wide mathematical value.
    pub const fn integer(self, bits: u64) -> Option<i128> {
        if self.is_signed_integer() {
            let shift = u64::BITS as u8 - self.bit_width();
            let value = ((bits << shift) as i64 >> shift) as i128;

            Some(value)
        } else if self.is_unsigned_integer() {
            let width = self.bit_width();
            let value = if width == u64::BITS as u8 {
                bits
            } else {
                bits & ((1_u64 << width) - 1)
            };

            Some(value as i128)
        } else {
            None
        }
    }

    /// Return the inclusive mathematical bounds of one integer representation.
    pub const fn integer_bounds(self) -> Option<(i128, i128)> {
        let width = self.bit_width();
        if self.is_signed_integer() {
            Some((-(1_i128 << (width - 1)), (1_i128 << (width - 1)) - 1))
        } else if self.is_unsigned_integer() {
            Some((0, (1_i128 << width) - 1))
        } else {
            None
        }
    }

    /// Decode one floating-point register word.
    pub fn float(self, bits: u64) -> Option<f64> {
        let format = self.float_format()?;

        Some(float_from_bits(format, bits))
    }

    /// Encode one floating-point register word.
    pub fn float_bits(self, value: f64) -> Option<u64> {
        let format = self.float_format()?;

        Some(float_to_bits(format, value))
    }

    /// Return the concrete floating-point format.
    const fn float_format(self) -> Option<FloatFormat> {
        match self {
            Self::Float32 => Some(FloatFormat::Float32),
            Self::Float64 => Some(FloatFormat::Float64),
            _ => None,
        }
    }
}

/// One boolean operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum BooleanOperation {
    /// Compute logical conjunction.
    And = 0,
    /// Compute logical disjunction.
    Or = 1,
    /// Compute logical exclusive disjunction.
    Xor = 2,
    /// Compute logical negation.
    Not = 3,
}

impl BooleanOperation {
    /// Return the boolean operation with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "xor" => Some(Self::Xor),
            "not" => Some(Self::Not),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Not => "not",
        }
    }

    /// Decode one stable boolean operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::And),
            1 => Some(Self::Or),
            2 => Some(Self::Xor),
            3 => Some(Self::Not),
            _ => None,
        }
    }

    /// Return the number of input values consumed by this operation.
    pub const fn input_count(self) -> usize {
        if matches!(self, Self::Not) { 1 } else { 2 }
    }
}

/// One scalar integer operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum IntegerOperation {
    /// Add with two's-complement wrapping.
    Add = 0,
    /// Subtract with two's-complement wrapping.
    Subtract = 1,
    /// Multiply with two's-complement wrapping.
    Multiply = 2,
    /// Divide and trap on invalid arithmetic.
    Divide = 3,
    /// Compute the remainder and trap on invalid arithmetic.
    Remainder = 4,
    /// Compute bitwise conjunction.
    And = 5,
    /// Compute bitwise disjunction.
    Or = 6,
    /// Compute bitwise exclusive disjunction.
    Xor = 7,
    /// Compute bitwise negation.
    Not = 8,
    /// Shift bits left.
    ShiftLeft = 9,
    /// Shift bits right.
    ShiftRight = 10,
    /// Rotate bits left.
    RotateLeft = 11,
    /// Rotate bits right.
    RotateRight = 12,
    /// Negate with two's-complement wrapping.
    Negate = 13,
    /// Compare for equality.
    Equal = 14,
    /// Compare for inequality.
    NotEqual = 15,
    /// Compare whether the left value is less.
    LessThan = 16,
    /// Compare whether the left value is at most the right value.
    LessEqual = 17,
    /// Compare whether the left value is greater.
    GreaterThan = 18,
    /// Compare whether the left value is at least the right value.
    GreaterEqual = 19,
    /// Count leading zero bits.
    LeadingZeroCount = 20,
    /// Count trailing zero bits.
    TrailingZeroCount = 21,
    /// Count set bits.
    PopulationCount = 22,
    /// Reverse byte order.
    ByteSwap = 23,
    /// Reverse bit order.
    BitReverse = 24,
    /// Add and return an overflow flag.
    AddOverflow = 25,
    /// Subtract and return an overflow flag.
    SubtractOverflow = 26,
    /// Multiply and return an overflow flag.
    MultiplyOverflow = 27,
    /// Add with saturation.
    AddSaturating = 28,
    /// Subtract with saturation.
    SubtractSaturating = 29,
    /// Compute the midpoint without intermediate overflow.
    Midpoint = 30,
    /// Clamp one value between ordered bounds.
    Clamp = 31,
    /// Divide and round the quotient toward positive infinity.
    DivideCeil = 32,
    /// Compute the least nonnegative remainder.
    RemainderEuclidean = 33,
    /// Test whether one integer is a multiple of another.
    IsMultipleOf = 34,
    /// Isolate the least-significant one bit.
    IsolateLowestOne = 35,
    /// Compute the absolute difference.
    AbsDiff = 36,
}

impl IntegerOperation {
    /// Return the integer operation with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Self::Add),
            "sub" => Some(Self::Subtract),
            "mul" => Some(Self::Multiply),
            "div" => Some(Self::Divide),
            "rem" => Some(Self::Remainder),
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "xor" => Some(Self::Xor),
            "not" => Some(Self::Not),
            "shl" => Some(Self::ShiftLeft),
            "shr" => Some(Self::ShiftRight),
            "rotateLeft" => Some(Self::RotateLeft),
            "rotateRight" => Some(Self::RotateRight),
            "negate" => Some(Self::Negate),
            "eq" => Some(Self::Equal),
            "ne" => Some(Self::NotEqual),
            "lt" => Some(Self::LessThan),
            "le" => Some(Self::LessEqual),
            "gt" => Some(Self::GreaterThan),
            "ge" => Some(Self::GreaterEqual),
            "countLeadingZeros" => Some(Self::LeadingZeroCount),
            "countTrailingZeros" => Some(Self::TrailingZeroCount),
            "countOnes" => Some(Self::PopulationCount),
            "byteSwap" => Some(Self::ByteSwap),
            "reverseBits" => Some(Self::BitReverse),
            "add.overflowing" => Some(Self::AddOverflow),
            "sub.overflowing" => Some(Self::SubtractOverflow),
            "mul.overflowing" => Some(Self::MultiplyOverflow),
            "add.saturating" => Some(Self::AddSaturating),
            "sub.saturating" => Some(Self::SubtractSaturating),
            "midpoint" => Some(Self::Midpoint),
            "clamp" => Some(Self::Clamp),
            "divideCeil" => Some(Self::DivideCeil),
            "remainderEuclidean" => Some(Self::RemainderEuclidean),
            "isMultipleOf" => Some(Self::IsMultipleOf),
            "isolateLowestOne" => Some(Self::IsolateLowestOne),
            "absDiff" => Some(Self::AbsDiff),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Remainder => "rem",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Not => "not",
            Self::ShiftLeft => "shl",
            Self::ShiftRight => "shr",
            Self::RotateLeft => "rotateLeft",
            Self::RotateRight => "rotateRight",
            Self::Negate => "negate",
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::LessThan => "lt",
            Self::LessEqual => "le",
            Self::GreaterThan => "gt",
            Self::GreaterEqual => "ge",
            Self::LeadingZeroCount => "countLeadingZeros",
            Self::TrailingZeroCount => "countTrailingZeros",
            Self::PopulationCount => "countOnes",
            Self::ByteSwap => "byteSwap",
            Self::BitReverse => "reverseBits",
            Self::AddOverflow => "add.overflowing",
            Self::SubtractOverflow => "sub.overflowing",
            Self::MultiplyOverflow => "mul.overflowing",
            Self::AddSaturating => "add.saturating",
            Self::SubtractSaturating => "sub.saturating",
            Self::Midpoint => "midpoint",
            Self::Clamp => "clamp",
            Self::DivideCeil => "divideCeil",
            Self::RemainderEuclidean => "remainderEuclidean",
            Self::IsMultipleOf => "isMultipleOf",
            Self::IsolateLowestOne => "isolateLowestOne",
            Self::AbsDiff => "absDiff",
        }
    }

    /// Decode one stable integer operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        if code <= Self::AbsDiff as u8 {
            // SAFETY: every code through the final variant is assigned contiguously.
            Some(unsafe { std::mem::transmute::<u8, Self>(code) })
        } else {
            None
        }
    }

    /// Return whether this operation produces a boolean.
    pub const fn returns_boolean(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::LessThan
                | Self::LessEqual
                | Self::GreaterThan
                | Self::GreaterEqual
                | Self::IsMultipleOf
        )
    }

    /// Return the scalar representation produced for one input representation.
    pub const fn result_scalar(self, input: Scalar) -> Option<Scalar> {
        if !input.is_integer() {
            None
        } else if self.returns_boolean() {
            Some(Scalar::Boolean)
        } else if self.is_count() {
            Some(Scalar::Uint32)
        } else if matches!(self, Self::AbsDiff) {
            input.unsigned()
        } else {
            Some(input)
        }
    }

    /// Return whether this operation returns an overflow flag.
    pub const fn is_overflowing(self) -> bool {
        matches!(
            self,
            Self::AddOverflow | Self::SubtractOverflow | Self::MultiplyOverflow
        )
    }

    /// Return whether this operation returns a bit count.
    pub const fn is_count(self) -> bool {
        matches!(
            self,
            Self::LeadingZeroCount | Self::TrailingZeroCount | Self::PopulationCount
        )
    }

    /// Return whether the second input is an unsigned 32-bit count.
    pub const fn uses_count(self) -> bool {
        matches!(
            self,
            Self::ShiftLeft | Self::ShiftRight | Self::RotateLeft | Self::RotateRight
        )
    }

    /// Return the number of logical input values consumed by this operation.
    pub const fn input_count(self) -> usize {
        if matches!(
            self,
            Self::Not
                | Self::Negate
                | Self::LeadingZeroCount
                | Self::TrailingZeroCount
                | Self::PopulationCount
                | Self::ByteSwap
                | Self::BitReverse
                | Self::IsolateLowestOne
        ) {
            1
        } else if matches!(self, Self::Clamp) {
            3
        } else {
            2
        }
    }
}

/// One scalar floating-point operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FloatOperation {
    /// Add two values.
    Add = 0,
    /// Subtract two values.
    Subtract = 1,
    /// Multiply two values.
    Multiply = 2,
    /// Divide two values.
    Divide = 3,
    /// Compute the truncating floating-point remainder.
    Remainder = 4,
    /// Negate one value.
    Negate = 5,
    /// Compare for ordered equality.
    Equal = 6,
    /// Compare for inequality, including unordered operands.
    NotEqual = 7,
    /// Compare for ordered less-than.
    LessThan = 8,
    /// Compare for ordered less-than-or-equal.
    LessEqual = 9,
    /// Compare for ordered greater-than.
    GreaterThan = 10,
    /// Compare for ordered greater-than-or-equal.
    GreaterEqual = 11,
    /// Compute the square root.
    SquareRoot = 12,
    /// Compute the absolute value.
    Absolute = 13,
    /// Compute one fused multiply-add.
    FusedMultiplyAdd = 14,
    /// Copy one sign bit.
    CopySign = 15,
    /// Select the minimum value, propagating NaN and preferring negative zero.
    Minimum = 16,
    /// Select the maximum value, propagating NaN and preferring positive zero.
    Maximum = 17,
    /// Compute the sine.
    Sin = 18,
    /// Compute the cosine.
    Cos = 19,
    /// Compute the tangent.
    Tan = 20,
    /// Compute the inverse sine.
    Asin = 21,
    /// Compute the inverse cosine.
    Acos = 22,
    /// Compute the inverse tangent.
    Atan = 23,
    /// Compute the two-argument inverse tangent.
    Atan2 = 24,
    /// Compute the natural exponential.
    Exp = 25,
    /// Compute the base-two exponential.
    Exp2 = 26,
    /// Compute the natural logarithm.
    Log = 27,
    /// Compute the base-two logarithm.
    Log2 = 28,
    /// Compute the base-ten logarithm.
    Log10 = 29,
    /// Raise one value to one power.
    Pow = 30,
    /// Round downward.
    Floor = 31,
    /// Round upward.
    Ceil = 32,
    /// Round toward zero.
    Truncate = 33,
    /// Round to the nearest integral value, breaking ties toward even.
    RoundTiesEven = 34,
    /// Compute the midpoint without avoidable overflow or underflow.
    Midpoint = 35,
    /// Clamp one value between ordered bounds.
    Clamp = 36,
    /// Test whether one value is finite.
    IsFinite = 37,
    /// Test whether one value is infinite.
    IsInfinite = 38,
    /// Round to the nearest integral value, breaking ties toward positive infinity.
    Round = 39,
    /// Round to the nearest integral value, breaking ties away from zero.
    RoundTiesAway = 40,
    /// Compute the cube root.
    CubeRoot = 41,
    /// Compute the natural exponential minus one.
    Expm1 = 42,
    /// Compute the natural logarithm after adding one.
    Log1p = 43,
}

impl FloatOperation {
    /// Return the floating-point operation with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Self::Add),
            "sub" => Some(Self::Subtract),
            "mul" => Some(Self::Multiply),
            "div" => Some(Self::Divide),
            "rem" => Some(Self::Remainder),
            "negate" => Some(Self::Negate),
            "eq" => Some(Self::Equal),
            "ne" => Some(Self::NotEqual),
            "lt" => Some(Self::LessThan),
            "le" => Some(Self::LessEqual),
            "gt" => Some(Self::GreaterThan),
            "ge" => Some(Self::GreaterEqual),
            "sqrt" => Some(Self::SquareRoot),
            "cbrt" => Some(Self::CubeRoot),
            "abs" => Some(Self::Absolute),
            "fma" => Some(Self::FusedMultiplyAdd),
            "copySign" => Some(Self::CopySign),
            "min" => Some(Self::Minimum),
            "max" => Some(Self::Maximum),
            "sin" => Some(Self::Sin),
            "cos" => Some(Self::Cos),
            "tan" => Some(Self::Tan),
            "asin" => Some(Self::Asin),
            "acos" => Some(Self::Acos),
            "atan" => Some(Self::Atan),
            "atan2" => Some(Self::Atan2),
            "exp" => Some(Self::Exp),
            "expm1" => Some(Self::Expm1),
            "exp2" => Some(Self::Exp2),
            "log" => Some(Self::Log),
            "log1p" => Some(Self::Log1p),
            "log2" => Some(Self::Log2),
            "log10" => Some(Self::Log10),
            "pow" => Some(Self::Pow),
            "floor" => Some(Self::Floor),
            "ceil" => Some(Self::Ceil),
            "truncate" => Some(Self::Truncate),
            "roundTiesEven" => Some(Self::RoundTiesEven),
            "midpoint" => Some(Self::Midpoint),
            "clamp" => Some(Self::Clamp),
            "isFinite" => Some(Self::IsFinite),
            "isInfinite" => Some(Self::IsInfinite),
            "round" => Some(Self::Round),
            "roundTiesAway" => Some(Self::RoundTiesAway),
            _ => None,
        }
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Subtract => "sub",
            Self::Multiply => "mul",
            Self::Divide => "div",
            Self::Remainder => "rem",
            Self::Negate => "negate",
            Self::Equal => "eq",
            Self::NotEqual => "ne",
            Self::LessThan => "lt",
            Self::LessEqual => "le",
            Self::GreaterThan => "gt",
            Self::GreaterEqual => "ge",
            Self::SquareRoot => "sqrt",
            Self::CubeRoot => "cbrt",
            Self::Absolute => "abs",
            Self::FusedMultiplyAdd => "fma",
            Self::CopySign => "copySign",
            Self::Minimum => "min",
            Self::Maximum => "max",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Exp => "exp",
            Self::Expm1 => "expm1",
            Self::Exp2 => "exp2",
            Self::Log => "log",
            Self::Log1p => "log1p",
            Self::Log2 => "log2",
            Self::Log10 => "log10",
            Self::Pow => "pow",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Truncate => "truncate",
            Self::RoundTiesEven => "roundTiesEven",
            Self::Midpoint => "midpoint",
            Self::Clamp => "clamp",
            Self::IsFinite => "isFinite",
            Self::IsInfinite => "isInfinite",
            Self::Round => "round",
            Self::RoundTiesAway => "roundTiesAway",
        }
    }

    /// Decode one stable floating-point operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        if code <= Self::Log1p as u8 {
            // SAFETY: every code through the final variant is assigned contiguously
            Some(unsafe { std::mem::transmute::<u8, Self>(code) })
        } else {
            None
        }
    }

    /// Return whether this operation produces a boolean.
    pub const fn returns_boolean(self) -> bool {
        matches!(
            self,
            Self::Equal
                | Self::NotEqual
                | Self::LessThan
                | Self::LessEqual
                | Self::GreaterThan
                | Self::GreaterEqual
                | Self::IsFinite
                | Self::IsInfinite
        )
    }

    /// Return the scalar representation produced for one input representation.
    pub const fn result_scalar(self, input: Scalar) -> Option<Scalar> {
        if !input.is_float() {
            None
        } else if self.returns_boolean() {
            Some(Scalar::Boolean)
        } else {
            Some(input)
        }
    }

    /// Return the number of input values consumed by this operation.
    pub const fn input_count(self) -> usize {
        if matches!(
            self,
            Self::Negate
                | Self::SquareRoot
                | Self::CubeRoot
                | Self::Absolute
                | Self::Sin
                | Self::Cos
                | Self::Tan
                | Self::Asin
                | Self::Acos
                | Self::Atan
                | Self::Exp
                | Self::Expm1
                | Self::Exp2
                | Self::Log
                | Self::Log1p
                | Self::Log2
                | Self::Log10
                | Self::Floor
                | Self::Ceil
                | Self::Truncate
                | Self::RoundTiesEven
                | Self::Round
                | Self::RoundTiesAway
                | Self::IsFinite
                | Self::IsInfinite
        ) {
            1
        } else if matches!(self, Self::FusedMultiplyAdd | Self::Clamp) {
            3
        } else {
            2
        }
    }
}

/// One scalar representation conversion.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CastOperation {
    /// Truncate an integer.
    Truncate = 0,
    /// Saturate an integer to a narrower representation.
    Saturate = 1,
    /// Extend a signed integer.
    SignExtend = 2,
    /// Extend an unsigned integer.
    ZeroExtend = 3,
    /// Convert a floating-point value to an integer.
    FloatToInt = 4,
    /// Convert a floating-point value to an integer with saturation.
    FloatToIntSaturating = 5,
    /// Convert an integer to a floating-point value.
    IntToFloat = 6,
    /// Convert between floating-point representations.
    FloatConvert = 7,
    /// Preserve bits while changing their interpretation.
    Bit = 8,
    /// Convert one pointer to an unsigned integer.
    PointerToInt = 9,
    /// Convert one unsigned integer to a pointer.
    IntToPointer = 10,
}

impl CastOperation {
    /// Parse one conversion name with source and target types.
    pub fn parse(name: &str) -> Option<(Self, ValueType, ValueType)> {
        let (conversion, target) = name.rsplit_once('.')?;
        let (operation, source) = conversion.rsplit_once('.')?;
        let source = Self::value_type(source)?;
        let target = Self::value_type(target)?;
        let source_scalar = source.scalar_type();
        let target_scalar = target.scalar_type();
        let cast = match (operation, source_scalar, target_scalar) {
            ("truncate", Some(source), Some(target))
                if source.is_integer() && target.is_integer() =>
            {
                Self::Truncate
            }
            ("saturate", Some(source), Some(target))
                if source.is_integer() && target.is_integer() =>
            {
                Self::Saturate
            }
            ("extend", Some(source), Some(target))
                if source.is_integer() && target.is_integer() =>
            {
                if source.is_signed_integer() {
                    Self::SignExtend
                } else {
                    Self::ZeroExtend
                }
            }
            ("truncate", Some(source), Some(target))
                if source.is_float() && target.is_integer() =>
            {
                Self::FloatToInt
            }
            ("truncate.saturating", Some(source), Some(target))
                if source.is_float() && target.is_integer() =>
            {
                Self::FloatToIntSaturating
            }
            ("convert", Some(source), Some(target)) if source.is_integer() && target.is_float() => {
                Self::IntToFloat
            }
            ("promote" | "demote", Some(source), Some(target))
                if source.is_float() && target.is_float() =>
            {
                Self::FloatConvert
            }
            ("reinterpret", Some(_), Some(_)) => Self::Bit,
            ("reinterpret", None, Some(Scalar::Uint64)) if source.is_pointer() => {
                Self::PointerToInt
            }
            ("reinterpret", Some(Scalar::Uint64), None) if target.is_pointer() => {
                Self::IntToPointer
            }
            _ => return None,
        };

        Some((cast, source, target))
    }

    /// Return the canonical operation name for one exact conversion.
    pub const fn name(self, source: ValueType, target: ValueType) -> Option<&'static str> {
        match self {
            Self::Truncate => Some("truncate"),
            Self::Saturate => Some("saturate"),
            Self::SignExtend | Self::ZeroExtend => Some("extend"),
            Self::FloatToInt => Some("truncate"),
            Self::FloatToIntSaturating => Some("truncate.saturating"),
            Self::IntToFloat => Some("convert"),
            Self::FloatConvert => {
                let Some(source) = source.scalar_type() else {
                    return None;
                };
                let Some(target) = target.scalar_type() else {
                    return None;
                };

                if target.bit_width() < source.bit_width() {
                    Some("demote")
                } else if target.bit_width() > source.bit_width() {
                    Some("promote")
                } else {
                    None
                }
            }
            Self::Bit | Self::PointerToInt | Self::IntToPointer => Some("reinterpret"),
        }
    }

    /// Return one scalar or pointer type used in conversion names.
    fn value_type(name: &str) -> Option<ValueType> {
        if name == "pointer" {
            Some(ValueType::pointer())
        } else {
            Scalar::from_name(name).map(ValueType::scalar)
        }
    }

    /// Decode one stable scalar cast operation code.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Truncate),
            1 => Some(Self::Saturate),
            2 => Some(Self::SignExtend),
            3 => Some(Self::ZeroExtend),
            4 => Some(Self::FloatToInt),
            5 => Some(Self::FloatToIntSaturating),
            6 => Some(Self::IntToFloat),
            7 => Some(Self::FloatConvert),
            8 => Some(Self::Bit),
            9 => Some(Self::PointerToInt),
            10 => Some(Self::IntToPointer),
            _ => None,
        }
    }

    /// Return whether this integer conversion accepts two dense integer representations.
    pub const fn supports(self, source: u16, target: u16) -> bool {
        let source_width = source / 2;
        let target_width = target / 2;

        match self {
            Self::Truncate => target_width < source_width,
            Self::Saturate => source != target,
            Self::SignExtend | Self::ZeroExtend => target_width > source_width,
            Self::FloatToInt
            | Self::FloatToIntSaturating
            | Self::IntToFloat
            | Self::FloatConvert
            | Self::Bit
            | Self::PointerToInt
            | Self::IntToPointer => false,
        }
    }
}
