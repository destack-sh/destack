use destack_mir::{BinaryOperator, Constant, UnaryOperator};

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

/// Try to fold a binary operation on constants.
pub fn fold_binary(operator: BinaryOperator, left: Constant, right: Constant) -> Option<Constant> {
    match (&left, &right) {
        (
            Constant::Int {
                value: l,
                width: lw,
                is_signed: true,
            },
            Constant::Int {
                value: r,
                width: rw,
                is_signed: true,
            },
        ) if lw == rw => fold_binary_signed(*l, *r, *lw, operator),

        (
            Constant::UInt {
                value: l,
                width: lw,
            },
            Constant::UInt {
                value: r,
                width: rw,
            },
        ) if lw == rw => fold_binary_unsigned(*l, *r, *lw, operator),

        (
            Constant::Float {
                bits: lb,
                width: lw,
            },
            Constant::Float {
                bits: rb,
                width: rw,
            },
        ) if lw == rw => fold_binary_float(*lb, *rb, *lw, operator),

        (Constant::Boolean { value: l }, Constant::Boolean { value: r }) => {
            fold_binary_bool(*l, *r, operator)
        }

        _ => None,
    }
}

/// Fold a binary operation on signed integers.
pub fn fold_binary_signed(
    left: i64,
    right: i64,
    width: u8,
    operator: BinaryOperator,
) -> Option<Constant> {
    let result_int = |value: i64| {
        Some(Constant::Int {
            value,
            width,
            is_signed: true,
        })
    };
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    match operator {
        BinaryOperator::Add => result_int(left.wrapping_add(right)),
        BinaryOperator::Subtract => result_int(left.wrapping_sub(right)),
        BinaryOperator::Multiply => result_int(left.wrapping_mul(right)),
        BinaryOperator::SignedDivide => {
            if right != 0 {
                result_int(left.wrapping_div(right))
            } else {
                None
            }
        }
        BinaryOperator::SignedRemainder => {
            if right != 0 {
                result_int(left.wrapping_rem(right))
            } else {
                None
            }
        }
        BinaryOperator::And => result_int(left & right),
        BinaryOperator::Or => result_int(left | right),
        BinaryOperator::Xor => result_int(left ^ right),
        BinaryOperator::ShiftLeft => result_int(left.wrapping_shl(right as u32)),
        BinaryOperator::ArithmeticShiftRight => result_int(left.wrapping_shr(right as u32)),
        BinaryOperator::Equal => result_bool(left == right),
        BinaryOperator::NotEqual => result_bool(left != right),
        BinaryOperator::SignedLessThan => result_bool(left < right),
        BinaryOperator::SignedLessEqual => result_bool(left <= right),
        BinaryOperator::SignedGreaterThan => result_bool(left > right),
        BinaryOperator::SignedGreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on unsigned integers.
pub fn fold_binary_unsigned(
    left: u64,
    right: u64,
    width: u8,
    operator: BinaryOperator,
) -> Option<Constant> {
    let result_uint = |value: u64| Some(Constant::UInt { value, width });
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    match operator {
        BinaryOperator::Add => result_uint(left.wrapping_add(right)),
        BinaryOperator::Subtract => result_uint(left.wrapping_sub(right)),
        BinaryOperator::Multiply => result_uint(left.wrapping_mul(right)),
        BinaryOperator::UnsignedDivide => {
            if right != 0 {
                result_uint(left.wrapping_div(right))
            } else {
                None
            }
        }
        BinaryOperator::UnsignedRemainder => {
            if right != 0 {
                result_uint(left.wrapping_rem(right))
            } else {
                None
            }
        }
        BinaryOperator::And => result_uint(left & right),
        BinaryOperator::Or => result_uint(left | right),
        BinaryOperator::Xor => result_uint(left ^ right),
        BinaryOperator::ShiftLeft => result_uint(left.wrapping_shl(right as u32)),
        BinaryOperator::LogicalShiftRight => result_uint(left.wrapping_shr(right as u32)),
        BinaryOperator::Equal => result_bool(left == right),
        BinaryOperator::NotEqual => result_bool(left != right),
        BinaryOperator::UnsignedLessThan => result_bool(left < right),
        BinaryOperator::UnsignedLessEqual => result_bool(left <= right),
        BinaryOperator::UnsignedGreaterThan => result_bool(left > right),
        BinaryOperator::UnsignedGreaterEqual => result_bool(left >= right),
        _ => None,
    }
}

/// Fold a binary operation on floats.
pub fn fold_binary_float(
    left_bits: u64,
    right_bits: u64,
    width: u8,
    operator: BinaryOperator,
) -> Option<Constant> {
    let result_float = |value: f64| {
        Some(Constant::Float {
            bits: value.to_bits(),
            width,
        })
    };
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    if width == 32 {
        let left = f32::from_bits(left_bits as u32);
        let right = f32::from_bits(right_bits as u32);
        match operator {
            BinaryOperator::FloatAdd => result_float((left + right) as f64),
            BinaryOperator::FloatSubtract => result_float((left - right) as f64),
            BinaryOperator::FloatMultiply => result_float((left * right) as f64),
            BinaryOperator::FloatDivide => result_float((left / right) as f64),
            BinaryOperator::FloatEqual => result_bool(left == right),
            BinaryOperator::FloatNotEqual => result_bool(left != right),
            BinaryOperator::FloatLessThan => result_bool(left < right),
            BinaryOperator::FloatLessEqual => result_bool(left <= right),
            BinaryOperator::FloatGreaterThan => result_bool(left > right),
            BinaryOperator::FloatGreaterEqual => result_bool(left >= right),
            _ => None,
        }
    } else if width == 64 {
        let left = f64::from_bits(left_bits);
        let right = f64::from_bits(right_bits);
        match operator {
            BinaryOperator::FloatAdd => result_float(left + right),
            BinaryOperator::FloatSubtract => result_float(left - right),
            BinaryOperator::FloatMultiply => result_float(left * right),
            BinaryOperator::FloatDivide => result_float(left / right),
            BinaryOperator::FloatEqual => result_bool(left == right),
            BinaryOperator::FloatNotEqual => result_bool(left != right),
            BinaryOperator::FloatLessThan => result_bool(left < right),
            BinaryOperator::FloatLessEqual => result_bool(left <= right),
            BinaryOperator::FloatGreaterThan => result_bool(left > right),
            BinaryOperator::FloatGreaterEqual => result_bool(left >= right),
            _ => None,
        }
    } else {
        None
    }
}

/// Fold a binary operation on booleans.
pub fn fold_binary_bool(left: bool, right: bool, operator: BinaryOperator) -> Option<Constant> {
    let result_bool = |value: bool| Some(Constant::Boolean { value });

    match operator {
        BinaryOperator::And => result_bool(left && right),
        BinaryOperator::Or => result_bool(left || right),
        BinaryOperator::Xor => result_bool(left ^ right),
        BinaryOperator::Equal => result_bool(left == right),
        BinaryOperator::NotEqual => result_bool(left != right),
        _ => None,
    }
}

/// Try to fold a unary operation on a constant.
pub fn fold_unary(operator: UnaryOperator, value: Constant) -> Option<Constant> {
    match (operator, &value) {
        (
            UnaryOperator::Negate,
            Constant::Int {
                value: v,
                width,
                is_signed: true,
            },
        ) => Some(Constant::Int {
            value: v.wrapping_neg(),
            width: *width,
            is_signed: true,
        }),

        (UnaryOperator::FloatNegate, Constant::Float { bits, width }) => {
            if *width == 32 {
                let f = f32::from_bits(*bits as u32);
                Some(Constant::Float {
                    bits: ((-f).to_bits()) as u64,
                    width: 32,
                })
            } else if *width == 64 {
                let f = f64::from_bits(*bits);
                Some(Constant::Float {
                    bits: (-f).to_bits(),
                    width: 64,
                })
            } else {
                None
            }
        }

        (UnaryOperator::Not, Constant::Boolean { value: v }) => {
            Some(Constant::Boolean { value: !v })
        }

        (
            UnaryOperator::Not,
            Constant::Int {
                value: v,
                width,
                is_signed,
            },
        ) => Some(Constant::Int {
            value: !v,
            width: *width,
            is_signed: *is_signed,
        }),

        (UnaryOperator::Not, Constant::UInt { value: v, width }) => Some(Constant::UInt {
            value: !v,
            width: *width,
        }),

        _ => None,
    }
}
