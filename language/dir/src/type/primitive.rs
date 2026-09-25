use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use tspp_core::{FloatFormat, roundtrip_float};

use crate::{LanguageItem, Literal, Ownership, RangeType, ScalarDomain, StringId};

/// A primitive type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum PrimitiveType {
    /// Boolean type `boolean`.
    Boolean,
    /// Character type `char`.
    Character,
    /// String type `string`.
    String,
    /// Bigint type `bigint`.
    Bigint,
    /// Integer type, like `int32`, `uint8`, or `usize`.
    Integer(IntegerType),
    /// Float type, like `float32` or `float64`.
    Float(FloatType),
}

impl PrimitiveType {
    /// Return the written source spelling.
    pub fn as_str(self) -> String {
        match self {
            Self::Boolean => "boolean".to_string(),
            Self::String => "string".to_string(),
            Self::Character => "char".to_string(),
            Self::Bigint => "bigint".to_string(),
            Self::Integer(integer) => integer.as_str(),
            Self::Float(float) => float.as_str().to_string(),
        }
    }

    /// Return this primitive's scalar domain.
    pub fn scalar_domain(self) -> ScalarDomain {
        match self {
            Self::Boolean => ScalarDomain::Boolean,
            Self::Character => ScalarDomain::Character,
            Self::String => ScalarDomain::String,
            Self::Bigint => ScalarDomain::Bigint,
            Self::Integer(_) => ScalarDomain::Integer,
            Self::Float(_) => ScalarDomain::Float,
        }
    }

    /// Return whether this primitive is a signed integer.
    pub fn is_signed_integer(self) -> bool {
        matches!(self, Self::Integer(integer) if integer.is_signed())
    }

    /// Return whether this primitive is an unsigned integer.
    pub fn is_unsigned_integer(self) -> bool {
        matches!(self, Self::Integer(integer) if integer.is_unsigned())
    }

    /// Return this primitive's default ownership.
    pub fn ownership(self) -> Ownership {
        match self {
            Self::String | Self::Bigint => Ownership::Managed,
            Self::Boolean | Self::Character | Self::Integer(_) | Self::Float(_) => Ownership::Owned,
        }
    }

    /// Return whether this primitive widens losslessly into another.
    pub fn widens_to(self, target: PrimitiveType) -> bool {
        match (self, target) {
            // integers widen into wider integers that hold every inhabitant
            (Self::Integer(source), Self::Integer(target)) => source.widens_to(target),
            // characters widen into integers that hold every unicode scalar
            (Self::Character, Self::Integer(target)) => {
                IntegerType::CHARACTER_MAGNITUDE.widens_to(target)
            }
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

    /// Return the language item carrying this primitive's runtime representation.
    pub fn representation_item(&self) -> Option<LanguageItem> {
        self.scalar_domain().representation_item()
    }
}

/// One width-less scalar source alias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ScalarAlias {
    /// `int`, the `int64` source alias.
    Int,
    /// `uint`, the `uint64` source alias.
    Uint,
    /// `float`, the `float64` source alias.
    Float,
}

impl ScalarAlias {
    /// Return the sized primitive this alias lowers to.
    pub fn primitive(self) -> PrimitiveType {
        match self {
            Self::Int => PrimitiveType::Integer(IntegerType::Fixed {
                width: 64,
                is_signed: true,
            }),
            Self::Uint => PrimitiveType::Integer(IntegerType::Fixed {
                width: 64,
                is_signed: false,
            }),
            Self::Float => PrimitiveType::Float(FloatType::Float64),
        }
    }

    /// Return the written source spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Uint => "uint",
            Self::Float => "float",
        }
    }
}

/// The backing representation of an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum EnumBackingType {
    /// Integer-backed enums, like `enum Status { Ready = 0 }`.
    Integer(IntegerType),
    /// String-backed enums, like `enum Mode { Inline = "inline" }`.
    String,
}

impl EnumBackingType {
    /// The backing selected for an enum without an explicit scalar domain.
    pub const DEFAULT: Self = Self::Integer(IntegerType::Fixed {
        width: 64,
        is_signed: true,
    });

    /// Return whether this backing type contains one resolved enum value.
    pub fn contains(self, value: EnumVariantValue) -> bool {
        match (self, value) {
            (Self::String, EnumVariantValue::String(_)) => true,
            (Self::Integer(integer), EnumVariantValue::Integer(value)) => {
                integer.fits_literal(value)
            }
            (Self::String, EnumVariantValue::Integer(_))
            | (Self::Integer(_), EnumVariantValue::String(_)) => false,
        }
    }
}

/// A resolved enum variant value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum EnumVariantValue {
    /// Integer enum value.
    Integer(i64),
    /// String enum value.
    String(StringId),
}

impl EnumVariantValue {
    /// Return this value's default enum backing type.
    pub fn default_backing(self) -> EnumBackingType {
        match self {
            Self::Integer(_) => EnumBackingType::DEFAULT,
            Self::String(_) => EnumBackingType::String,
        }
    }

    /// Increment this value for one implicit enum variant.
    pub fn increment(self) -> Result<Self, EnumVariantIncrementError> {
        match self {
            Self::Integer(value) => value
                .checked_add(1)
                .map(Self::Integer)
                .ok_or(EnumVariantIncrementError::Overflow),
            Self::String(_) => Err(EnumVariantIncrementError::ExplicitValueRequired),
        }
    }
}

/// Why an enum value cannot produce the next implicit value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumVariantIncrementError {
    /// String backed variants require explicit values.
    ExplicitValueRequired,
    /// The preceding integer is the largest supported enum value.
    Overflow,
}

impl From<EnumVariantValue> for Literal {
    /// Convert one resolved enum variant value into its scalar literal.
    fn from(value: EnumVariantValue) -> Self {
        match value {
            EnumVariantValue::Integer(value) => Self::Integer(value),
            EnumVariantValue::String(value) => Self::String(value),
        }
    }
}

/// An integer type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum IntegerType {
    /// A fixed-width signed or unsigned integer, like `int32` or `uint8`.
    Fixed { width: u16, is_signed: bool },
    /// A pointer-sized signed or unsigned integer, `isize` or `usize`.
    Pointer { is_signed: bool },
}

impl IntegerType {
    /// The integer type of an unconstrained integer literal in an integer context.
    pub const DEFAULT: Self = Self::Fixed {
        width: 64,
        is_signed: true,
    };

    /// The narrowest width a pointer-sized integer can have on any target.
    pub const MINIMUM_POINTER_WIDTH: u16 = 32;

    /// The widest width a pointer-sized integer can have on any target.
    pub const MAXIMUM_POINTER_WIDTH: u16 = 64;

    /// The unsigned magnitude holding every unicode scalar value.
    pub const CHARACTER_MAGNITUDE: Self = Self::Fixed {
        width: 21,
        is_signed: false,
    };

    /// Return whether this integer type widens losslessly into another.
    pub fn widens_to(self, target: IntegerType) -> bool {
        match (self, target) {
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
            // a pointer-sized target holds at least its narrowest guaranteed width
            (IntegerType::Fixed { .. }, IntegerType::Pointer { is_signed }) => {
                self.widens_to(IntegerType::Fixed {
                    width: Self::MINIMUM_POINTER_WIDTH,
                    is_signed,
                })
            }
            // a pointer-sized source requires room for its widest possible width
            (IntegerType::Pointer { is_signed }, IntegerType::Fixed { .. }) => IntegerType::Fixed {
                width: Self::MAXIMUM_POINTER_WIDTH,
                is_signed,
            }
            .widens_to(target),
            // pointer-sized integers widen only at matching signedness
            (
                IntegerType::Pointer { is_signed: source },
                IntegerType::Pointer { is_signed: target },
            ) => source == target,
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
            IntegerType::Pointer { .. } => false,
        }
    }

    /// Return whether one integer literal fits this integer type.
    pub fn fits_literal(self, value: i64) -> bool {
        let value = i128::from(value);

        match self {
            IntegerType::Pointer { is_signed } => is_signed || value >= 0,
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

    /// Return this integer type as a finite interval, if representable.
    pub fn finite_interval(self) -> Option<RangeType> {
        let (start, end) = match self {
            Self::Fixed {
                width,
                is_signed: true,
            } if (1..=63).contains(&width) => {
                let limit = 1_i64.checked_shl(u32::from(width - 1))?;
                (-limit, limit - 1)
            }
            Self::Fixed {
                width,
                is_signed: false,
            } if (1..=63).contains(&width) => {
                let limit = 1_i64.checked_shl(u32::from(width))?;
                (0, limit - 1)
            }
            Self::Fixed { .. } | Self::Pointer { .. } => return None,
        };

        Some(RangeType {
            start: Some(Literal::Integer(start)),
            end: Some(Literal::Integer(end)),
            is_inclusive: true,
        })
    }

    /// Return the fixed bit width, if known without target layout.
    pub fn width(&self) -> Option<u16> {
        match self {
            IntegerType::Fixed { width, .. } => Some(*width),
            IntegerType::Pointer { .. } => None,
        }
    }

    /// Return the unsigned integer type with the same width.
    pub fn unsigned(self) -> Self {
        match self {
            Self::Fixed { width, .. } => Self::Fixed {
                width,
                is_signed: false,
            },
            Self::Pointer { .. } => Self::Pointer { is_signed: false },
        }
    }

    /// Return whether this integer type is signed.
    pub fn is_signed(&self) -> bool {
        match self {
            IntegerType::Fixed {
                width: _,
                is_signed,
            }
            | IntegerType::Pointer { is_signed } => *is_signed,
        }
    }

    /// Return whether this integer type is unsigned.
    pub fn is_unsigned(&self) -> bool {
        !self.is_signed()
    }

    /// Get the string representation of the integer type.
    #[inline]
    pub fn as_str(self) -> String {
        match self {
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FloatType {
    /// 32-bit IEEE-754 float `float32`.
    Float32,
    /// 64-bit IEEE-754 float `float64`.
    Float64,
}

impl FloatType {
    /// Return the mantissa and exponent bits of this float type.
    pub fn shape(self) -> (u32, u32) {
        match self {
            FloatType::Float32 => (24, 8),
            FloatType::Float64 => (53, 11),
        }
    }

    /// Return whether one integer literal fits this float type exactly.
    pub fn fits_integer_literal(self, value: i64) -> bool {
        let value = value as f64;

        self.roundtrip_f64(value) == value
    }

    /// Return whether one float literal fits this float type exactly.
    pub fn fits_literal(self, value: f64) -> bool {
        let rounded = self.roundtrip_f64(value);

        rounded == value || (rounded.is_nan() && value.is_nan())
    }

    /// Return the concrete bit width.
    pub fn width(self) -> u16 {
        match self {
            FloatType::Float32 => 32,
            FloatType::Float64 => 64,
        }
    }

    /// Get the string representation of the float type.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        }
    }

    /// Round one `f64` value to this float type and back.
    pub fn roundtrip_f64(self, value: f64) -> f64 {
        let format = match self {
            FloatType::Float64 => FloatFormat::Float64,
            FloatType::Float32 => FloatFormat::Float32,
        };

        roundtrip_float(format, value)
    }
}
