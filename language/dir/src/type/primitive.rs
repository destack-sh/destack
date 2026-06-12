use serde::{Deserialize, Serialize};

use destack_core::{FloatFormat, roundtrip_float};

use crate::{LanguageItem, Layout, Niche, StringId};

/// A primitive type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// Boolean type `boolean`.
    Boolean,
    /// Character type `char`.
    Character,
    /// String type `string` (unsized).
    String,
    /// Bigint type `bigint` (unsized).
    Bigint,
    /// Integer type, like `int32`, `uint8`, or `usize`.
    Integer(IntegerType),
    /// Float type, like `float64` or `bfloat16`.
    Float(FloatType),
    /// Symbol type `symbol`.
    Symbol,
    /// Unique symbol type `unique symbol`.
    UniqueSymbol,
}

impl PrimitiveType {
    /// Return whether this primitive widens losslessly into another.
    pub fn widens_to(self, target: PrimitiveType) -> bool {
        match (self, target) {
            // integers widen into wider integers that hold every inhabitant
            (Self::Integer(source), Self::Integer(target)) => source.widens_to(target),
            // fixed integers widen into floats that represent them exactly
            (Self::Integer(source), Self::Float(target)) => source.widens_to_float(target),
            // floats widen into formats with at least their mantissa and exponent
            (Self::Float(source), Self::Float(target)) => {
                let (source_mantissa, source_exponent) = source.shape();
                let (target_mantissa, target_exponent) = target.shape();

                source_mantissa <= target_mantissa && source_exponent <= target_exponent
            }
            _ => false,
        }
    }

    /// Return this primitive's layout, when it has a concrete one.
    pub fn layout(self, pointer_bytes: u32) -> Option<Layout> {
        let layout = match self {
            Self::Boolean => Layout::scalar(
                1,
                1,
                Some(Niche {
                    offset: 0,
                    width: 1,
                    start: 0,
                    end: 1,
                }),
            ),
            Self::Character => Layout::scalar(
                4,
                4,
                Some(Niche {
                    offset: 0,
                    width: 4,
                    start: 0,
                    end: 0x10FFFF,
                }),
            ),
            Self::String | Self::Bigint => return None,
            Self::Integer(integer) => integer.layout(pointer_bytes),
            Self::Float(float) => float.layout(),
            Self::Symbol | Self::UniqueSymbol => Layout::pointer(pointer_bytes, true),
        };

        Some(layout)
    }

    /// Return the language item owning this primitive's members.
    pub fn owner_item(&self) -> Option<LanguageItem> {
        match self {
            Self::String => Some(LanguageItem::String),
            Self::Integer(_) | Self::Float(_) => Some(LanguageItem::Number),
            _ => None,
        }
    }
}

/// The backing representation of an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnumBackingType {
    /// Integer-backed enums, like `enum Status { Ready = 0 }`.
    Integer(IntegerType),
    /// String-backed enums, like `enum Mode { Inline = "inline" }`.
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
    /// The signed or unsigned integer family, `int` or `uint`.
    Integer { is_signed: bool },
    /// A fixed-width signed or unsigned integer, like `int32` or `uint8`.
    Fixed { width: u16, is_signed: bool },
    /// A pointer-sized signed or unsigned integer, `isize` or `usize`.
    Pointer { is_signed: bool },
}

impl IntegerType {
    /// Return whether this integer type widens losslessly into another.
    pub fn widens_to(self, target: IntegerType) -> bool {
        match (self, target) {
            // the arbitrary signed integer type holds every integer
            (_, IntegerType::Integer { is_signed: true }) => true,
            (
                IntegerType::Fixed {
                    width: source_width,
                    is_signed: source_signed,
                },
                IntegerType::Fixed {
                    width: target_width,
                    is_signed: target_signed,
                },
            ) => {
                // same signedness widens by width
                if source_signed == target_signed {
                    source_width <= target_width
                }
                // unsigned widens into strictly wider signed
                else if !source_signed && target_signed {
                    source_width < target_width
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Return whether this integer type is exactly representable in one float type.
    pub fn widens_to_float(self, target: FloatType) -> bool {
        let (mantissa, _) = target.shape();

        match self {
            IntegerType::Fixed { width, is_signed } => {
                // signed types spend one bit on the sign
                let magnitude = if is_signed {
                    u32::from(width) - 1
                } else {
                    u32::from(width)
                };

                magnitude <= mantissa
            }
            IntegerType::Integer { .. } | IntegerType::Pointer { .. } => false,
        }
    }

    /// Return whether one integer literal fits this integer type.
    pub fn fits_literal(self, value: i64) -> bool {
        let value = i128::from(value);

        match self {
            IntegerType::Integer { is_signed } | IntegerType::Pointer { is_signed } => {
                is_signed || value >= 0
            }
            IntegerType::Fixed { width, is_signed } => {
                if width == 0 {
                    return false;
                }
                if is_signed {
                    let limit = 1_i128.checked_shl(u32::from(width - 1));
                    let Some(limit) = limit else {
                        return true;
                    };

                    value >= -limit && value < limit
                } else {
                    let limit = 1_i128.checked_shl(u32::from(width));
                    let Some(limit) = limit else {
                        return value >= 0;
                    };

                    value >= 0 && value < limit
                }
            }
        }
    }

    /// Return this integer's layout.
    pub fn layout(self, pointer_bytes: u32) -> Layout {
        match self {
            IntegerType::Integer { .. } | IntegerType::Pointer { .. } => {
                Layout::scalar(pointer_bytes, pointer_bytes, None)
            }
            IntegerType::Fixed { width, .. } => {
                let bytes = u32::from(width).div_ceil(8).max(1);

                Layout::scalar(bytes, bytes, None)
            }
        }
    }

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
    /// The floating-point family `float`.
    Float,
    /// 16-bit IEEE-754 binary16 float `float16`.
    Float16,
    /// 16-bit bfloat format `bfloat16`.
    Bfloat16,
    /// 32-bit IEEE-754 float `float32`.
    Float32,
    /// 64-bit IEEE-754 float `float64`.
    Float64,
}

impl FloatType {
    /// Return the mantissa and exponent bits of this float type.
    pub fn shape(self) -> (u32, u32) {
        match self {
            FloatType::Float16 => (11, 5),
            FloatType::Bfloat16 => (8, 8),
            FloatType::Float32 => (24, 8),
            FloatType::Float | FloatType::Float64 => (53, 11),
        }
    }

    /// Return whether one integer literal fits this float type exactly.
    pub fn fits_integer_literal(self, value: i64) -> bool {
        let value = value as f64;

        self.roundtrip_f64(value)
            .is_some_and(|rounded| rounded == value)
    }

    /// Return whether one float literal fits this float type exactly.
    pub fn fits_literal(self, value: f64) -> bool {
        self.roundtrip_f64(value)
            .is_some_and(|rounded| rounded == value)
    }

    /// Return this float's layout.
    pub fn layout(self) -> Layout {
        match self {
            FloatType::Float16 | FloatType::Bfloat16 => Layout::scalar(2, 2, None),
            FloatType::Float32 => Layout::scalar(4, 4, None),
            FloatType::Float | FloatType::Float64 => Layout::scalar(8, 8, None),
        }
    }

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
