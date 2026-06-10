use serde::{Deserialize, Serialize};

use destack_core::{FloatFormat, roundtrip_float};

use crate::StringId;

/// A primitive type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// Boolean type.
    Boolean,
    /// Character type.
    Character,
    /// String type (unsized).
    String,
    /// Bigint type (unsized).
    Bigint,
    /// Integer type.
    Integer(IntegerType),
    /// Float type.
    Float(FloatType),
    /// Symbol type.
    Symbol,
    /// Unique symbol type.
    UniqueSymbol,
}

/// The backing representation of an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnumBackingType {
    /// Integer-backed enums.
    Integer(IntegerType),
    /// String-backed enums.
    String,
}

/// A resolved enum field value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnumFieldValue {
    /// Integer enum value.
    Int(i64),
    /// String enum value.
    String(StringId),
}

/// An integer type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntegerType {
    /// The signed or unsigned integer family.
    Integer { is_signed: bool },
    /// A fixed-width signed or unsigned integer.
    Fixed { width: u16, is_signed: bool },
    /// A pointer-sized signed or unsigned integer.
    Pointer { is_signed: bool },
}

impl IntegerType {
    /// Return the fixed bit width, if known without target layout.
    pub fn width(&self) -> Option<u16> {
        match self {
            IntegerType::Integer { .. } => None,
            IntegerType::Fixed { width, .. } => Some(*width),
            IntegerType::Pointer { .. } => None,
        }
    }

    /// Whether the integer type is signed.
    pub fn is_signed(&self) -> bool {
        match self {
            IntegerType::Integer { is_signed }
            | IntegerType::Fixed {
                width: _,
                is_signed,
            }
            | IntegerType::Pointer { is_signed } => *is_signed,
        }
    }

    /// Get the string representation of the integer type.
    #[inline]
    pub fn as_str(self) -> String {
        match self {
            IntegerType::Integer { is_signed } => {
                if is_signed {
                    "int".to_string()
                } else {
                    "uint".to_string()
                }
            }
            IntegerType::Fixed { width, is_signed } => {
                if is_signed {
                    format!("int{width}")
                } else {
                    format!("uint{width}")
                }
            }
            IntegerType::Pointer { is_signed } => {
                if is_signed {
                    "isize".to_string()
                } else {
                    "usize".to_string()
                }
            }
        }
    }
}

/// A floating-point type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FloatType {
    /// The floating-point family.
    Float,
    /// 16-bit IEEE-754 binary16 float.
    Float16,
    /// 16-bit bfloat format.
    Bfloat16,
    /// 32-bit IEEE-754 float.
    Float32,
    /// 64-bit IEEE-754 float.
    Float64,
}

impl FloatType {
    /// Return the concrete bit width, if known without target layout.
    pub fn width(&self) -> Option<u16> {
        match self {
            FloatType::Float => None,
            FloatType::Float16 | FloatType::Bfloat16 => Some(16),
            FloatType::Float32 => Some(32),
            FloatType::Float64 => Some(64),
        }
    }

    /// Get the string representation of the float type.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            FloatType::Float => "float",
            FloatType::Float16 => "float16",
            FloatType::Bfloat16 => "bfloat16",
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        }
    }

    /// Round one `f64` value to this float type and back.
    pub fn roundtrip_f64(self, value: f64) -> Option<f64> {
        let format = match self {
            FloatType::Float | FloatType::Float64 => FloatFormat::Float64,
            FloatType::Float16 => FloatFormat::Float16,
            FloatType::Bfloat16 => FloatFormat::Bfloat16,
            FloatType::Float32 => FloatFormat::Float32,
        };

        roundtrip_float(format, value)
    }
}
