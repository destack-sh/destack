use std::cmp::Ordering;
use std::mem;

use destack_core::{float_from_bits, float_to_bits};
use destack_mir as mir;

use crate::Word;
use crate::diagnostic::Error;
use crate::program::{BinaryFloatKernel, ScalarLayout, UnaryFloatKernel};

/// Return one integer scalar layout.
#[inline(always)]
fn int_layout(ty: ScalarLayout) -> Result<(u16, bool), Error> {
    let ScalarLayout::Int { width, is_signed } = ty else {
        return Err(Error::type_mismatch("integer scalar", format!("{ty:?}")));
    };

    Ok((width, is_signed))
}

/// Return one signed integer scalar layout.
#[inline(always)]
fn signed_int_layout(ty: ScalarLayout) -> Result<u16, Error> {
    let ScalarLayout::Int {
        width,
        is_signed: true,
    } = ty
    else {
        return Err(Error::type_mismatch(
            "signed integer scalar",
            format!("{ty:?}"),
        ));
    };

    Ok(width)
}

/// Return one float scalar layout with the expected width.
#[inline(always)]
fn expect_float_width(ty: ScalarLayout, expected_width: u16) -> Result<(), Error> {
    if let ScalarLayout::Float { format } = ty
        && format.width() == expected_width
    {
        return Ok(());
    }

    Err(Error::type_mismatch(
        format!("float{expected_width} scalar"),
        format!("{ty:?}"),
    ))
}

/// Return one boolean scalar layout.
#[inline(always)]
fn expect_bool(ty: ScalarLayout) -> Result<(), Error> {
    if matches!(ty, ScalarLayout::Bool) {
        return Ok(());
    }

    Err(Error::type_mismatch("boolean scalar", format!("{ty:?}")))
}

/// Build one integer word.
#[inline(always)]
fn scalar_integer_word(value: u64, width: u16, is_signed: bool) -> Result<Word, Error> {
    let width = width_u8(width)?;

    Ok(if is_signed {
        Word::int(value as i64, width)
    } else {
        Word::uint(value, width)
    })
}

/// And two boolean values.
#[inline(always)]
pub(crate) fn and_bool(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_bool(ty)?;

    Ok(Word::bool(left.as_bool() && right.as_bool()))
}

/// Or two boolean values.
#[inline(always)]
pub(crate) fn or_bool(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_bool(ty)?;

    Ok(Word::bool(left.as_bool() || right.as_bool()))
}

/// Xor two boolean values.
#[inline(always)]
pub(crate) fn xor_bool(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_bool(ty)?;

    Ok(Word::bool(left.as_bool() ^ right.as_bool()))
}

/// Add two integer values.
#[inline(always)]
pub(crate) fn add_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, is_signed) = int_layout(ty)?;

    scalar_integer_word(left.as_u64().wrapping_add(right.as_u64()), width, is_signed)
}

/// Subtract two integer values.
#[inline(always)]
pub(crate) fn sub_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, is_signed) = int_layout(ty)?;

    scalar_integer_word(left.as_u64().wrapping_sub(right.as_u64()), width, is_signed)
}

/// Multiply two integer values.
#[inline(always)]
pub(crate) fn mul_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, is_signed) = int_layout(ty)?;

    scalar_integer_word(left.as_u64().wrapping_mul(right.as_u64()), width, is_signed)
}

/// Divide two signed integer values.
#[inline(always)]
pub(crate) fn div_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let width = signed_int_layout(ty)?;
    if right.as_i64() == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(Word::int(
        left.as_i64().wrapping_div(right.as_i64()),
        width_u8(width)?,
    ))
}

/// Divide two unsigned integer values.
#[inline(always)]
pub(crate) fn div_uint(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, _) = int_layout(ty)?;
    if right.as_u64() == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(Word::uint(
        left.as_u64().wrapping_div(right.as_u64()),
        width_u8(width)?,
    ))
}

/// Remainder two signed integer values.
#[inline(always)]
pub(crate) fn rem_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let width = signed_int_layout(ty)?;
    if right.as_i64() == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(Word::int(
        left.as_i64().wrapping_rem(right.as_i64()),
        width_u8(width)?,
    ))
}

/// Remainder two unsigned integer values.
#[inline(always)]
pub(crate) fn rem_uint(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, _) = int_layout(ty)?;
    if right.as_u64() == 0 {
        return Err(Error::division_by_zero());
    }

    Ok(Word::uint(
        left.as_u64().wrapping_rem(right.as_u64()),
        width_u8(width)?,
    ))
}

/// And two integer values.
#[inline(always)]
pub(crate) fn and_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, is_signed) = int_layout(ty)?;

    scalar_integer_word(left.as_u64() & right.as_u64(), width, is_signed)
}

/// Or two integer values.
#[inline(always)]
pub(crate) fn or_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, is_signed) = int_layout(ty)?;

    scalar_integer_word(left.as_u64() | right.as_u64(), width, is_signed)
}

/// Xor two integer values.
#[inline(always)]
pub(crate) fn xor_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, is_signed) = int_layout(ty)?;

    scalar_integer_word(left.as_u64() ^ right.as_u64(), width, is_signed)
}

/// Shift one integer value left.
#[inline(always)]
pub(crate) fn shl_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, is_signed) = int_layout(ty)?;

    scalar_integer_word(
        left.as_u64().wrapping_shl(right.as_u64() as u32),
        width,
        is_signed,
    )
}

/// Arithmetically shift one integer value right.
#[inline(always)]
pub(crate) fn shr_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let width = signed_int_layout(ty)?;

    Ok(Word::int(
        left.as_i64().wrapping_shr(right.as_u64() as u32),
        width_u8(width)?,
    ))
}

/// Logically shift one integer value right.
#[inline(always)]
pub(crate) fn shr_uint(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    let (width, _) = int_layout(ty)?;

    Ok(Word::uint(
        left.as_u64().wrapping_shr(right.as_u64() as u32),
        width_u8(width)?,
    ))
}

/// Add two float32 values.
#[inline(always)]
pub(crate) fn add_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::float32(left.as_f32() + right.as_f32()))
}

/// Add two float64 values.
#[inline(always)]
pub(crate) fn add_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::float64(left.as_f64() + right.as_f64()))
}

/// Subtract two float32 values.
#[inline(always)]
pub(crate) fn sub_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::float32(left.as_f32() - right.as_f32()))
}

/// Subtract two float64 values.
#[inline(always)]
pub(crate) fn sub_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::float64(left.as_f64() - right.as_f64()))
}

/// Multiply two float32 values.
#[inline(always)]
pub(crate) fn mul_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::float32(left.as_f32() * right.as_f32()))
}

/// Multiply two float64 values.
#[inline(always)]
pub(crate) fn mul_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::float64(left.as_f64() * right.as_f64()))
}

/// Divide two float32 values.
#[inline(always)]
pub(crate) fn div_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::float32(left.as_f32() / right.as_f32()))
}

/// Divide two float64 values.
#[inline(always)]
pub(crate) fn div_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::float64(left.as_f64() / right.as_f64()))
}

/// Execute one generic binary float operation.
#[inline(always)]
pub(crate) fn binary_float(
    ty: ScalarLayout,
    kernel: BinaryFloatKernel,
    left: Word,
    right: Word,
) -> Result<Word, Error> {
    let ScalarLayout::Float { format } = ty else {
        return Err(Error::type_mismatch("float scalar", format!("{ty:?}")));
    };
    let left = f64_from_float_word(left, format);
    let right = f64_from_float_word(right, format);

    let result = match kernel {
        BinaryFloatKernel::Add => return Ok(float_word_from_f64(left + right, format)),
        BinaryFloatKernel::Subtract => return Ok(float_word_from_f64(left - right, format)),
        BinaryFloatKernel::Multiply => return Ok(float_word_from_f64(left * right, format)),
        BinaryFloatKernel::Divide => return Ok(float_word_from_f64(left / right, format)),
        BinaryFloatKernel::Equal => left == right,
        BinaryFloatKernel::NotEqual => left != right,
        BinaryFloatKernel::LessThan => left < right,
        BinaryFloatKernel::LessEqual => left <= right,
        BinaryFloatKernel::GreaterThan => left > right,
        BinaryFloatKernel::GreaterEqual => left >= right,
    };

    Ok(Word::bool(result))
}

/// Add two generic float values.
#[inline(always)]
pub(crate) fn add_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::Add, left, right)
}

/// Subtract two generic float values.
#[inline(always)]
pub(crate) fn sub_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::Subtract, left, right)
}

/// Multiply two generic float values.
#[inline(always)]
pub(crate) fn mul_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::Multiply, left, right)
}

/// Divide two generic float values.
#[inline(always)]
pub(crate) fn div_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::Divide, left, right)
}

/// Compare generic float values for equality.
#[inline(always)]
pub(crate) fn eq_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::Equal, left, right)
}

/// Compare generic float values for inequality.
#[inline(always)]
pub(crate) fn ne_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::NotEqual, left, right)
}

/// Compare generic float values with less than.
#[inline(always)]
pub(crate) fn lt_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::LessThan, left, right)
}

/// Compare generic float values with less than or equal.
#[inline(always)]
pub(crate) fn le_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::LessEqual, left, right)
}

/// Compare generic float values with greater than.
#[inline(always)]
pub(crate) fn gt_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::GreaterThan, left, right)
}

/// Compare generic float values with greater than or equal.
#[inline(always)]
pub(crate) fn ge_float(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    binary_float(ty, BinaryFloatKernel::GreaterEqual, left, right)
}

/// Compare two integer values for equality.
#[inline(always)]
pub(crate) fn eq_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    int_layout(ty)?;

    Ok(Word::bool(left.as_u64() == right.as_u64()))
}

/// Compare two boolean values for equality.
#[inline(always)]
pub(crate) fn eq_bool(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_bool(ty)?;

    Ok(Word::bool(left.as_bool() == right.as_bool()))
}

/// Compare two integer values for inequality.
#[inline(always)]
pub(crate) fn ne_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    int_layout(ty)?;

    Ok(Word::bool(left.as_u64() != right.as_u64()))
}

/// Compare two boolean values for inequality.
#[inline(always)]
pub(crate) fn ne_bool(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_bool(ty)?;

    Ok(Word::bool(left.as_bool() != right.as_bool()))
}

/// Compare signed integers with less than.
#[inline(always)]
pub(crate) fn lt_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    signed_int_layout(ty)?;

    Ok(Word::bool(left.as_i64() < right.as_i64()))
}

/// Compare unsigned integers with less than.
#[inline(always)]
pub(crate) fn lt_uint(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    int_layout(ty)?;

    Ok(Word::bool(left.as_u64() < right.as_u64()))
}

/// Compare signed integers with less than or equal.
#[inline(always)]
pub(crate) fn le_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    signed_int_layout(ty)?;

    Ok(Word::bool(left.as_i64() <= right.as_i64()))
}

/// Compare unsigned integers with less than or equal.
#[inline(always)]
pub(crate) fn le_uint(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    int_layout(ty)?;

    Ok(Word::bool(left.as_u64() <= right.as_u64()))
}

/// Compare signed integers with greater than.
#[inline(always)]
pub(crate) fn gt_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    signed_int_layout(ty)?;

    Ok(Word::bool(left.as_i64() > right.as_i64()))
}

/// Compare unsigned integers with greater than.
#[inline(always)]
pub(crate) fn gt_uint(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    int_layout(ty)?;

    Ok(Word::bool(left.as_u64() > right.as_u64()))
}

/// Compare signed integers with greater than or equal.
#[inline(always)]
pub(crate) fn ge_int(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    signed_int_layout(ty)?;

    Ok(Word::bool(left.as_i64() >= right.as_i64()))
}

/// Compare unsigned integers with greater than or equal.
#[inline(always)]
pub(crate) fn ge_uint(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    int_layout(ty)?;

    Ok(Word::bool(left.as_u64() >= right.as_u64()))
}

/// Compare float32 values for equality.
#[inline(always)]
pub(crate) fn eq_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::bool(left.as_f32() == right.as_f32()))
}

/// Compare float64 values for equality.
#[inline(always)]
pub(crate) fn eq_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::bool(left.as_f64() == right.as_f64()))
}

/// Compare float32 values for inequality.
#[inline(always)]
pub(crate) fn ne_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::bool(left.as_f32() != right.as_f32()))
}

/// Compare float64 values for inequality.
#[inline(always)]
pub(crate) fn ne_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::bool(left.as_f64() != right.as_f64()))
}

/// Compare float32 values with less than.
#[inline(always)]
pub(crate) fn lt_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::bool(left.as_f32() < right.as_f32()))
}

/// Compare float64 values with less than.
#[inline(always)]
pub(crate) fn lt_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::bool(left.as_f64() < right.as_f64()))
}

/// Compare float32 values with less than or equal.
#[inline(always)]
pub(crate) fn le_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::bool(left.as_f32() <= right.as_f32()))
}

/// Compare float64 values with less than or equal.
#[inline(always)]
pub(crate) fn le_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::bool(left.as_f64() <= right.as_f64()))
}

/// Compare float32 values with greater than.
#[inline(always)]
pub(crate) fn gt_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::bool(left.as_f32() > right.as_f32()))
}

/// Compare float64 values with greater than.
#[inline(always)]
pub(crate) fn gt_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::bool(left.as_f64() > right.as_f64()))
}

/// Compare float32 values with greater than or equal.
#[inline(always)]
pub(crate) fn ge_f32(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::bool(left.as_f32() >= right.as_f32()))
}

/// Compare float64 values with greater than or equal.
#[inline(always)]
pub(crate) fn ge_f64(ty: ScalarLayout, left: Word, right: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::bool(left.as_f64() >= right.as_f64()))
}

/// Negate one float32 value.
#[inline(always)]
pub(crate) fn neg_f32(ty: ScalarLayout, value: Word) -> Result<Word, Error> {
    expect_float_width(ty, 32)?;

    Ok(Word::float32(-value.as_f32()))
}

/// Negate one float64 value.
#[inline(always)]
pub(crate) fn neg_f64(ty: ScalarLayout, value: Word) -> Result<Word, Error> {
    expect_float_width(ty, 64)?;

    Ok(Word::float64(-value.as_f64()))
}

/// Execute one generic unary float operation.
#[inline(always)]
pub(crate) fn unary_float(
    ty: ScalarLayout,
    kernel: UnaryFloatKernel,
    value: Word,
) -> Result<Word, Error> {
    let ScalarLayout::Float { format } = ty else {
        return Err(Error::type_mismatch("float scalar", format!("{ty:?}")));
    };
    let value = f64_from_float_word(value, format);

    let result = match kernel {
        UnaryFloatKernel::Negate => -value,
    };

    Ok(float_word_from_f64(result, format))
}

/// Negate one generic float value.
#[inline(always)]
pub(crate) fn neg_float(ty: ScalarLayout, value: Word) -> Result<Word, Error> {
    unary_float(ty, UnaryFloatKernel::Negate, value)
}

/// Invert one boolean value.
#[inline(always)]
pub(crate) fn not_bool(ty: ScalarLayout, value: Word) -> Result<Word, Error> {
    expect_bool(ty)?;

    Ok(Word::bool(!value.as_bool()))
}

/// Negate one signed integer value.
#[inline(always)]
pub(crate) fn neg_int(ty: ScalarLayout, value: Word) -> Result<Word, Error> {
    let ScalarLayout::Int {
        width,
        is_signed: true,
    } = ty
    else {
        return Err(Error::type_mismatch("signed integer", format!("{ty:?}")));
    };
    let width = width_u8(width)?;

    Ok(Word::int(value.as_i64().wrapping_neg(), width))
}

/// Invert one integer value.
#[inline(always)]
pub(crate) fn not_int(ty: ScalarLayout, value: Word) -> Result<Word, Error> {
    let ScalarLayout::Int { width, is_signed } = ty else {
        return Err(Error::type_mismatch("integer", format!("{ty:?}")));
    };
    let width = width_u8(width)?;

    Ok(if is_signed {
        Word::int(!value.as_i64(), width)
    } else {
        Word::uint(!value.as_u64(), width)
    })
}

/// Compile-time scalar conversion behavior.
trait ScalarConversion {
    /// Whether integer to float conversion must round-trip exactly.
    const CHECK_INT_TO_FLOAT: bool = false;

    /// Whether float narrowing must round-trip exactly.
    const CHECK_FLOAT_NARROWING: bool = false;

    /// Return whether non-finite floats are accepted.
    fn accepts_non_finite() -> bool {
        false
    }

    /// Round one float before converting it to an integer.
    fn round_float_to_int(value: f64) -> Result<f64, Error>;

    /// Round one float before converting it to another float.
    fn round_float_to_float(value: f64) -> f64;

    /// Convert one out-of-range signed integer conversion.
    fn clamp_signed(value: i128, min: i64, max: i64) -> Result<i64, Error> {
        Err(Error::type_mismatch(
            format!("signed range {min}..={max}"),
            value.to_string(),
        ))
    }

    /// Convert one out-of-range unsigned integer conversion.
    fn clamp_unsigned(value: i128, max: u64) -> Result<u64, Error> {
        Err(Error::type_mismatch(
            format!("unsigned range 0..={max}"),
            value.to_string(),
        ))
    }
}

/// Exact scalar conversion.
struct ExactConversion;

impl ScalarConversion for ExactConversion {
    const CHECK_INT_TO_FLOAT: bool = true;
    const CHECK_FLOAT_NARROWING: bool = true;

    fn round_float_to_int(value: f64) -> Result<f64, Error> {
        if value.fract() != 0.0 {
            return Err(Error::type_mismatch("integral float", value.to_string()));
        }

        Ok(value)
    }

    fn round_float_to_float(value: f64) -> f64 {
        value
    }
}

/// Round-to-nearest-even scalar conversion.
struct RoundTiesEvenConversion;

impl ScalarConversion for RoundTiesEvenConversion {
    fn round_float_to_int(value: f64) -> Result<f64, Error> {
        Ok(value.round_ties_even())
    }

    fn round_float_to_float(value: f64) -> f64 {
        value.round_ties_even()
    }
}

/// Round-toward-zero scalar conversion.
struct RoundTowardZeroConversion;

impl ScalarConversion for RoundTowardZeroConversion {
    fn round_float_to_int(value: f64) -> Result<f64, Error> {
        Ok(value.trunc())
    }

    fn round_float_to_float(value: f64) -> f64 {
        value.trunc()
    }
}

/// Round-toward-negative-infinity scalar conversion.
struct RoundFloorConversion;

impl ScalarConversion for RoundFloorConversion {
    fn round_float_to_int(value: f64) -> Result<f64, Error> {
        Ok(value.floor())
    }

    fn round_float_to_float(value: f64) -> f64 {
        value.floor()
    }
}

/// Round-toward-positive-infinity scalar conversion.
struct RoundCeilConversion;

impl ScalarConversion for RoundCeilConversion {
    fn round_float_to_int(value: f64) -> Result<f64, Error> {
        Ok(value.ceil())
    }

    fn round_float_to_float(value: f64) -> f64 {
        value.ceil()
    }
}

/// Saturating scalar conversion.
struct SaturatingConversion;

impl ScalarConversion for SaturatingConversion {
    fn accepts_non_finite() -> bool {
        true
    }

    fn round_float_to_int(value: f64) -> Result<f64, Error> {
        Ok(value.trunc())
    }

    fn round_float_to_float(value: f64) -> f64 {
        value
    }

    fn clamp_signed(value: i128, min: i64, max: i64) -> Result<i64, Error> {
        Ok(if value < i128::from(min) { min } else { max })
    }

    fn clamp_unsigned(value: i128, max: u64) -> Result<u64, Error> {
        Ok(if value < 0 { 0 } else { max })
    }
}

/// Result of evaluating scalar arithmetic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ScalarResult {
    /// One word-sized result.
    Word(Word),
    /// One frame-backed result.
    Bytes(Vec<u8>),
}

/// Return one integer byte scalar layout.
fn byte_integer_layout(ty: ScalarLayout, expected: &'static str) -> Result<(u16, bool), Error> {
    let ScalarLayout::Int { width, is_signed } = ty else {
        return Err(Error::type_mismatch(
            expected.to_string(),
            format!("{ty:?}"),
        ));
    };

    Ok((width, is_signed))
}

/// Return two normalized byte inputs.
fn normalized_byte_inputs(left: &[u8], right: &[u8], width: u16) -> (Vec<u8>, Vec<u8>) {
    let left = normalized_integer_bytes(left, width);
    let right = normalized_integer_bytes(right, width);

    (left, right)
}

/// Add two fixed-width integer byte values.
pub(crate) fn add_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = add_bytes(&left, &right, width);

    Ok(ScalarResult::Bytes(result))
}

/// Subtract two fixed-width integer byte values.
pub(crate) fn subtract_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = subtract_bytes(&left, &right, width);

    Ok(ScalarResult::Bytes(result))
}

/// Multiply two fixed-width integer byte values.
pub(crate) fn multiply_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = multiply_bytes(&left, &right, width);

    Ok(ScalarResult::Bytes(result))
}

/// Divide two signed fixed-width integer byte values.
pub(crate) fn divide_signed_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let (left, right) = normalized_byte_inputs(left, right, width);
    let (quotient, _) = divide_signed_bytes(&left, &right, width)?;

    Ok(ScalarResult::Bytes(quotient))
}

/// Divide two unsigned fixed-width integer byte values.
pub(crate) fn divide_unsigned_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "unsigned integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let (quotient, _) = divide_unsigned_bytes(&left, &right, width)?;

    Ok(ScalarResult::Bytes(quotient))
}

/// Remainder two signed fixed-width integer byte values.
pub(crate) fn remainder_signed_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let (left, right) = normalized_byte_inputs(left, right, width);
    let (_, remainder) = divide_signed_bytes(&left, &right, width)?;

    Ok(ScalarResult::Bytes(remainder))
}

/// Remainder two unsigned fixed-width integer byte values.
pub(crate) fn remainder_unsigned_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "unsigned integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let (_, remainder) = divide_unsigned_bytes(&left, &right, width)?;

    Ok(ScalarResult::Bytes(remainder))
}

/// Compare two fixed-width integer byte values for equality.
pub(crate) fn equal_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = compare_unsigned_bytes(&left, &right).is_eq();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two fixed-width integer byte values for inequality.
pub(crate) fn not_equal_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = !compare_unsigned_bytes(&left, &right).is_eq();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two signed fixed-width integer byte values with less than.
pub(crate) fn less_signed_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = compare_signed_bytes(&left, &right, width).is_lt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two unsigned fixed-width integer byte values with less than.
pub(crate) fn less_unsigned_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "unsigned integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = compare_unsigned_bytes(&left, &right).is_lt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two signed fixed-width integer byte values with less or equal.
pub(crate) fn less_equal_signed_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = !compare_signed_bytes(&left, &right, width).is_gt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two unsigned fixed-width integer byte values with less or equal.
pub(crate) fn less_equal_unsigned_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "unsigned integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = !compare_unsigned_bytes(&left, &right).is_gt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two signed fixed-width integer byte values with greater than.
pub(crate) fn greater_signed_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = compare_signed_bytes(&left, &right, width).is_gt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two unsigned fixed-width integer byte values with greater than.
pub(crate) fn greater_unsigned_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "unsigned integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = compare_unsigned_bytes(&left, &right).is_gt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two signed fixed-width integer byte values with greater or equal.
pub(crate) fn greater_equal_signed_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = !compare_signed_bytes(&left, &right, width).is_lt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// Compare two unsigned fixed-width integer byte values with greater or equal.
pub(crate) fn greater_equal_unsigned_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "unsigned integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = !compare_unsigned_bytes(&left, &right).is_lt();

    Ok(ScalarResult::Word(Word::bool(result)))
}

/// And two fixed-width integer byte values.
pub(crate) fn and_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = bitwise_bytes(&left, &right, |left, right| left & right, width);

    Ok(ScalarResult::Bytes(result))
}

/// Or two fixed-width integer byte values.
pub(crate) fn or_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = bitwise_bytes(&left, &right, |left, right| left | right, width);

    Ok(ScalarResult::Bytes(result))
}

/// Xor two fixed-width integer byte values.
pub(crate) fn xor_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = bitwise_bytes(&left, &right, |left, right| left ^ right, width);

    Ok(ScalarResult::Bytes(result))
}

/// Shift one fixed-width integer byte value left.
pub(crate) fn shift_left_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = shift_left_bytes(&left, shift_amount(&right), width);

    Ok(ScalarResult::Bytes(result))
}

/// Arithmetically shift one fixed-width integer byte value right.
pub(crate) fn shift_right_signed_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let (left, right) = normalized_byte_inputs(left, right, width);
    let fill = integer_is_negative(&left, width);
    let result = shift_right_bytes(&left, shift_amount(&right), fill, width);

    Ok(ScalarResult::Bytes(result))
}

/// Logically shift one fixed-width integer byte value right.
pub(crate) fn shift_right_unsigned_bytes_value(
    ty: ScalarLayout,
    left: &[u8],
    right: &[u8],
) -> Result<ScalarResult, Error> {
    let (width, _) = byte_integer_layout(ty, "unsigned integer byte scalar")?;
    let (left, right) = normalized_byte_inputs(left, right, width);
    let result = shift_right_bytes(&left, shift_amount(&right), false, width);

    Ok(ScalarResult::Bytes(result))
}

/// Negate one fixed-width integer byte value.
pub(crate) fn negate_bytes_value(ty: ScalarLayout, value: &[u8]) -> Result<Vec<u8>, Error> {
    let (width, is_signed) = byte_integer_layout(ty, "signed integer byte scalar")?;
    if !is_signed {
        return Err(Error::type_mismatch(
            "signed integer byte scalar",
            format!("{ty:?}"),
        ));
    }

    let value = normalized_integer_bytes(value, width);
    let result = negate_bytes(&value, width);

    Ok(result)
}

/// Invert one fixed-width integer byte value.
pub(crate) fn not_bytes_value(ty: ScalarLayout, value: &[u8]) -> Result<Vec<u8>, Error> {
    let (width, _) = byte_integer_layout(ty, "integer byte scalar")?;
    let value = normalized_integer_bytes(value, width);
    let mut result = value.into_iter().map(|byte| !byte).collect::<Vec<_>>();
    mask_unused_integer_bits(&mut result, width);

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
        return Err(Error::division_by_zero());
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

/// Convert a scalar value exactly.
pub(crate) fn convert_scalar_exact(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
) -> Result<Word, Error> {
    convert_scalar_value::<ExactConversion>(value, source, dest)
}

/// Convert a scalar value by rounding to nearest even.
pub(crate) fn convert_scalar_round_ties_even(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
) -> Result<Word, Error> {
    convert_scalar_value::<RoundTiesEvenConversion>(value, source, dest)
}

/// Convert a scalar value by rounding toward zero.
pub(crate) fn convert_scalar_round_toward_zero(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
) -> Result<Word, Error> {
    convert_scalar_value::<RoundTowardZeroConversion>(value, source, dest)
}

/// Convert a scalar value by rounding toward negative infinity.
pub(crate) fn convert_scalar_round_floor(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
) -> Result<Word, Error> {
    convert_scalar_value::<RoundFloorConversion>(value, source, dest)
}

/// Convert a scalar value by rounding toward positive infinity.
pub(crate) fn convert_scalar_round_ceil(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
) -> Result<Word, Error> {
    convert_scalar_value::<RoundCeilConversion>(value, source, dest)
}

/// Convert a scalar value with saturation.
pub(crate) fn convert_scalar_saturate(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
) -> Result<Word, Error> {
    convert_scalar_value::<SaturatingConversion>(value, source, dest)
}

/// Convert a scalar value between numeric types.
fn convert_scalar_value<C>(
    value: Word,
    source: ScalarLayout,
    dest: ScalarLayout,
) -> Result<Word, Error>
where
    C: ScalarConversion,
{
    // short-circuit identical scalar kinds
    if matches!((source, dest), (ScalarLayout::Bool, ScalarLayout::Bool)) {
        return Ok(value);
    }

    // reject boolean numeric conversions
    if matches!(source, ScalarLayout::Bool) || matches!(dest, ScalarLayout::Bool) {
        return Err(Error::type_mismatch(
            "numeric conversion",
            format!("{value:?}"),
        ));
    }

    // convert between numeric kinds
    match (source, dest) {
        (
            ScalarLayout::Int { width, is_signed },
            ScalarLayout::Int {
                width: dest_width,
                is_signed: dest_signed,
            },
        ) => convert_int_to_int::<C>(value, width, is_signed, dest_width, dest_signed),
        (ScalarLayout::Int { width, is_signed }, ScalarLayout::Float { format }) => {
            convert_int_to_float::<C>(value, width, is_signed, format)
        }
        (
            ScalarLayout::Float { format },
            ScalarLayout::Int {
                width: dest_width,
                is_signed,
            },
        ) => convert_float_to_int::<C>(value, format, dest_width, is_signed),
        (
            ScalarLayout::Float { format },
            ScalarLayout::Float {
                format: dest_format,
            },
        ) => convert_float_to_float::<C>(value, format, dest_format),
        _ => Err(Error::type_mismatch(
            "numeric conversion",
            format!("{value:?}"),
        )),
    }
}

/// Convert an integer value to another integer type.
fn convert_int_to_int<C>(
    value: Word,
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
    dest_signed: bool,
) -> Result<Word, Error>
where
    C: ScalarConversion,
{
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
                    let clamped = clamp_or_error_signed::<C>(value, min, max)?;
                    return Ok(Word::int(clamped, dest_width_u8));
                }

                value as i64
            }
        };
        let clamped = clamp_or_error_signed::<C>(i128::from(value), min, max)?;

        Ok(Word::int(clamped, dest_width_u8))
    } else {
        // convert into the unsigned destination domain
        let max = unsigned_max(dest_width);
        let value = match source_value {
            IntValue::Signed(value) => {
                if value < 0 {
                    let clamped = clamp_or_error_unsigned::<C>(-1, max)?;
                    return Ok(Word::uint(clamped, dest_width_u8));
                }

                value as i128
            }
            IntValue::Unsigned(value) => i128::from(value),
        };
        let clamped = clamp_or_error_unsigned::<C>(value, max)?;

        Ok(Word::uint(clamped, dest_width_u8))
    }
}

/// Convert an integer value to a float.
fn convert_int_to_float<C>(
    value: Word,
    source_width: u16,
    source_signed: bool,
    destination: mir::FloatType,
) -> Result<Word, Error>
where
    C: ScalarConversion,
{
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
    if C::CHECK_INT_TO_FLOAT {
        let round_trip = float_from_bits(
            destination.format(),
            float_to_bits(destination.format(), float_value),
        );
        if round_trip != float_value {
            return Err(Error::type_mismatch(
                "exact integer to float conversion",
                float_value.to_string(),
            ));
        }
    }

    // emit destination float
    Ok(float_word_from_f64(float_value, destination))
}

/// Convert a float value to an integer.
fn convert_float_to_int<C>(
    value: Word,
    source: mir::FloatType,
    dest_width: u16,
    dest_signed: bool,
) -> Result<Word, Error>
where
    C: ScalarConversion,
{
    // resolve width information
    let dest_width_u8 = width_u8(dest_width)?;

    // resolve float value
    let float_value = f64_from_float_word(value, source);

    // require finite values for exact conversions
    if !float_value.is_finite() && !C::accepts_non_finite() {
        return Err(Error::type_mismatch(
            "finite float",
            float_value.to_string(),
        ));
    }

    // round according to the requested mode
    let rounded = C::round_float_to_int(float_value)?;

    // convert to destination integer
    if dest_signed {
        // clamp or reject in the signed destination domain
        let (min, max) = signed_bounds(dest_width);
        let value = if rounded.is_finite() {
            rounded as i128
        } else {
            0
        };
        let clamped = clamp_or_error_signed::<C>(value, min, max)?;

        Ok(Word::int(clamped, dest_width_u8))
    } else {
        // clamp or reject in the unsigned destination domain
        let max = unsigned_max(dest_width);
        let value = if rounded.is_finite() {
            rounded as i128
        } else {
            0
        };
        let clamped = clamp_or_error_unsigned::<C>(value, max)?;

        Ok(Word::uint(clamped, dest_width_u8))
    }
}

/// Convert a float value to another float type.
fn convert_float_to_float<C>(
    value: Word,
    source: mir::FloatType,
    destination: mir::FloatType,
) -> Result<Word, Error>
where
    C: ScalarConversion,
{
    // resolve float value
    let float_value = f64_from_float_word(value, source);

    // apply rounding mode for narrowing conversions
    let rounded = C::round_float_to_float(float_value);

    // enforce exactness if requested
    if C::CHECK_FLOAT_NARROWING && destination.width() < source.width() {
        let round_trip = float_from_bits(
            destination.format(),
            float_to_bits(destination.format(), rounded),
        );
        if round_trip != rounded {
            return Err(Error::type_mismatch(
                "exact float conversion",
                rounded.to_string(),
            ));
        }
    }

    // emit destination float
    Ok(float_word_from_f64(rounded, destination))
}

/// Decode one float word as f64.
#[inline(always)]
fn f64_from_float_word(value: Word, format: mir::FloatType) -> f64 {
    float_from_bits(format.format(), value.bits())
}

/// Encode one f64 value as one float word.
#[inline(always)]
fn float_word_from_f64(value: f64, format: mir::FloatType) -> Word {
    Word::from_bits(float_to_bits(format.format(), value))
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

    // require lowered integer widths
    debug_assert!(width > 0);

    // compute min and max
    let shift = width as u32 - 1;
    let max = (1i64 << shift) - 1;
    let min = -(1i64 << shift);

    (min, max)
}

/// Convert a bit width to a u8, erroring on overflow.
pub(crate) fn width_u8(width: u16) -> Result<u8, Error> {
    // convert width and reject oversized values
    u8::try_from(width).map_err(|_| Error::type_mismatch("width <= 255", width.to_string()))
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
fn clamp_or_error_signed<C>(value: i128, min: i64, max: i64) -> Result<i64, Error>
where
    C: ScalarConversion,
{
    // accept values in range
    if value >= i128::from(min) && value <= i128::from(max) {
        return Ok(value as i64);
    }

    C::clamp_signed(value, min, max)
}

/// Apply saturating or exact behavior for unsigned conversions.
fn clamp_or_error_unsigned<C>(value: i128, max: u64) -> Result<u64, Error>
where
    C: ScalarConversion,
{
    // accept values in range
    if value >= 0 && value <= i128::from(max) {
        return Ok(value as u64);
    }

    C::clamp_unsigned(value, max)
}

/// Add two scalar values using the supplied type.
pub(crate) fn reduce_add(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().wrapping_add(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().wrapping_add(b.as_u64()), width))
        }
        ScalarLayout::Float {
            format: mir::FloatType::Float32,
        } => Ok(Word::float32(a.as_f32() + b.as_f32())),
        ScalarLayout::Float {
            format: mir::FloatType::Float64,
        } => Ok(Word::float64(a.as_f64() + b.as_f64())),
        ScalarLayout::Float { .. } => add_float(ty, a, b),
        _ => Err(reduce_type_error("add", a, b)),
    }
}

/// Multiply two scalar values using the supplied type.
pub(crate) fn reduce_multiply(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().wrapping_mul(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().wrapping_mul(b.as_u64()), width))
        }
        ScalarLayout::Float {
            format: mir::FloatType::Float32,
        } => Ok(Word::float32(a.as_f32() * b.as_f32())),
        ScalarLayout::Float {
            format: mir::FloatType::Float64,
        } => Ok(Word::float64(a.as_f64() * b.as_f64())),
        ScalarLayout::Float { .. } => mul_float(ty, a, b),
        _ => Err(reduce_type_error("multiply", a, b)),
    }
}

/// Select the minimum of two scalar values using the supplied type.
pub(crate) fn reduce_min(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().min(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().min(b.as_u64()), width))
        }
        ScalarLayout::Float {
            format: mir::FloatType::Float32,
        } => Ok(Word::float32(a.as_f32().min(b.as_f32()))),
        ScalarLayout::Float {
            format: mir::FloatType::Float64,
        } => Ok(Word::float64(a.as_f64().min(b.as_f64()))),
        ScalarLayout::Float { format } => {
            let a = f64_from_float_word(a, format);
            let b = f64_from_float_word(b, format);

            Ok(float_word_from_f64(a.min(b), format))
        }
        _ => Err(reduce_type_error("min", a, b)),
    }
}

/// Select the maximum of two scalar values using the supplied type.
pub(crate) fn reduce_max(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
    match ty {
        ScalarLayout::Int { width, is_signed } if is_signed => {
            let width = width_u8(width)?;

            Ok(Word::int(a.as_i64().max(b.as_i64()), width))
        }
        ScalarLayout::Int { width, .. } => {
            let width = width_u8(width)?;

            Ok(Word::uint(a.as_u64().max(b.as_u64()), width))
        }
        ScalarLayout::Float {
            format: mir::FloatType::Float32,
        } => Ok(Word::float32(a.as_f32().max(b.as_f32()))),
        ScalarLayout::Float {
            format: mir::FloatType::Float64,
        } => Ok(Word::float64(a.as_f64().max(b.as_f64()))),
        ScalarLayout::Float { format } => {
            let a = f64_from_float_word(a, format);
            let b = f64_from_float_word(b, format);

            Ok(float_word_from_f64(a.max(b), format))
        }
        _ => Err(reduce_type_error("max", a, b)),
    }
}

/// Apply bitwise or logical and to two scalar values.
pub(crate) fn reduce_and(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
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
pub(crate) fn reduce_or(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
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
pub(crate) fn reduce_xor(ty: ScalarLayout, a: Word, b: Word) -> Result<Word, Error> {
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
    Error::type_mismatch(format!("scalar {op} inputs"), format!("{a:?}, {b:?}"))
}
