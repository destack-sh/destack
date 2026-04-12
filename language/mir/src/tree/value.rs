use serde::{Deserialize, Serialize};

use crate::{LocalNodeId, Type};

/// SSA value (virtual register).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Value(pub u32);

impl Value {
    /// Create a new value.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the numeric id.
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// One SSA value with its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedValue {
    /// The SSA value.
    pub value: Value,
    /// The type of the value.
    pub ty: LocalNodeId<Type>,
}

impl TypedValue {
    /// Create a new typed value.
    pub fn new(value: Value, ty: LocalNodeId<Type>) -> Self {
        Self { value, ty }
    }
}

/// A compile-time constant value in MIR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Constant {
    /// Null reference constant.
    Null,
    /// Boolean constant.
    Boolean {
        /// The boolean value.
        value: bool,
    },
    /// Signed integer constant.
    Int {
        /// The value, sign-extended to 64 bits.
        value: i64,
        /// The bit width of the integer type.
        width: u8,
        /// Whether this represents a signed integer type.
        is_signed: bool,
    },
    /// Unsigned integer constant.
    UInt {
        /// The value, zero-extended to 64 bits.
        value: u64,
        /// The bit width of the integer type.
        width: u8,
    },
    /// Floating point constant.
    Float {
        /// The value stored as raw bits.
        bits: u64,
        /// The bit width, 32 or 64.
        width: u8,
    },
    /// Character constant.
    Char {
        /// The character value.
        value: char,
    },
}

impl Constant {
    /// Create a null constant.
    pub fn null() -> Self {
        Self::Null
    }

    /// Create a new integer constant.
    pub fn int8(value: i8) -> Self {
        Self::Int {
            value: value as i64,
            width: 8,
            is_signed: true,
        }
    }

    /// Create a new integer constant.
    pub fn int16(value: i16) -> Self {
        Self::Int {
            value: value as i64,
            width: 16,
            is_signed: true,
        }
    }

    /// Create a new integer constant.
    pub fn int32(value: i32) -> Self {
        Self::Int {
            value: value as i64,
            width: 32,
            is_signed: true,
        }
    }

    /// Create a new integer constant.
    pub fn int64(value: i64) -> Self {
        Self::Int {
            value,
            width: 64,
            is_signed: true,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint8(value: u8) -> Self {
        Self::UInt {
            value: value as u64,
            width: 8,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint16(value: u16) -> Self {
        Self::UInt {
            value: value as u64,
            width: 16,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint32(value: u32) -> Self {
        Self::UInt {
            value: value as u64,
            width: 32,
        }
    }

    /// Create a new unsigned integer constant.
    pub fn uint64(value: u64) -> Self {
        Self::UInt { value, width: 64 }
    }

    /// Create a new floating point constant.
    pub fn float32(value: f32) -> Self {
        Self::Float {
            bits: value.to_bits() as u64,
            width: 32,
        }
    }

    /// Create a new floating point constant.
    pub fn float64(value: f64) -> Self {
        Self::Float {
            bits: value.to_bits(),
            width: 64,
        }
    }

    /// Create a new boolean constant.
    pub fn boolean(value: bool) -> Self {
        Self::Boolean { value }
    }

    /// Create a new character constant.
    pub fn char(value: char) -> Self {
        Self::Char { value }
    }
}
