use crate::{LocalNodeId, Type};

/// SSA value (virtual register).
///
/// Values are created by instructions and consumed by other instructions.
/// Each value is defined exactly once (SSA property).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

/// A typed value (value + its type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypedValue {
    /// The value.
    pub value: Value,
    /// The type.
    pub ty: LocalNodeId<Type>,
}

impl TypedValue {
    /// Create a new typed value.
    pub fn new(value: Value, ty: LocalNodeId<Type>) -> Self {
        Self { value, ty }
    }
}

/// Constant value in MIR.
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    /// Boolean constant.
    Boolean { value: bool },
    /// Integer constant (up to 64 bits).
    Int { value: i64, width: u8, is_signed: bool },
    /// Unsigned integer constant (up to 64 bits).
    UInt { value: u64, width: u8 },
    /// Floating point constant.
    Float { bits: u64, width: u8 },
}

impl Constant {
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
}
