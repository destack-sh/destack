use serde::{Deserialize, Serialize};

use crate::StringId;

/// A primitive type.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnumBackingType {
    /// Integer-backed enums.
    Integer(IntegerType),
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

/// An integer type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        if !value.is_finite() {
            return None;
        }

        Some(match self {
            FloatType::Float | FloatType::Float64 => value,
            FloatType::Float16 => f64::from(float16_bits_to_f32(f32_to_float16_bits(value as f32))),
            FloatType::Bfloat16 => {
                f64::from(bfloat16_bits_to_f32(f32_to_bfloat16_bits(value as f32)))
            }
            FloatType::Float32 => f64::from(value as f32),
        })
    }
}

/// Round one `f32` to IEEE-754 binary16 bits.
fn f32_to_float16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32;
    let mantissa = bits & 0x7fffff;

    if exponent == 0xff {
        let payload = if mantissa == 0 { 0 } else { 0x0200 };
        return sign | 0x7c00 | payload;
    }

    let half_exponent = exponent - 127 + 15;
    if half_exponent >= 0x1f {
        return sign | 0x7c00;
    }

    if half_exponent <= 0 {
        if half_exponent < -10 {
            return sign;
        }

        let mantissa = mantissa | 0x800000;
        let shift = (14 - half_exponent) as u32;
        let rounded = round_shift_u32(mantissa, shift);

        return sign | rounded as u16;
    }

    let rounded = round_shift_u32(mantissa, 13);
    if rounded == 0x0400 {
        let exponent = half_exponent + 1;
        if exponent >= 0x1f {
            return sign | 0x7c00;
        }

        return sign | ((exponent as u16) << 10);
    }

    sign | ((half_exponent as u16) << 10) | rounded as u16
}

/// Decode IEEE-754 binary16 bits into an `f32`.
fn float16_bits_to_f32(bits: u16) -> f32 {
    let sign = (u32::from(bits & 0x8000)) << 16;
    let exponent = (bits >> 10) & 0x1f;
    let mantissa = u32::from(bits & 0x03ff);

    let float_bits = if exponent == 0 {
        if mantissa == 0 {
            sign
        } else {
            let shift = mantissa.leading_zeros() - 22;
            let exponent = 127 - 15 - shift as i32;
            let mantissa = (mantissa << (shift + 1)) & 0x7fffff;

            sign | ((exponent as u32) << 23) | mantissa
        }
    } else if exponent == 0x1f {
        sign | 0x7f800000 | (mantissa << 13)
    } else {
        let exponent = u32::from(exponent) + 127 - 15;

        sign | (exponent << 23) | (mantissa << 13)
    };

    f32::from_bits(float_bits)
}

/// Round one `f32` to BF16 bits.
fn f32_to_bfloat16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let rounding_bias = ((bits >> 16) & 1) + 0x7fff;

    ((bits.wrapping_add(rounding_bias)) >> 16) as u16
}

/// Decode BF16 bits into an `f32`.
fn bfloat16_bits_to_f32(bits: u16) -> f32 {
    f32::from_bits(u32::from(bits) << 16)
}

/// Shift right with round-to-nearest-even.
fn round_shift_u32(value: u32, shift: u32) -> u32 {
    let shifted = value >> shift;
    let remainder = value & ((1u32 << shift) - 1);
    let halfway = 1u32 << (shift - 1);
    let increment = remainder > halfway || (remainder == halfway && shifted & 1 == 1);

    shifted + u32::from(increment)
}
