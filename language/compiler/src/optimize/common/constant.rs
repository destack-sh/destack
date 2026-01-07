use destack_mir::Constant;

/// Check if a constant is zero.
pub fn constant_is_zero(constant: Option<&Constant>) -> bool {
    matches!(
        constant,
        Some(Constant::Int { value: 0, .. })
            | Some(Constant::UInt { value: 0, .. })
            | Some(Constant::Boolean { value: false })
    )
}

/// Check if a constant is one.
pub fn constant_is_one(constant: Option<&Constant>) -> bool {
    matches!(
        constant,
        Some(Constant::Int { value: 1, .. })
            | Some(Constant::UInt { value: 1, .. })
            | Some(Constant::Boolean { value: true })
    )
}

/// Check if a constant has all bits set (i.e., -1 for signed, max for unsigned).
pub fn constant_is_all_ones(constant: Option<&Constant>) -> bool {
    match constant {
        Some(Constant::Int { value: -1, .. }) => true,
        Some(Constant::UInt { value, width }) => {
            let mask = if *width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            *value == mask
        }
        Some(Constant::Boolean { value: true }) => true,
        _ => false,
    }
}

/// Check if a float constant is positive zero.
pub fn constant_is_float_zero(constant: Option<&Constant>) -> bool {
    matches!(constant, Some(Constant::Float { bits: 0, .. }))
}

/// Check if a float constant is one.
pub fn constant_is_float_one(constant: Option<&Constant>) -> bool {
    match constant {
        Some(Constant::Float { bits, width: 32 }) => f32::from_bits(*bits as u32) == 1.0,
        Some(Constant::Float { bits, width: 64 }) => f64::from_bits(*bits) == 1.0,
        _ => false,
    }
}

/// Create a zero constant matching the given constant's type.
pub fn constant_zero_like(template: &Constant) -> Constant {
    match template {
        Constant::Int {
            width, is_signed, ..
        } => Constant::Int {
            value: 0,
            width: *width,
            is_signed: *is_signed,
        },
        Constant::UInt { width, .. } => Constant::UInt {
            value: 0,
            width: *width,
        },
        Constant::Float { width, .. } => Constant::Float {
            bits: 0,
            width: *width,
        },
        Constant::Boolean { .. } => Constant::Boolean { value: false },
        _ => Constant::Int {
            value: 0,
            width: 32,
            is_signed: true,
        },
    }
}

/// Create an all-ones constant matching the given constant's type.
pub fn constant_all_ones_like(template: &Constant) -> Constant {
    match template {
        Constant::Int {
            width, is_signed, ..
        } => Constant::Int {
            value: -1,
            width: *width,
            is_signed: *is_signed,
        },
        Constant::UInt { width, .. } => Constant::UInt {
            value: u64::MAX,
            width: *width,
        },
        Constant::Boolean { .. } => Constant::Boolean { value: true },
        _ => Constant::Int {
            value: -1,
            width: 32,
            is_signed: true,
        },
    }
}
