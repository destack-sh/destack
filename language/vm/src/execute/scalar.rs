use std::cmp::Ordering;
use std::mem;

use super::operator;
use crate::Word;
use crate::diagnostic::Error;
use destack_mir as mir;

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

/// Scalar value layout for typed arithmetic.
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

/// Result of evaluating scalar arithmetic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ScalarResult {
    /// One word-sized result.
    Word(Word),
    /// One frame-backed result.
    Bytes(Vec<u8>),
}

/// Evaluate a binary operator over one fixed-width scalar byte value.
pub(crate) fn binary_bytes(
    ty: ScalarLayout,
    op: mir::BinaryOperator,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let ScalarLayout::Int { width, is_signed } = ty else {
        return Err(Error::TypeMismatch {
            expected: "integer byte scalar".to_string(),
            actual: format!("{ty:?}"),
        });
    };

    let mut left = normalized_integer_bytes(left, width);
    let right = normalized_integer_bytes(right, width);

    use mir::BinaryOperator::*;
    let result = match op {
        Add => ScalarResult::Bytes(add_bytes(&left, &right, width)),
        Subtract => ScalarResult::Bytes(subtract_bytes(&left, &right, width)),
        Multiply => ScalarResult::Bytes(multiply_bytes(&left, &right, width)),
        SignedDivide if is_signed => {
            ScalarResult::Bytes(divide_signed_bytes(&left, &right, width)?.0)
        }
        SignedRemainder if is_signed => {
            ScalarResult::Bytes(divide_signed_bytes(&left, &right, width)?.1)
        }
        UnsignedDivide => ScalarResult::Bytes(divide_unsigned_bytes(&left, &right, width)?.0),
        UnsignedRemainder => ScalarResult::Bytes(divide_unsigned_bytes(&left, &right, width)?.1),
        Equal => ScalarResult::Word(Word::bool(compare_unsigned_bytes(&left, &right).is_eq())),
        NotEqual => ScalarResult::Word(Word::bool(!compare_unsigned_bytes(&left, &right).is_eq())),
        SignedLessThan if is_signed => ScalarResult::Word(Word::bool(
            compare_signed_bytes(&left, &right, width).is_lt(),
        )),
        SignedLessEqual if is_signed => ScalarResult::Word(Word::bool(
            !compare_signed_bytes(&left, &right, width).is_gt(),
        )),
        SignedGreaterThan if is_signed => ScalarResult::Word(Word::bool(
            compare_signed_bytes(&left, &right, width).is_gt(),
        )),
        SignedGreaterEqual if is_signed => ScalarResult::Word(Word::bool(
            !compare_signed_bytes(&left, &right, width).is_lt(),
        )),
        UnsignedLessThan => {
            ScalarResult::Word(Word::bool(compare_unsigned_bytes(&left, &right).is_lt()))
        }
        UnsignedLessEqual => {
            ScalarResult::Word(Word::bool(!compare_unsigned_bytes(&left, &right).is_gt()))
        }
        UnsignedGreaterThan => {
            ScalarResult::Word(Word::bool(compare_unsigned_bytes(&left, &right).is_gt()))
        }
        UnsignedGreaterEqual => {
            ScalarResult::Word(Word::bool(!compare_unsigned_bytes(&left, &right).is_lt()))
        }
        And => ScalarResult::Bytes(bitwise_bytes(
            &left,
            &right,
            |left, right| left & right,
            width,
        )),
        Or => ScalarResult::Bytes(bitwise_bytes(
            &left,
            &right,
            |left, right| left | right,
            width,
        )),
        Xor => ScalarResult::Bytes(bitwise_bytes(
            &left,
            &right,
            |left, right| left ^ right,
            width,
        )),
        ShiftLeft => ScalarResult::Bytes(shift_left_bytes(&left, shift_amount(&right), width)),
        ArithmeticShiftRight if is_signed => {
            let fill = integer_is_negative(&left, width);

            ScalarResult::Bytes(shift_right_bytes(&left, shift_amount(&right), fill, width))
        }
        LogicalShiftRight => {
            left = shift_right_bytes(&left, shift_amount(&right), false, width);

            ScalarResult::Bytes(left)
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("wide integer operator for {op:?}"),
                actual: format!("{left:?}, {right:?}"),
            });
        }
    };

    Ok(result)
}

/// Evaluate a unary operator over one fixed-width scalar byte value.
pub(crate) fn unary_bytes(
    ty: ScalarLayout,
    op: mir::UnaryOperator,
    value: &[u8],
) -> Result<Vec<u8>, Error> {
    let ScalarLayout::Int { width, is_signed } = ty else {
        return Err(Error::TypeMismatch {
            expected: "integer byte scalar".to_string(),
            actual: format!("{ty:?}"),
        });
    };

    let value = normalized_integer_bytes(value, width);
    let result = match op {
        mir::UnaryOperator::Negate if is_signed => negate_bytes(&value, width),
        mir::UnaryOperator::Not => {
            let mut result = value.into_iter().map(|byte| !byte).collect::<Vec<_>>();
            mask_unused_integer_bits(&mut result, width);

            result
        }
        _ => {
            return Err(Error::TypeMismatch {
                expected: format!("wide integer operator for {op:?}"),
                actual: format!("{value:?}"),
            });
        }
    };

    Ok(result)
}

/// Convert one integer byte value to another integer byte value.
pub(crate) fn convert_integer_bytes(
    value: &[u8],
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
) -> Vec<u8> {
    let source = normalized_integer_bytes(value, source_width);
    let byte_len = integer_byte_len(dest_width);
    let fill = if source_signed && integer_is_negative(&source, source_width) {
        0xff
    } else {
        0
    };
    let mut result = vec![fill; byte_len];
    let copied = result.len().min(source.len());

    result[..copied].copy_from_slice(&source[..copied]);
    mask_unused_integer_bits(&mut result, dest_width);

    result
}

/// Decode one fixed-width integer byte value into a VM word.
pub(crate) fn integer_bytes_to_word(
    value: &[u8],
    width: u16,
    is_signed: bool,
) -> Result<Word, Error> {
    let value = normalized_integer_bytes(value, width);
    let fill = if is_signed && integer_is_negative(&value, width) {
        0xff
    } else {
        0
    };
    let mut bytes = [fill; Word::BYTE_LEN];
    let copied = bytes.len().min(value.len());

    bytes[..copied].copy_from_slice(&value[..copied]);

    Ok(if is_signed {
        Word::int(i64::from_le_bytes(bytes), width_u8(width)?)
    } else {
        Word::uint(u64::from_le_bytes(bytes), width_u8(width)?)
    })
}

/// Return one integer width in bytes.
fn integer_byte_len(width: u16) -> usize {
    usize::from(width).div_ceil(8)
}

/// Return a masked little-endian integer byte buffer.
fn normalized_integer_bytes(value: &[u8], width: u16) -> Vec<u8> {
    let byte_len = integer_byte_len(width);
    let mut result = vec![0; byte_len];
    let copied = result.len().min(value.len());

    result[..copied].copy_from_slice(&value[..copied]);
    mask_unused_integer_bits(&mut result, width);

    result
}

/// Clear high bits outside the integer width.
fn mask_unused_integer_bits(value: &mut [u8], width: u16) {
    let extra_bits = usize::from(width) % 8;
    if extra_bits == 0 || value.is_empty() {
        return;
    }

    let mask = (1u16 << extra_bits) as u8 - 1;
    let last = value.len() - 1;

    value[last] &= mask;
}

/// Return whether the fixed-width integer is negative.
fn integer_is_negative(value: &[u8], width: u16) -> bool {
    if width == 0 {
        return false;
    }

    let bit = usize::from(width - 1);
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);

    value.get(byte).is_some_and(|byte| byte & mask != 0)
}

/// Compare two unsigned integer byte buffers.
fn compare_unsigned_bytes(left: &[u8], right: &[u8]) -> Ordering {
    for index in (0..left.len().max(right.len())).rev() {
        let left = left.get(index).copied().unwrap_or(0);
        let right = right.get(index).copied().unwrap_or(0);

        match left.cmp(&right) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }

    Ordering::Equal
}

/// Compare two signed integer byte buffers.
fn compare_signed_bytes(left: &[u8], right: &[u8], width: u16) -> Ordering {
    let left_negative = integer_is_negative(left, width);
    let right_negative = integer_is_negative(right, width);
    if left_negative != right_negative {
        return if left_negative {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    compare_unsigned_bytes(left, right)
}

/// Add two little-endian integer byte buffers.
fn add_bytes(left: &[u8], right: &[u8], width: u16) -> Vec<u8> {
    let mut result = vec![0; left.len()];
    let mut carry = 0u16;

    for index in 0..result.len() {
        let sum = left[index] as u16 + right[index] as u16 + carry;
        result[index] = sum as u8;
        carry = sum >> 8;
    }

    mask_unused_integer_bits(&mut result, width);

    result
}

/// Subtract two little-endian integer byte buffers.
fn subtract_bytes(left: &[u8], right: &[u8], width: u16) -> Vec<u8> {
    let mut result = vec![0; left.len()];
    let mut borrow = 0i16;

    for index in 0..result.len() {
        let diff = left[index] as i16 - right[index] as i16 - borrow;
        result[index] = diff as u8;
        borrow = if diff < 0 { 1 } else { 0 };
    }

    mask_unused_integer_bits(&mut result, width);

    result
}

/// Multiply two little-endian integer byte buffers.
fn multiply_bytes(left: &[u8], right: &[u8], width: u16) -> Vec<u8> {
    let mut wide = vec![0u32; left.len() + right.len()];

    for left_index in 0..left.len() {
        for right_index in 0..right.len() {
            wide[left_index + right_index] += left[left_index] as u32 * right[right_index] as u32;
        }
    }

    for index in 0..wide.len() - 1 {
        let carry = wide[index] >> 8;
        wide[index] &= 0xff;
        wide[index + 1] += carry;
    }

    let mut result = wide
        .into_iter()
        .take(left.len())
        .map(|byte| byte as u8)
        .collect::<Vec<_>>();
    mask_unused_integer_bits(&mut result, width);

    result
}

/// Apply one bitwise operation to two integer byte buffers.
fn bitwise_bytes(left: &[u8], right: &[u8], op: fn(u8, u8) -> u8, width: u16) -> Vec<u8> {
    let mut result = left
        .iter()
        .zip(right)
        .map(|(left, right)| op(*left, *right))
        .collect::<Vec<_>>();
    mask_unused_integer_bits(&mut result, width);

    result
}

/// Negate one fixed-width integer byte buffer.
fn negate_bytes(value: &[u8], width: u16) -> Vec<u8> {
    let mut result = value.iter().map(|byte| !byte).collect::<Vec<_>>();
    let one = {
        let mut one = vec![0; result.len()];
        if let Some(first) = one.first_mut() {
            *first = 1;
        }
        one
    };

    result = add_bytes(&result, &one, width);

    result
}

/// Return the shift amount encoded by one integer byte buffer.
fn shift_amount(value: &[u8]) -> usize {
    let mut result = 0usize;
    let copied = value.len().min(mem::size_of::<usize>());

    for (index, byte) in value.iter().take(copied).enumerate() {
        result |= (*byte as usize) << (index * 8);
    }

    result
}

/// Shift one integer byte buffer left.
fn shift_left_bytes(value: &[u8], shift: usize, width: u16) -> Vec<u8> {
    if shift >= usize::from(width) {
        return vec![0; value.len()];
    }

    let mut result = vec![0; value.len()];
    for bit in 0..usize::from(width) - shift {
        if get_bit(value, bit) {
            set_bit(&mut result, bit + shift);
        }
    }
    mask_unused_integer_bits(&mut result, width);

    result
}

/// Shift one integer byte buffer right.
fn shift_right_bytes(value: &[u8], shift: usize, fill: bool, width: u16) -> Vec<u8> {
    let mut result = if fill {
        let mut result = vec![0xff; value.len()];
        mask_unused_integer_bits(&mut result, width);

        result
    } else {
        vec![0; value.len()]
    };
    if shift >= usize::from(width) {
        return result;
    }

    for bit in shift..usize::from(width) {
        if get_bit(value, bit) {
            set_bit(&mut result, bit - shift);
        } else if fill {
            clear_bit(&mut result, bit - shift);
        }
    }
    mask_unused_integer_bits(&mut result, width);

    result
}

/// Divide two unsigned integer byte buffers.
fn divide_unsigned_bytes(
    left: &[u8],
    right: &[u8],
    width: u16,
) -> Result<(Vec<u8>, Vec<u8>), Error> {
    if right.iter().all(|byte| *byte == 0) {
        return Err(Error::DivisionByZero);
    }

    let mut quotient = vec![0; left.len()];
    let mut remainder = vec![0; left.len()];

    for bit in (0..usize::from(width)).rev() {
        remainder = shift_left_bytes(&remainder, 1, width);
        if get_bit(left, bit) {
            set_bit(&mut remainder, 0);
        }

        if compare_unsigned_bytes(&remainder, right) != Ordering::Less {
            remainder = subtract_bytes(&remainder, right, width);
            set_bit(&mut quotient, bit);
        }
    }

    Ok((quotient, remainder))
}

/// Divide two signed integer byte buffers.
fn divide_signed_bytes(left: &[u8], right: &[u8], width: u16) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let left_negative = integer_is_negative(left, width);
    let right_negative = integer_is_negative(right, width);
    let left_abs = if left_negative {
        negate_bytes(left, width)
    } else {
        left.to_vec()
    };
    let right_abs = if right_negative {
        negate_bytes(right, width)
    } else {
        right.to_vec()
    };

    let (mut quotient, mut remainder) = divide_unsigned_bytes(&left_abs, &right_abs, width)?;
    if left_negative != right_negative {
        quotient = negate_bytes(&quotient, width);
    }
    if left_negative {
        remainder = negate_bytes(&remainder, width);
    }

    Ok((quotient, remainder))
}

/// Read one bit from a little-endian integer byte buffer.
fn get_bit(value: &[u8], bit: usize) -> bool {
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);

    value.get(byte).is_some_and(|byte| byte & mask != 0)
}

/// Set one bit in a little-endian integer byte buffer.
fn set_bit(value: &mut [u8], bit: usize) {
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);
    if let Some(byte) = value.get_mut(byte) {
        *byte |= mask;
    }
}

/// Clear one bit in a little-endian integer byte buffer.
fn clear_bit(value: &mut [u8], bit: usize) {
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);
    if let Some(byte) = value.get_mut(byte) {
        *byte &= !mask;
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

/// Return the scalar value layout for a MIR type.
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

/// Evaluate one reduction operator over two values.
pub(crate) fn reduce_operator(
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

/// Evaluate one typed binary operator over two scalar values.
pub(crate) fn binary_operator(
    ty: ScalarLayout,
    op: mir::BinaryOperator,
    a: Word,
    b: Word,
) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int {
            is_signed: true, ..
        } => operator::evaluate_binary_int(op, a, b),
        ScalarLayout::Int {
            is_signed: false, ..
        } => match op {
            mir::BinaryOperator::Equal => Ok(Word::bool(a.as_u64() == b.as_u64())),
            mir::BinaryOperator::NotEqual => Ok(Word::bool(a.as_u64() != b.as_u64())),
            _ => operator::evaluate_binary_uint(op, a, b),
        },
        ScalarLayout::Float { width: 32 } => operator::evaluate_binary_float32(op, a, b),
        ScalarLayout::Float { width: 64 } => operator::evaluate_binary_float64(op, a, b),
        ScalarLayout::Bool => match op {
            mir::BinaryOperator::Equal => Ok(Word::bool(a.as_bool() == b.as_bool())),
            mir::BinaryOperator::NotEqual => Ok(Word::bool(a.as_bool() != b.as_bool())),
            _ => operator::evaluate_binary_bool(op, a, b),
        },
        _ => Err(reduce_type_error("binary", a, b)),
    }
}

/// Evaluate one typed unary operator over one scalar value.
pub(crate) fn unary_operator(
    ty: ScalarLayout,
    op: mir::UnaryOperator,
    value: Word,
) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Float { width: 32 } => operator::evaluate_unary_float32(op, value),
        ScalarLayout::Float { width: 64 } => operator::evaluate_unary_float64(op, value),
        ScalarLayout::Bool => operator::evaluate_unary_bool(op, value),
        _ => Err(Error::TypeMismatch {
            expected: format!("typed unary operator for {op:?}"),
            actual: format!("{value:?}"),
        }),
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
