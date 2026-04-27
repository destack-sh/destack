use super::operator;
use super::prelude::*;

/// Reduction operators for vector or tensor reductions.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ReduceOperator {
    /// Add values.
    Add,
    /// Multiply values.
    Multiply,
    /// Select the minimum value.
    Min,
    /// Select the maximum value.
    Max,
    /// Apply bitwise or logical and.
    And,
    /// Apply bitwise or logical or.
    Or,
    /// Apply bitwise or logical xor.
    Xor,
}

impl From<mir::VectorReduceOperator> for ReduceOperator {
    fn from(value: mir::VectorReduceOperator) -> Self {
        match value {
            mir::VectorReduceOperator::Add => ReduceOperator::Add,
            mir::VectorReduceOperator::Multiply => ReduceOperator::Multiply,
            mir::VectorReduceOperator::Min => ReduceOperator::Min,
            mir::VectorReduceOperator::Max => ReduceOperator::Max,
            mir::VectorReduceOperator::And => ReduceOperator::And,
            mir::VectorReduceOperator::Or => ReduceOperator::Or,
            mir::VectorReduceOperator::Xor => ReduceOperator::Xor,
        }
    }
}

impl From<mir::TensorReduceOperator> for ReduceOperator {
    fn from(value: mir::TensorReduceOperator) -> Self {
        match value {
            mir::TensorReduceOperator::Add => ReduceOperator::Add,
            mir::TensorReduceOperator::Multiply => ReduceOperator::Multiply,
            mir::TensorReduceOperator::Min => ReduceOperator::Min,
            mir::TensorReduceOperator::Max => ReduceOperator::Max,
            mir::TensorReduceOperator::And => ReduceOperator::And,
            mir::TensorReduceOperator::Or => ReduceOperator::Or,
            mir::TensorReduceOperator::Xor => ReduceOperator::Xor,
        }
    }
}

/// Scalar value representation for typed arithmetic.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ScalarLayout {
    /// Signed or unsigned integers with a bit width.
    Int {
        /// The bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Floating-point values with a bit width.
    Float {
        /// The bit width.
        width: u16,
    },
    /// Boolean values.
    Bool,
}

/// Conversion modes for vector or tensor element conversions.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ScalarConvertMode {
    /// Require an exact conversion without rounding or saturation.
    Exact,
    /// Round to nearest, ties to even.
    RoundTiesEven,
    /// Round toward zero.
    RoundTowardZero,
    /// Round toward negative infinity.
    RoundFloor,
    /// Round toward positive infinity.
    RoundCeil,
    /// Clamp overflow to the destination range.
    Saturate,
}

impl From<mir::VectorConvertMode> for ScalarConvertMode {
    fn from(value: mir::VectorConvertMode) -> Self {
        match value {
            mir::VectorConvertMode::Exact => ScalarConvertMode::Exact,
            mir::VectorConvertMode::RoundTiesEven => ScalarConvertMode::RoundTiesEven,
            mir::VectorConvertMode::RoundTowardZero => ScalarConvertMode::RoundTowardZero,
            mir::VectorConvertMode::RoundFloor => ScalarConvertMode::RoundFloor,
            mir::VectorConvertMode::RoundCeil => ScalarConvertMode::RoundCeil,
            mir::VectorConvertMode::Saturate => ScalarConvertMode::Saturate,
        }
    }
}

impl From<mir::TensorConvertMode> for ScalarConvertMode {
    fn from(value: mir::TensorConvertMode) -> Self {
        match value {
            mir::TensorConvertMode::Exact => ScalarConvertMode::Exact,
            mir::TensorConvertMode::RoundTiesEven => ScalarConvertMode::RoundTiesEven,
            mir::TensorConvertMode::RoundTowardZero => ScalarConvertMode::RoundTowardZero,
            mir::TensorConvertMode::RoundFloor => ScalarConvertMode::RoundFloor,
            mir::TensorConvertMode::RoundCeil => ScalarConvertMode::RoundCeil,
            mir::TensorConvertMode::Saturate => ScalarConvertMode::Saturate,
        }
    }
}

/// Resolve the scalar value representation from a MIR type.
pub(crate) fn scalar_layout(
    tree: &mir::Tree,
    ty: mir::LocalNodeId<mir::Type>,
) -> Result<ScalarLayout, Error> {
    // resolve scalar types
    match tree.get(ty) {
        mir::Type::Int { width, is_signed } => Ok(ScalarLayout::Int {
            width: *width,
            is_signed: *is_signed,
        }),
        mir::Type::Isize => Ok(ScalarLayout::Int {
            width: usize::BITS as u16,
            is_signed: true,
        }),
        mir::Type::Usize | mir::Type::TypeDescriptor | mir::Type::TypeId => Ok(ScalarLayout::Int {
            width: usize::BITS as u16,
            is_signed: false,
        }),
        mir::Type::Float { width } => Ok(ScalarLayout::Float { width: *width }),
        mir::Type::Boolean => Ok(ScalarLayout::Bool),
        _ => Err(Error::TypeMismatch {
            expected: "scalar type".to_string(),
            actual: format!("{ty:?}"),
        }),
    }
}

/// Convert a scalar value between numeric types.
pub(crate) fn convert_scalar_value(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
    mode: ScalarConvertMode,
) -> Result<Word, Error> {
    // short-circuit identical scalar kinds
    if matches!((source, dest), (ScalarLayout::Bool, ScalarLayout::Bool)) {
        return Ok(value);
    }

    // reject boolean numeric conversions
    if matches!(source, ScalarLayout::Bool) || matches!(dest, ScalarLayout::Bool) {
        return Err(Error::TypeMismatch {
            expected: "numeric conversion".to_string(),
            actual: format!("{value:?}"),
        });
    }

    // convert between numeric kinds
    match (source, dest) {
        (
            ScalarLayout::Int { width, is_signed },
            ScalarLayout::Int {
                width: dest_width,
                is_signed: dest_signed,
            },
        ) => convert_int_to_int(value, width, is_signed, dest_width, dest_signed, mode),
        (ScalarLayout::Int { width, is_signed }, ScalarLayout::Float { width: dest_width }) => {
            convert_int_to_float(value, width, is_signed, dest_width, mode)
        }
        (
            ScalarLayout::Float { width },
            ScalarLayout::Int {
                width: dest_width,
                is_signed,
            },
        ) => convert_float_to_int(value, width, dest_width, is_signed, mode),
        (ScalarLayout::Float { width }, ScalarLayout::Float { width: dest_width }) => {
            convert_float_to_float(value, width, dest_width, mode)
        }
        _ => Err(Error::TypeMismatch {
            expected: "numeric conversion".to_string(),
            actual: format!("{value:?}"),
        }),
    }
}

/// Convert an integer value to another integer type.
pub(crate) fn convert_int_to_int(
    value: Word,
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
    dest_signed: bool,
    mode: ScalarConvertMode,
) -> Result<Word, Error> {
    // resolve width information
    let source_width_u8 = width_u8(source_width)?;
    let dest_width_u8 = width_u8(dest_width)?;

    // resolve source value
    let source_value = if source_signed {
        let value = truncate_signed(value.as_i64(), source_width_u8);

        IntValue::Signed(value)
    } else {
        let value = truncate_unsigned(value.as_u64(), source_width_u8);

        IntValue::Unsigned(value)
    };

    // convert to destination representation
    if dest_signed {
        // convert into the signed destination domain
        let (min, max) = signed_bounds(dest_width);
        let value = match source_value {
            IntValue::Signed(value) => value,
            IntValue::Unsigned(value) => {
                let value = i128::from(value);
                if value > i128::from(i64::MAX) {
                    let clamped = clamp_or_error_signed(value, min, max, mode)?;
                    return Ok(Word::int(clamped, dest_width_u8));
                }

                value as i64
            }
        };
        let clamped = clamp_or_error_signed(i128::from(value), min, max, mode)?;

        Ok(Word::int(clamped, dest_width_u8))
    } else {
        // convert into the unsigned destination domain
        let max = unsigned_max(dest_width);
        let value = match source_value {
            IntValue::Signed(value) => {
                if value < 0 {
                    let clamped = clamp_or_error_unsigned(-1, max, mode)?;
                    return Ok(Word::uint(clamped, dest_width_u8));
                }

                value as i128
            }
            IntValue::Unsigned(value) => i128::from(value),
        };
        let clamped = clamp_or_error_unsigned(value, max, mode)?;

        Ok(Word::uint(clamped, dest_width_u8))
    }
}

/// Convert an integer value to a float.
pub(crate) fn convert_int_to_float(
    value: Word,
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
    mode: ScalarConvertMode,
) -> Result<Word, Error> {
    // resolve width information
    let source_width_u8 = width_u8(source_width)?;

    // resolve integer value
    let int_value = if source_signed {
        let value = truncate_signed(value.as_i64(), source_width_u8);

        IntValue::Signed(value)
    } else {
        let value = truncate_unsigned(value.as_u64(), source_width_u8);

        IntValue::Unsigned(value)
    };

    // convert to float
    let float_value = match int_value {
        IntValue::Signed(value) => value as f64,
        IntValue::Unsigned(value) => value as f64,
    };

    // enforce exactness if requested
    if matches!(mode, ScalarConvertMode::Exact) {
        let round_trip = if dest_width == 32 {
            (float_value as f32) as f64
        } else {
            float_value
        };
        if round_trip != float_value {
            return Err(Error::TypeMismatch {
                expected: "exact integer to float conversion".to_string(),
                actual: float_value.to_string(),
            });
        }
    }

    // emit destination float
    if dest_width == 32 {
        Ok(Word::float32(float_value as f32))
    } else {
        Ok(Word::float64(float_value))
    }
}

/// Convert a float value to an integer.
pub(crate) fn convert_float_to_int(
    value: Word,
    source_width: u16,
    dest_width: u16,
    dest_signed: bool,
    mode: ScalarConvertMode,
) -> Result<Word, Error> {
    // resolve width information
    let dest_width_u8 = width_u8(dest_width)?;

    // resolve float value
    let float_value = match source_width {
        32 => value.as_float32() as f64,
        64 => value.as_float64(),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "float width 32 or 64".to_string(),
                actual: source_width.to_string(),
            });
        }
    };

    // require finite values for exact conversions
    if !float_value.is_finite() && !matches!(mode, ScalarConvertMode::Saturate) {
        return Err(Error::TypeMismatch {
            expected: "finite float".to_string(),
            actual: float_value.to_string(),
        });
    }

    // round according to the requested mode
    let rounded = match mode {
        ScalarConvertMode::Exact => {
            if float_value.fract() != 0.0 {
                return Err(Error::TypeMismatch {
                    expected: "integral float".to_string(),
                    actual: float_value.to_string(),
                });
            }

            float_value
        }
        ScalarConvertMode::RoundTiesEven => float_value.round_ties_even(),
        ScalarConvertMode::RoundTowardZero => float_value.trunc(),
        ScalarConvertMode::RoundFloor => float_value.floor(),
        ScalarConvertMode::RoundCeil => float_value.ceil(),
        ScalarConvertMode::Saturate => float_value.trunc(),
    };

    // convert to destination integer
    if dest_signed {
        // clamp or reject in the signed destination domain
        let (min, max) = signed_bounds(dest_width);
        let value = if rounded.is_finite() {
            rounded as i128
        } else {
            0
        };
        let clamped = clamp_or_error_signed(value, min, max, mode)?;

        Ok(Word::int(clamped, dest_width_u8))
    } else {
        // clamp or reject in the unsigned destination domain
        let max = unsigned_max(dest_width);
        let value = if rounded.is_finite() {
            rounded as i128
        } else {
            0
        };
        let clamped = clamp_or_error_unsigned(value, max, mode)?;

        Ok(Word::uint(clamped, dest_width_u8))
    }
}

/// Convert a float value to another float type.
pub(crate) fn convert_float_to_float(
    value: Word,
    source_width: u16,
    dest_width: u16,
    mode: ScalarConvertMode,
) -> Result<Word, Error> {
    // resolve float value
    let float_value = match source_width {
        32 => value.as_float32() as f64,
        64 => value.as_float64(),
        _ => {
            return Err(Error::TypeMismatch {
                expected: "float width 32 or 64".to_string(),
                actual: source_width.to_string(),
            });
        }
    };

    // apply rounding mode for narrowing conversions
    let rounded = match mode {
        ScalarConvertMode::Exact => float_value,
        ScalarConvertMode::RoundTiesEven => float_value.round_ties_even(),
        ScalarConvertMode::RoundTowardZero => float_value.trunc(),
        ScalarConvertMode::RoundFloor => float_value.floor(),
        ScalarConvertMode::RoundCeil => float_value.ceil(),
        ScalarConvertMode::Saturate => float_value,
    };

    // enforce exactness if requested
    if matches!(mode, ScalarConvertMode::Exact) && dest_width == 32 {
        let round_trip = (rounded as f32) as f64;
        if round_trip != rounded {
            return Err(Error::TypeMismatch {
                expected: "exact float conversion".to_string(),
                actual: rounded.to_string(),
            });
        }
    }

    // emit destination float
    if dest_width == 32 {
        Ok(Word::float32(rounded as f32))
    } else {
        Ok(Word::float64(rounded))
    }
}

/// Resolved integer values for conversions.
enum IntValue {
    /// A signed integer value.
    Signed(i64),
    /// An unsigned integer value.
    Unsigned(u64),
}

/// Compute signed integer bounds for a width.
pub(crate) fn signed_bounds(width: u16) -> (i64, i64) {
    // handle full-width values
    if width >= 64 {
        return (i64::MIN, i64::MAX);
    }

    // compute min and max
    let shift = (width as u32).saturating_sub(1);
    let max = (1i64 << shift) - 1;
    let min = -(1i64 << shift);

    (min, max)
}

/// Convert a bit width to a u8, erroring on overflow.
pub(crate) fn width_u8(width: u16) -> Result<u8, Error> {
    // convert width and reject oversized values
    u8::try_from(width).map_err(|_| Error::TypeMismatch {
        expected: "width <= 255".to_string(),
        actual: width.to_string(),
    })
}

/// Compute the unsigned max for a width.
pub(crate) fn unsigned_max(width: u16) -> u64 {
    // handle full-width values
    if width >= 64 {
        return u64::MAX;
    }

    // compute max
    (1u64 << width) - 1
}

/// Truncate a signed integer to one width.
pub(crate) fn truncate_signed(value: i64, width: u8) -> i64 {
    if width >= u64::BITS as u8 {
        return value;
    }

    let mask = (1u64 << width) - 1;
    let masked = (value as u64) & mask;
    let sign_bit = 1u64 << (width - 1);

    if masked & sign_bit != 0 {
        (masked | !mask) as i64
    } else {
        masked as i64
    }
}

/// Truncate an unsigned integer to one width.
pub(crate) fn truncate_unsigned(value: u64, width: u8) -> u64 {
    if width >= u64::BITS as u8 {
        return value;
    }

    let mask = (1u64 << width) - 1;

    value & mask
}

/// Apply saturating or exact behavior for signed conversions.
pub(crate) fn clamp_or_error_signed(
    value: i128,
    min: i64,
    max: i64,
    mode: ScalarConvertMode,
) -> Result<i64, Error> {
    // accept values in range
    if value >= i128::from(min) && value <= i128::from(max) {
        return Ok(value as i64);
    }

    // saturate or error
    if matches!(mode, ScalarConvertMode::Saturate) {
        return Ok(if value < i128::from(min) { min } else { max });
    }

    Err(Error::TypeMismatch {
        expected: format!("signed range {min}..={max}"),
        actual: value.to_string(),
    })
}

/// Apply saturating or exact behavior for unsigned conversions.
pub(crate) fn clamp_or_error_unsigned(
    value: i128,
    max: u64,
    mode: ScalarConvertMode,
) -> Result<u64, Error> {
    // accept values in range
    if value >= 0 && value <= i128::from(max) {
        return Ok(value as u64);
    }

    // saturate or error
    if matches!(mode, ScalarConvertMode::Saturate) {
        return Ok(if value < 0 { 0 } else { max });
    }

    Err(Error::TypeMismatch {
        expected: format!("unsigned range 0..={max}"),
        actual: value.to_string(),
    })
}

/// Apply a reduction operator to two values.
pub(crate) fn apply_reduce_operator(
    ty: ScalarLayout,
    op: ReduceOperator,
    a: Word,
    b: Word,
) -> Result<Word, Error> {
    match op {
        ReduceOperator::Add => reduce_add(ty, a, b),
        ReduceOperator::Multiply => reduce_multiply(ty, a, b),
        ReduceOperator::Min => reduce_min(ty, a, b),
        ReduceOperator::Max => reduce_max(ty, a, b),
        ReduceOperator::And => reduce_and(ty, a, b),
        ReduceOperator::Or => reduce_or(ty, a, b),
        ReduceOperator::Xor => reduce_xor(ty, a, b),
    }
}

/// Apply a typed binary operator to two scalar values.
pub(crate) fn apply_binary_operator(
    ty: ScalarLayout,
    op: mir::BinaryOperator,
    a: Word,
    b: Word,
) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int {
            is_signed: true, ..
        } => operator::execute_binary_int(op, a, b),
        ScalarLayout::Int {
            is_signed: false, ..
        } => match op {
            mir::BinaryOperator::Equal => Ok(Word::bool(a.as_u64() == b.as_u64())),
            mir::BinaryOperator::NotEqual => Ok(Word::bool(a.as_u64() != b.as_u64())),
            _ => operator::execute_binary_uint(op, a, b),
        },
        ScalarLayout::Float { width: 32 } => operator::execute_binary_float32(op, a, b),
        ScalarLayout::Float { width: 64 } => operator::execute_binary_float64(op, a, b),
        ScalarLayout::Bool => match op {
            mir::BinaryOperator::Equal => Ok(Word::bool(a.as_bool() == b.as_bool())),
            mir::BinaryOperator::NotEqual => Ok(Word::bool(a.as_bool() != b.as_bool())),
            _ => operator::execute_binary_bool(op, a, b),
        },
        _ => Err(reduce_type_error("binary", a, b)),
    }
}

/// Add two scalar values using the supplied type.
fn reduce_add(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().wrapping_add(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().wrapping_add(b.as_u64()), width))
        }
        ScalarLayout::Float { width: 32 } => Ok(Word::float32(a.as_f32() + b.as_f32())),
        ScalarLayout::Float { width: 64 } => Ok(Word::float64(a.as_f64() + b.as_f64())),
        _ => Err(reduce_type_error("add", a, b)),
    }
}

/// Multiply two scalar values using the supplied type.
fn reduce_multiply(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().wrapping_mul(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().wrapping_mul(b.as_u64()), width))
        }
        ScalarLayout::Float { width: 32 } => Ok(Word::float32(a.as_f32() * b.as_f32())),
        ScalarLayout::Float { width: 64 } => Ok(Word::float64(a.as_f64() * b.as_f64())),
        _ => Err(reduce_type_error("multiply", a, b)),
    }
}

/// Select the minimum of two scalar values using the supplied type.
fn reduce_min(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().min(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().min(b.as_u64()), width))
        }
        ScalarLayout::Float { width: 32 } => Ok(Word::float32(a.as_f32().min(b.as_f32()))),
        ScalarLayout::Float { width: 64 } => Ok(Word::float64(a.as_f64().min(b.as_f64()))),
        _ => Err(reduce_type_error("min", a, b)),
    }
}

/// Select the maximum of two scalar values using the supplied type.
fn reduce_max(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().max(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().max(b.as_u64()), width))
        }
        ScalarLayout::Float { width: 32 } => Ok(Word::float32(a.as_f32().max(b.as_f32()))),
        ScalarLayout::Float { width: 64 } => Ok(Word::float64(a.as_f64().max(b.as_f64()))),
        _ => Err(reduce_type_error("max", a, b)),
    }
}

/// Apply bitwise or logical and to two scalar values.
fn reduce_and(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64() & b.as_i64(), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64() & b.as_u64(), width))
        }
        ScalarLayout::Bool => Ok(Word::bool(a.as_bool() && b.as_bool())),
        _ => Err(reduce_type_error("and", a, b)),
    }
}

/// Apply bitwise or logical or to two scalar values.
fn reduce_or(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64() | b.as_i64(), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64() | b.as_u64(), width))
        }
        ScalarLayout::Bool => Ok(Word::bool(a.as_bool() || b.as_bool())),
        _ => Err(reduce_type_error("or", a, b)),
    }
}

/// Apply bitwise or logical xor to two scalar values.
fn reduce_xor(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64() ^ b.as_i64(), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64() ^ b.as_u64(), width))
        }
        ScalarLayout::Bool => Ok(Word::bool(a.as_bool() ^ b.as_bool())),
        _ => Err(reduce_type_error("xor", a, b)),
    }
}

/// Build one scalar reduction type error.
fn reduce_type_error(op: &str, a: Word, b: Word) -> Error {
    Error::TypeMismatch {
        expected: format!("scalar {op} operands"),
        actual: format!("{a:?}, {b:?}"),
    }
}
