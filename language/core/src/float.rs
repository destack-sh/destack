/// A concrete binary floating-point format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatFormat {
    /// IEEE-754 binary32.
    Float32,
    /// IEEE-754 binary64.
    Float64,
}

impl FloatFormat {
    /// Return the bit width.
    pub const fn width(self) -> u16 {
        match self {
            FloatFormat::Float32 => 32,
            FloatFormat::Float64 => 64,
        }
    }
}

/// Encode one float value as raw bits in a `u64` container.
pub fn float_to_bits(format: FloatFormat, value: f64) -> u64 {
    match format {
        FloatFormat::Float32 => u64::from((value as f32).to_bits()),
        FloatFormat::Float64 => value.to_bits(),
    }
}

/// Decode raw float bits from a `u64` container.
pub fn float_from_bits(format: FloatFormat, bits: u64) -> f64 {
    match format {
        FloatFormat::Float32 => f64::from(f32::from_bits(bits as u32)),
        FloatFormat::Float64 => f64::from_bits(bits),
    }
}

/// Round one `f64` through a concrete float format.
pub fn roundtrip_float(format: FloatFormat, value: f64) -> f64 {
    float_from_bits(format, float_to_bits(format, value))
}
