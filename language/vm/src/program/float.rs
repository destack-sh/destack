use destack_mir as mir;

use crate::{Error, Result};

const FLOAT_FORMAT_FLOAT16: u32 = 1;
const FLOAT_FORMAT_BFLOAT16: u32 = 2;
const FLOAT_FORMAT_FLOAT32: u32 = 3;
const FLOAT_FORMAT_FLOAT64: u32 = 4;
const FLOAT_KERNEL_ADD: u32 = 1;
const FLOAT_KERNEL_SUBTRACT: u32 = 2;
const FLOAT_KERNEL_MULTIPLY: u32 = 3;
const FLOAT_KERNEL_DIVIDE: u32 = 4;
const FLOAT_KERNEL_EQUAL: u32 = 5;
const FLOAT_KERNEL_NOT_EQUAL: u32 = 6;
const FLOAT_KERNEL_LESS_THAN: u32 = 7;
const FLOAT_KERNEL_LESS_EQUAL: u32 = 8;
const FLOAT_KERNEL_GREATER_THAN: u32 = 9;
const FLOAT_KERNEL_GREATER_EQUAL: u32 = 10;
const FLOAT_KERNEL_NEGATE: u32 = 1;
const FLOAT_KERNEL_SHIFT: u32 = 8;
const FLOAT_FORMAT_MASK: u32 = (1 << FLOAT_KERNEL_SHIFT) - 1;

/// Encoded binary float operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BinaryFloat {
    /// The packed instruction field.
    field: u32,
}

impl BinaryFloat {
    /// Encode one binary float operation.
    #[inline(always)]
    pub(crate) const fn new(format: mir::FloatType, kernel: BinaryFloatKernel) -> Self {
        Self {
            field: float_format_field(format) | (kernel.field() << FLOAT_KERNEL_SHIFT),
        }
    }

    /// Decode one instruction field.
    #[inline(always)]
    pub(crate) const fn from_field(field: u32) -> Self {
        Self { field }
    }

    /// Return the packed instruction field.
    #[inline(always)]
    pub(crate) const fn field(self) -> u32 {
        self.field
    }

    /// Decode the float format and binary kernel.
    #[inline(always)]
    pub(crate) fn decode(self) -> Result<(mir::FloatType, BinaryFloatKernel)> {
        let format = float_format_from_field(self.field & FLOAT_FORMAT_MASK)?;
        let kernel = BinaryFloatKernel::from_field(self.field >> FLOAT_KERNEL_SHIFT)?;

        Ok((format, kernel))
    }
}

/// Encoded unary float operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UnaryFloat {
    /// The packed instruction field.
    field: u32,
}

impl UnaryFloat {
    /// Encode one unary float operation.
    #[inline(always)]
    pub(crate) const fn new(format: mir::FloatType, kernel: UnaryFloatKernel) -> Self {
        Self {
            field: float_format_field(format) | (kernel.field() << FLOAT_KERNEL_SHIFT),
        }
    }

    /// Decode one instruction field.
    #[inline(always)]
    pub(crate) const fn from_field(field: u32) -> Self {
        Self { field }
    }

    /// Return the packed instruction field.
    #[inline(always)]
    pub(crate) const fn field(self) -> u32 {
        self.field
    }

    /// Decode the float format and unary kernel.
    #[inline(always)]
    pub(crate) fn decode(self) -> Result<(mir::FloatType, UnaryFloatKernel)> {
        let format = float_format_from_field(self.field & FLOAT_FORMAT_MASK)?;
        let kernel = UnaryFloatKernel::from_field(self.field >> FLOAT_KERNEL_SHIFT)?;

        Ok((format, kernel))
    }
}

/// Generic binary float kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryFloatKernel {
    /// Floating-point addition.
    Add,
    /// Floating-point subtraction.
    Subtract,
    /// Floating-point multiplication.
    Multiply,
    /// Floating-point division.
    Divide,
    /// Floating-point equality.
    Equal,
    /// Floating-point inequality.
    NotEqual,
    /// Floating-point less-than.
    LessThan,
    /// Floating-point less-or-equal.
    LessEqual,
    /// Floating-point greater-than.
    GreaterThan,
    /// Floating-point greater-or-equal.
    GreaterEqual,
}

impl BinaryFloatKernel {
    /// Return the kernel for one MIR binary operator.
    #[inline(always)]
    pub(crate) const fn from_mir(operator: mir::BinaryOperator) -> Option<Self> {
        use mir::BinaryOperator;

        match operator {
            BinaryOperator::FloatAdd => Some(Self::Add),
            BinaryOperator::FloatSubtract => Some(Self::Subtract),
            BinaryOperator::FloatMultiply => Some(Self::Multiply),
            BinaryOperator::FloatDivide => Some(Self::Divide),
            BinaryOperator::FloatEqual => Some(Self::Equal),
            BinaryOperator::FloatNotEqual => Some(Self::NotEqual),
            BinaryOperator::FloatLessThan => Some(Self::LessThan),
            BinaryOperator::FloatLessEqual => Some(Self::LessEqual),
            BinaryOperator::FloatGreaterThan => Some(Self::GreaterThan),
            BinaryOperator::FloatGreaterEqual => Some(Self::GreaterEqual),
            _ => None,
        }
    }

    /// Decode one instruction field.
    #[inline(always)]
    const fn from_field(field: u32) -> Result<Self> {
        match field {
            FLOAT_KERNEL_ADD => Ok(Self::Add),
            FLOAT_KERNEL_SUBTRACT => Ok(Self::Subtract),
            FLOAT_KERNEL_MULTIPLY => Ok(Self::Multiply),
            FLOAT_KERNEL_DIVIDE => Ok(Self::Divide),
            FLOAT_KERNEL_EQUAL => Ok(Self::Equal),
            FLOAT_KERNEL_NOT_EQUAL => Ok(Self::NotEqual),
            FLOAT_KERNEL_LESS_THAN => Ok(Self::LessThan),
            FLOAT_KERNEL_LESS_EQUAL => Ok(Self::LessEqual),
            FLOAT_KERNEL_GREATER_THAN => Ok(Self::GreaterThan),
            FLOAT_KERNEL_GREATER_EQUAL => Ok(Self::GreaterEqual),
            _ => Err(Error::invalid_instruction()),
        }
    }

    /// Return the packed instruction field.
    #[inline(always)]
    const fn field(self) -> u32 {
        match self {
            Self::Add => FLOAT_KERNEL_ADD,
            Self::Subtract => FLOAT_KERNEL_SUBTRACT,
            Self::Multiply => FLOAT_KERNEL_MULTIPLY,
            Self::Divide => FLOAT_KERNEL_DIVIDE,
            Self::Equal => FLOAT_KERNEL_EQUAL,
            Self::NotEqual => FLOAT_KERNEL_NOT_EQUAL,
            Self::LessThan => FLOAT_KERNEL_LESS_THAN,
            Self::LessEqual => FLOAT_KERNEL_LESS_EQUAL,
            Self::GreaterThan => FLOAT_KERNEL_GREATER_THAN,
            Self::GreaterEqual => FLOAT_KERNEL_GREATER_EQUAL,
        }
    }
}

/// Generic unary float kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryFloatKernel {
    /// Floating-point negation.
    Negate,
}

impl UnaryFloatKernel {
    /// Return the kernel for one MIR unary operator.
    #[inline(always)]
    pub(crate) const fn from_mir(operator: mir::UnaryOperator) -> Option<Self> {
        match operator {
            mir::UnaryOperator::FloatNegate => Some(Self::Negate),
            _ => None,
        }
    }

    /// Decode one instruction field.
    #[inline(always)]
    const fn from_field(field: u32) -> Result<Self> {
        match field {
            FLOAT_KERNEL_NEGATE => Ok(Self::Negate),
            _ => Err(Error::invalid_instruction()),
        }
    }

    /// Return the packed instruction field.
    #[inline(always)]
    const fn field(self) -> u32 {
        match self {
            Self::Negate => FLOAT_KERNEL_NEGATE,
        }
    }
}

/// Encode one float format.
#[inline(always)]
const fn float_format_field(format: mir::FloatType) -> u32 {
    match format {
        mir::FloatType::Float16 => FLOAT_FORMAT_FLOAT16,
        mir::FloatType::Bfloat16 => FLOAT_FORMAT_BFLOAT16,
        mir::FloatType::Float32 => FLOAT_FORMAT_FLOAT32,
        mir::FloatType::Float64 => FLOAT_FORMAT_FLOAT64,
    }
}

/// Decode one float format.
#[inline(always)]
const fn float_format_from_field(field: u32) -> Result<mir::FloatType> {
    match field {
        FLOAT_FORMAT_FLOAT16 => Ok(mir::FloatType::Float16),
        FLOAT_FORMAT_BFLOAT16 => Ok(mir::FloatType::Bfloat16),
        FLOAT_FORMAT_FLOAT32 => Ok(mir::FloatType::Float32),
        FLOAT_FORMAT_FLOAT64 => Ok(mir::FloatType::Float64),
        _ => Err(Error::invalid_instruction()),
    }
}
