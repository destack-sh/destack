use serde::{Deserialize, Serialize};

use crate::StringId;

/// A PrimitiveType is a primitive type node.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// Boolean type.
    Boolean,
    /// Character type.
    Character,
    /// String type (unsized).
    String,
    /// Bigint type (unsized).
    Bigint,
    /// "Number" type (alias).
    Number,
    /// Integer type.
    Int(IntType),
    /// Float type.
    Float(FloatType),
    /// Symbol type.
    Symbol,
    /// Unique symbol type.
    UniqueSymbol,
}

/// The backing representation of an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumBackingType {
    /// Integer-backed enums.
    Int(IntType),
    /// String-backed enums.
    String,
}

/// A resolved enum field value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumFieldValue {
    /// Integer enum value.
    Int(i64),
    /// String enum value.
    String(StringId),
}

/// An IntType represents arbitrary width integer with signedness.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntType {
    /// 8-bit signed integer (range: -2^7 to 2^7-1)
    Int8,
    /// 16-bit signed integer (range: -2^15 to 2^15-1)
    Int16,
    /// 32-bit signed integer (range: -2^31 to 2^31-1)
    Int32,
    /// 64-bit signed integer (range: -2^63 to 2^63-1)
    Int64,
    /// 128-bit signed integer (range: -2^127 to 2^127-1)
    Int128,
    /// 256-bit signed integer (range: -2^255 to 2^255-1)
    Int256,
    /// Pointer sized signed integer.
    Isize,
    /// 8-bit unsigned integer (range: 0 to 2^8-1)
    Uint8,
    /// 16-bit unsigned integer (range: 0 to 2^16-1)
    Uint16,
    /// 32-bit unsigned integer (range: 0 to 2^32-1)
    Uint32,
    /// 64-bit unsigned integer (range: 0 to 2^64-1)
    Uint64,
    /// 128-bit unsigned integer (range: 0 to 2^128-1)
    Uint128,
    /// 256-bit unsigned integer (range: 0 to 2^256-1)
    Uint256,
    /// Pointer sized unsigned integer.
    Usize,
    /// Arbitrary width integer with signedness.
    Arbitrary { width: u16, is_signed: bool },
}

impl IntType {
    /// Try to simplify an arbitrary width integer type to a fixed width integer type.
    pub fn try_simplify(&self) -> Option<IntType> {
        match *self {
            IntType::Arbitrary {
                width,
                is_signed: true,
            } => match width {
                8 => Some(IntType::Int8),
                16 => Some(IntType::Int16),
                32 => Some(IntType::Int32),
                64 => Some(IntType::Int64),
                128 => Some(IntType::Int128),
                256 => Some(IntType::Int256),
                _ => None,
            },
            IntType::Arbitrary {
                width,
                is_signed: false,
            } => match width {
                8 => Some(IntType::Uint8),
                16 => Some(IntType::Uint16),
                32 => Some(IntType::Uint32),
                64 => Some(IntType::Uint64),
                128 => Some(IntType::Uint128),
                256 => Some(IntType::Uint256),
                _ => None,
            },
            _ => None,
        }
    }

    /// Simplify this integer type if possible.
    pub fn simplify(self) -> Self {
        match self.try_simplify() {
            Some(int_type) => int_type,
            None => self,
        }
    }

    /// Get the width of the integer type.
    pub fn width(&self) -> Option<u16> {
        let width = match self {
            IntType::Int8 => 8,
            IntType::Int16 => 16,
            IntType::Int32 => 32,
            IntType::Int64 => 64,
            IntType::Int128 => 128,
            IntType::Int256 => 256,
            IntType::Isize => return None,
            IntType::Uint8 => 8,
            IntType::Uint16 => 16,
            IntType::Uint32 => 32,
            IntType::Uint64 => 64,
            IntType::Uint128 => 128,
            IntType::Uint256 => 256,
            IntType::Usize => return None,
            IntType::Arbitrary {
                width,
                is_signed: _,
            } => *width,
        };
        Some(width)
    }

    /// Whether the integer type is signed.
    pub fn is_signed(&self) -> bool {
        match self {
            IntType::Int8 => true,
            IntType::Int16 => true,
            IntType::Int32 => true,
            IntType::Int64 => true,
            IntType::Int128 => true,
            IntType::Int256 => true,
            IntType::Isize => true,
            IntType::Uint8 => false,
            IntType::Uint16 => false,
            IntType::Uint32 => false,
            IntType::Uint64 => false,
            IntType::Uint128 => false,
            IntType::Uint256 => false,
            IntType::Usize => false,
            IntType::Arbitrary {
                width: _,
                is_signed,
            } => *is_signed,
        }
    }

    /// Get the string representation of the integer type.
    #[inline]
    pub fn as_str(self) -> String {
        match self {
            IntType::Int8 => "int8".to_string(),
            IntType::Int16 => "int16".to_string(),
            IntType::Int32 => "int32".to_string(),
            IntType::Int64 => "int64".to_string(),
            IntType::Int128 => "int128".to_string(),
            IntType::Int256 => "int256".to_string(),
            IntType::Isize => "isize".to_string(),
            IntType::Uint8 => "uint8".to_string(),
            IntType::Uint16 => "uint16".to_string(),
            IntType::Uint32 => "uint32".to_string(),
            IntType::Uint64 => "uint64".to_string(),
            IntType::Uint128 => "uint128".to_string(),
            IntType::Uint256 => "uint256".to_string(),
            IntType::Usize => "usize".to_string(),
            IntType::Arbitrary { width, is_signed } => {
                let mut as_str = if is_signed {
                    "int".to_string()
                } else {
                    "uint".to_string()
                };
                as_str.push_str(&width.to_string());
                as_str
            }
        }
    }
}

/// A FloatType represents IEEE-754 float.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum FloatType {
    /// 32-bit IEEE-754 float.
    Float32,
    /// 64-bit IEEE-754 float.
    Float64,
    /// Arbitrary width IEEE-754 float.
    Arbitrary { width: u16 },
}

impl FloatType {
    /// Try to simplify an arbitrary width float type to a fixed width float type.
    pub fn try_simplify(&self) -> Option<FloatType> {
        match self {
            &FloatType::Arbitrary { width } => match width {
                32 => Some(FloatType::Float32),
                64 => Some(FloatType::Float64),
                _ => None,
            },
            _ => None,
        }
    }

    /// Simplify this float type if possible.
    pub fn simplify(self) -> Self {
        match self.try_simplify() {
            Some(float_type) => float_type,
            None => self,
        }
    }

    /// Get the width of the float type.
    pub fn width(&self) -> u16 {
        match self {
            FloatType::Float32 => 32,
            FloatType::Float64 => 64,
            FloatType::Arbitrary { width } => *width,
        }
    }

    /// Get the string representation of the float type.
    #[inline]
    pub fn as_str(self) -> String {
        match self {
            FloatType::Float32 => "float32".to_string(),
            FloatType::Float64 => "float64".to_string(),
            FloatType::Arbitrary { width } => format!("float{width}"),
        }
    }
}
