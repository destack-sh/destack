/// A concrete binary floating-point format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatFormat {
    /// IEEE-754 binary16.
    Float16,
    /// Bfloat16.
    Bfloat16,
    /// IEEE-754 binary32.
    Float32,
    /// IEEE-754 binary64.
    Float64,
}

impl FloatFormat {
    /// Return the bit width.
    pub const fn width(self) -> u16 {
        match self {
            FloatFormat::Float16 | FloatFormat::Bfloat16 => 16,
            FloatFormat::Float32 => 32,
            FloatFormat::Float64 => 64,
        }
    }
}

/// Encode one float value as raw bits in a `u64` container.
pub fn float_to_bits(format: FloatFormat, value: f64) -> u64 {
    match format {
        FloatFormat::Float16 => u64::from(f32_to_float16_bits(value as f32)),
        FloatFormat::Bfloat16 => u64::from(f32_to_bfloat16_bits(value as f32)),
        FloatFormat::Float32 => u64::from((value as f32).to_bits()),
        FloatFormat::Float64 => value.to_bits(),
    }
}

/// Decode raw float bits from a `u64` container.
pub fn float_from_bits(format: FloatFormat, bits: u64) -> f64 {
    match format {
        FloatFormat::Float16 => f64::from(float16_bits_to_f32(bits as u16)),
        FloatFormat::Bfloat16 => f64::from(bfloat16_bits_to_f32(bits as u16)),
        FloatFormat::Float32 => f64::from(f32::from_bits(bits as u32)),
        FloatFormat::Float64 => f64::from_bits(bits),
    }
}

/// Round one `f64` through a concrete float format.
pub fn roundtrip_float(format: FloatFormat, value: f64) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }

    Some(float_from_bits(format, float_to_bits(format, value)))
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
