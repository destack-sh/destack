use std::error::Error;
use std::fmt;

use destack_core::{FloatFormat, float_to_bits};
use destack_heap::{HeapReference, SharedHeapReference};
use serde::{Deserialize, Serialize};

/// One engine value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueType {
    /// The void type.
    Void,
    /// The boolean type.
    Bool,
    /// The signed integer type.
    Int,
    /// The unsigned integer type.
    UInt,
    /// The 16-bit IEEE-754 floating point type.
    Float16,
    /// The 16-bit BF16 floating point type.
    Bfloat16,
    /// The 32-bit floating point type.
    Float32,
    /// The 64-bit floating point type.
    Float64,
    /// The character type.
    Char,
    /// The local heap reference type.
    HeapReference,
    /// The shared heap reference type.
    SharedHeapReference,
    /// The native address type.
    Address,
}

/// Engine value type mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueTypeMismatch {
    /// The expected value type.
    pub expected: ValueType,
    /// The actual value type.
    pub actual: ValueType,
}

impl ValueTypeMismatch {
    /// Create one engine value type mismatch.
    pub const fn new(expected: ValueType, actual: ValueType) -> Self {
        Self { expected, actual }
    }
}

impl fmt::Display for ValueTypeMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected value type {:?}, found {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for ValueTypeMismatch {}

/// One signed integer value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedInt {
    /// The integer payload.
    pub value: i128,
    /// The integer width in bits.
    pub width: u16,
}

/// One unsigned integer value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsignedInt {
    /// The integer payload.
    pub value: u128,
    /// The integer width in bits.
    pub width: u16,
}

/// One engine value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    /// The void value.
    Void,
    /// One boolean value.
    Bool(bool),
    /// One signed integer value with its width.
    Int {
        /// The integer payload.
        value: i128,
        /// The integer width in bits.
        width: u16,
    },
    /// One unsigned integer value with its width.
    UInt {
        /// The integer payload.
        value: u128,
        /// The integer width in bits.
        width: u16,
    },
    /// One 16-bit IEEE-754 float encoded as raw bits.
    Float16 {
        /// The IEEE-754 payload bits.
        bits: u16,
    },
    /// One 16-bit BF16 float encoded as raw bits.
    Bfloat16 {
        /// The BF16 payload bits.
        bits: u16,
    },
    /// One 32-bit float encoded as raw bits.
    Float32 {
        /// The IEEE-754 payload bits.
        bits: u32,
    },
    /// One 64-bit float encoded as raw bits.
    Float64 {
        /// The IEEE-754 payload bits.
        bits: u64,
    },
    /// One character value.
    Char(char),
    /// One heap reference.
    HeapReference(HeapReference),
    /// One shared heap reference.
    SharedHeapReference(SharedHeapReference),
    /// One native address.
    Address(usize),
}

impl Value {
    /// The void boundary value.
    pub const VOID: Self = Self::Void;

    /// Create one boolean value.
    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    /// Create one signed integer value.
    pub const fn int(value: i128, width: u16) -> Self {
        Self::Int { value, width }
    }

    /// Create one signed 8-bit integer value.
    pub const fn int8(value: i8) -> Self {
        Self::int(value as i128, 8)
    }

    /// Create one signed 16-bit integer value.
    pub const fn int16(value: i16) -> Self {
        Self::int(value as i128, 16)
    }

    /// Create one signed 32-bit integer value.
    pub const fn int32(value: i32) -> Self {
        Self::int(value as i128, 32)
    }

    /// Create one signed 64-bit integer value.
    pub const fn int64(value: i64) -> Self {
        Self::int(value as i128, 64)
    }

    /// Create one unsigned integer value.
    pub const fn uint(value: u128, width: u16) -> Self {
        Self::UInt { value, width }
    }

    /// Create one unsigned 8-bit integer value.
    pub const fn uint8(value: u8) -> Self {
        Self::uint(value as u128, 8)
    }

    /// Create one unsigned 16-bit integer value.
    pub const fn uint16(value: u16) -> Self {
        Self::uint(value as u128, 16)
    }

    /// Create one unsigned 32-bit integer value.
    pub const fn uint32(value: u32) -> Self {
        Self::uint(value as u128, 32)
    }

    /// Create one unsigned 64-bit integer value.
    pub const fn uint64(value: u64) -> Self {
        Self::uint(value as u128, 64)
    }

    /// Create one 16-bit IEEE-754 floating point value from bits.
    pub const fn float16_bits(bits: u16) -> Self {
        Self::Float16 { bits }
    }

    /// Create one 16-bit IEEE-754 floating point value.
    pub fn float16(value: f64) -> Self {
        Self::float16_bits(float_to_bits(FloatFormat::Float16, value) as u16)
    }

    /// Create one 16-bit BF16 floating point value from bits.
    pub const fn bfloat16_bits(bits: u16) -> Self {
        Self::Bfloat16 { bits }
    }

    /// Create one 16-bit BF16 floating point value.
    pub fn bfloat16(value: f64) -> Self {
        Self::bfloat16_bits(float_to_bits(FloatFormat::Bfloat16, value) as u16)
    }

    /// Create one 32-bit floating point value.
    pub const fn float32(value: f32) -> Self {
        Self::Float32 {
            bits: value.to_bits(),
        }
    }

    /// Create one 64-bit floating point value.
    pub const fn float64(value: f64) -> Self {
        Self::Float64 {
            bits: value.to_bits(),
        }
    }

    /// Create one character value.
    pub const fn char(value: char) -> Self {
        Self::Char(value)
    }

    /// Create one local heap reference value.
    pub const fn heap_reference(reference: HeapReference) -> Self {
        Self::HeapReference(reference)
    }

    /// Create one shared heap reference value.
    pub const fn shared_heap_reference(reference: SharedHeapReference) -> Self {
        Self::SharedHeapReference(reference)
    }

    /// Create one native address value.
    pub const fn address(address: usize) -> Self {
        Self::Address(address)
    }

    /// Return this value's type.
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::Void => ValueType::Void,
            Self::Bool(_) => ValueType::Bool,
            Self::Int { .. } => ValueType::Int,
            Self::UInt { .. } => ValueType::UInt,
            Self::Float16 { .. } => ValueType::Float16,
            Self::Bfloat16 { .. } => ValueType::Bfloat16,
            Self::Float32 { .. } => ValueType::Float32,
            Self::Float64 { .. } => ValueType::Float64,
            Self::Char(_) => ValueType::Char,
            Self::HeapReference(_) => ValueType::HeapReference,
            Self::SharedHeapReference(_) => ValueType::SharedHeapReference,
            Self::Address(_) => ValueType::Address,
        }
    }
}

impl TryFrom<&Value> for () {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Void => Ok(()),
            value => Err(ValueTypeMismatch::new(ValueType::Void, value.value_type())),
        }
    }
}

impl TryFrom<&Value> for bool {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool(value) => Ok(*value),
            value => Err(ValueTypeMismatch::new(ValueType::Bool, value.value_type())),
        }
    }
}

impl TryFrom<&Value> for SignedInt {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Int { value, width } => Ok(Self {
                value: *value,
                width: *width,
            }),
            value => Err(ValueTypeMismatch::new(ValueType::Int, value.value_type())),
        }
    }
}

impl TryFrom<&Value> for UnsignedInt {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::UInt { value, width } => Ok(Self {
                value: *value,
                width: *width,
            }),
            value => Err(ValueTypeMismatch::new(ValueType::UInt, value.value_type())),
        }
    }
}

impl TryFrom<&Value> for f32 {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float32 { bits } => Ok(f32::from_bits(*bits)),
            value => Err(ValueTypeMismatch::new(
                ValueType::Float32,
                value.value_type(),
            )),
        }
    }
}

impl TryFrom<&Value> for f64 {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float64 { bits } => Ok(f64::from_bits(*bits)),
            value => Err(ValueTypeMismatch::new(
                ValueType::Float64,
                value.value_type(),
            )),
        }
    }
}

impl TryFrom<&Value> for char {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Char(value) => Ok(*value),
            value => Err(ValueTypeMismatch::new(ValueType::Char, value.value_type())),
        }
    }
}

impl TryFrom<&Value> for HeapReference {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::HeapReference(reference) => Ok(*reference),
            value => Err(ValueTypeMismatch::new(
                ValueType::HeapReference,
                value.value_type(),
            )),
        }
    }
}

impl TryFrom<&Value> for SharedHeapReference {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::SharedHeapReference(reference) => Ok(*reference),
            value => Err(ValueTypeMismatch::new(
                ValueType::SharedHeapReference,
                value.value_type(),
            )),
        }
    }
}

impl TryFrom<&Value> for usize {
    type Error = ValueTypeMismatch;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Address(address) => Ok(*address),
            value => Err(ValueTypeMismatch::new(
                ValueType::Address,
                value.value_type(),
            )),
        }
    }
}
