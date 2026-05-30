use destack_core::{float_from_bits, float_to_bits};
use destack_mir as mir;

use super::scalar::{convert_integer_bytes, integer_bytes_to_word};
use crate::Word;
use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::{
    FloatCast, FloatToIntCast, FrameSelect, Instruction, IntToFloatCast, IntegerCast, PointerCast,
    WideIntegerCast, WordLayout,
};

/// Execute one lowered word cast.
fn execute_word_cast(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    cast: fn(Word, u32) -> Result<Word, Error>,
) -> Result<(), Error> {
    let dest = instruction.a;
    let argument = instruction.b;
    let cast_field = instruction.c;

    // cast the word directly
    let argument = activation.load_word_at(argument);
    let result = cast(argument, cast_field)?;

    // store result
    activation.store_word_at(dest, result);

    Ok(())
}

/// Execute one lowered wide integer cast.
fn cast_integer_bytes(
    source: &[u8],
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
) -> Vec<u8> {
    convert_integer_bytes(source, source_width, source_signed, dest_width)
}

/// Return the byte width of one integer bit width.
#[inline(always)]
fn integer_byte_len(width: u16) -> usize {
    width.div_ceil(8) as usize
}

/// Cast one integer bit pattern into the requested pointer-shaped target type.
fn cast_integer_to_pointer(raw: u64, field: u32) -> Result<Word, Error> {
    let layout = PointerCast::from_field(field).decode()?;

    Ok(layout.decode(raw))
}

/// Truncate a signed integer to a target bit width.
fn truncate_signed(value: i64, width: u8) -> i64 {
    if width >= Word::BIT_LEN {
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

/// Truncate an unsigned integer to a target bit width.
fn truncate_unsigned(value: u64, width: u8) -> u64 {
    if width >= Word::BIT_LEN {
        return value;
    }

    let mask = (1u64 << width) - 1;

    value & mask
}

/// Compute integer bounds for a width and signedness.
fn integer_bounds(width: u8, is_signed: bool) -> Option<(i128, i128)> {
    if width == 0 || width > Word::BIT_LEN {
        return None;
    }

    if is_signed {
        let shift = (width - 1) as u32;
        let min = -(1_i128 << shift);
        let max = (1_i128 << shift) - 1;

        Some((min, max))
    } else {
        let shift = width as u32;
        let max = (1_i128 << shift) - 1;

        Some((0, max))
    }
}

/// Convert a float to an integer when the conversion is in range and finite.
fn float_to_int_checked(value: f64, min_bound: i128, max_bound: i128) -> Option<i128> {
    if !value.is_finite() {
        return None;
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;

    let min_ok = value >= min_float;
    let max_ok = if max_is_rounded_up {
        value < max_float
    } else {
        value <= max_float
    };
    if !min_ok || !max_ok {
        return None;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound || truncated > max_bound {
        return None;
    }

    Some(truncated)
}

/// Convert a float to an integer using saturating semantics.
fn float_to_int_saturating(value: f64, min_bound: i128, max_bound: i128) -> i128 {
    if value.is_nan() {
        return 0;
    }

    if !value.is_finite() {
        return if value.is_sign_negative() {
            min_bound
        } else {
            max_bound
        };
    }

    let min_float = min_bound as f64;
    let max_float = max_bound as f64;
    let max_rounded = max_float.trunc() as i128;
    let max_is_rounded_up = max_rounded > max_bound;
    if value <= min_float {
        return min_bound;
    }

    if max_is_rounded_up {
        if value >= max_float {
            return max_bound;
        }
    } else if value >= max_float {
        return max_bound;
    }

    let truncated = value.trunc() as i128;
    if truncated < min_bound {
        return min_bound;
    }

    if truncated > max_bound {
        return max_bound;
    }

    truncated
}

/// Reinterpret one word value.
fn cast_bitcast(argument: Word, _field: u32) -> Result<Word, Error> {
    Ok(argument)
}

/// Truncate one integer word.
fn cast_truncate(argument: Word, field: u32) -> Result<Word, Error> {
    let (width, is_signed) = IntegerCast::from_field(field).decode();

    Ok(if is_signed {
        Word::int(truncate_signed(argument.as_i64(), width), width)
    } else {
        Word::uint(truncate_unsigned(argument.as_u64(), width), width)
    })
}

/// Zero extend one integer word.
fn cast_zero_extend(argument: Word, field: u32) -> Result<Word, Error> {
    let (width, _) = IntegerCast::from_field(field).decode();

    Ok(Word::uint(argument.as_u64(), width))
}

/// Sign extend one integer word.
fn cast_sign_extend(argument: Word, field: u32) -> Result<Word, Error> {
    let (width, _) = IntegerCast::from_field(field).decode();

    Ok(Word::int(argument.as_i64(), width))
}

/// Convert one float word to a signed integer word.
fn cast_float_to_signed_int(argument: Word, field: u32) -> Result<Word, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::invalid_cast())?;
    let value = float_word_to_f64(argument, source)?;
    let converted = float_to_int_checked(value, min_bound, max_bound)
        .ok_or(Error::bad_conversion_to_integer())?;

    Ok(Word::int(converted as i64, target_width))
}

/// Convert one float word to an unsigned integer word.
fn cast_float_to_unsigned_int(argument: Word, field: u32) -> Result<Word, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::invalid_cast())?;
    let value = float_word_to_f64(argument, source)?;
    let converted = float_to_int_checked(value, min_bound, max_bound)
        .ok_or(Error::bad_conversion_to_integer())?;

    Ok(Word::uint(converted as u64, target_width))
}

/// Saturating convert one float word to a signed integer word.
fn cast_float_to_signed_int_saturating(argument: Word, field: u32) -> Result<Word, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::invalid_cast())?;
    let value = float_word_to_f64(argument, source)?;
    let converted = float_to_int_saturating(value, min_bound, max_bound);

    Ok(Word::int(converted as i64, target_width))
}

/// Saturating convert one float word to an unsigned integer word.
fn cast_float_to_unsigned_int_saturating(argument: Word, field: u32) -> Result<Word, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::invalid_cast())?;
    let value = float_word_to_f64(argument, source)?;
    let converted = float_to_int_saturating(value, min_bound, max_bound);

    Ok(Word::uint(converted as u64, target_width))
}

/// Convert one signed integer word to a float word.
fn cast_signed_int_to_float(argument: Word, field: u32) -> Result<Word, Error> {
    let destination = IntToFloatCast::from_field(field).decode()?;

    f64_to_float_word(argument.as_i64() as f64, destination)
}

/// Convert one unsigned integer word to a float word.
fn cast_unsigned_int_to_float(argument: Word, field: u32) -> Result<Word, Error> {
    let destination = IntToFloatCast::from_field(field).decode()?;

    f64_to_float_word(argument.as_u64() as f64, destination)
}

/// Convert one float word.
fn cast_float_convert(argument: Word, field: u32) -> Result<Word, Error> {
    let (source, destination) = FloatCast::from_field(field).decode()?;
    let value = float_word_to_f64(argument, source)?;

    f64_to_float_word(value, destination)
}

/// Convert one pointer word to an integer word.
fn cast_pointer_to_int(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = IntegerCast::from_field(field).decode();

    Ok(Word::uint(argument.as_u64(), target_width))
}

/// Convert one integer word to a pointer word.
fn cast_int_to_pointer(argument: Word, field: u32) -> Result<Word, Error> {
    cast_integer_to_pointer(argument.as_u64(), field)
}

/// Execute bitcast word op.
pub(crate) fn execute_cast_bitcast(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_bitcast)
}

/// Execute truncate word op.
pub(crate) fn execute_cast_truncate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_truncate)
}

/// Execute zero extend word op.
pub(crate) fn execute_cast_zero_extend(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_zero_extend)
}

/// Execute sign extend word op.
pub(crate) fn execute_cast_sign_extend(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_sign_extend)
}

/// Execute float to signed integer word op.
pub(crate) fn execute_cast_float_to_signed_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_float_to_signed_int)
}

/// Execute float to unsigned integer word op.
pub(crate) fn execute_cast_float_to_unsigned_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_float_to_unsigned_int)
}

/// Execute saturating float to signed integer word op.
pub(crate) fn execute_cast_float_to_signed_int_saturating(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_float_to_signed_int_saturating)
}

/// Execute saturating float to unsigned integer word op.
pub(crate) fn execute_cast_float_to_unsigned_int_saturating(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(
        activation,
        instruction,
        cast_float_to_unsigned_int_saturating,
    )
}

/// Execute signed integer to float word op.
pub(crate) fn execute_cast_signed_int_to_float(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_signed_int_to_float)
}

/// Execute unsigned integer to float word op.
pub(crate) fn execute_cast_unsigned_int_to_float(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_unsigned_int_to_float)
}

/// Execute float convert word op.
pub(crate) fn execute_cast_float_convert(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_float_convert)
}

/// Decode one float word as f64.
fn float_word_to_f64(word: Word, layout: WordLayout) -> Result<f64, Error> {
    match layout {
        WordLayout::Float16 => Ok(float_from_bits(
            mir::FloatType::Float16.format(),
            word.bits(),
        )),
        WordLayout::Bfloat16 => Ok(float_from_bits(
            mir::FloatType::Bfloat16.format(),
            word.bits(),
        )),
        WordLayout::Float32 => Ok(word.as_float32() as f64),
        WordLayout::Float64 => Ok(word.as_float64()),
        _ => Err(Error::invalid_cast()),
    }
}

/// Encode one f64 into a float word layout.
fn f64_to_float_word(value: f64, layout: WordLayout) -> Result<Word, Error> {
    match layout {
        WordLayout::Float16 => Ok(Word::from_bits(float_to_bits(
            mir::FloatType::Float16.format(),
            value,
        ))),
        WordLayout::Bfloat16 => Ok(Word::from_bits(float_to_bits(
            mir::FloatType::Bfloat16.format(),
            value,
        ))),
        WordLayout::Float32 => Ok(Word::float32(value as f32)),
        WordLayout::Float64 => Ok(Word::float64(value)),
        _ => Err(Error::invalid_cast()),
    }
}

/// Execute pointer to integer word op.
pub(crate) fn execute_cast_pointer_to_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_pointer_to_int)
}

/// Execute integer to pointer word op.
pub(crate) fn execute_cast_int_to_pointer(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(activation, instruction, cast_int_to_pointer)
}

/// Execute word to wide integer cast op.
pub(crate) fn execute_cast_word_to_wide_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, _) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from word bits into frame bytes
    let source = activation.load_word_at(arg).to_byte_array();
    let result = cast_integer_bytes(&source, source_width, source_signed, dest_width);

    // store result bytes
    activation.store_frame_bytes_at(dest, &result);

    Ok(())
}

/// Execute wide integer to word cast op.
pub(crate) fn execute_cast_wide_int_to_word(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, dest_signed) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from frame bytes into word bits
    let source = activation.frame_bytes_at(arg, integer_byte_len(source_width));
    let bytes = cast_integer_bytes(source, source_width, source_signed, dest_width);
    let result = integer_bytes_to_word(&bytes, dest_width, dest_signed)?;

    // store result word
    activation.store_word_at(dest, result);

    Ok(())
}

/// Execute wide integer cast op.
pub(crate) fn execute_cast_wide_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, _) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from frame bytes into frame bytes
    let source = activation.frame_bytes_at(arg, integer_byte_len(source_width));
    let result = cast_integer_bytes(source, source_width, source_signed, dest_width);

    // store result bytes
    activation.store_frame_bytes_at(dest, &result);

    Ok(())
}

/// Execute word select op.
pub(crate) fn execute_select_word(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let condition = instruction.b;
    let then_value = instruction.c;
    let else_value = instruction.d;

    // select the source word
    let condition = activation.load_word_at(condition).as_bool();
    let source = if condition { then_value } else { else_value };
    let result = activation.load_word_at(source);

    // store result
    activation.store_word_at(dest, result);

    Ok(())
}

/// Execute frame select op.
pub(crate) fn execute_select_frame(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let FrameSelect {
        destination_offset,
        condition_offset,
        then_offset,
        else_offset,
        byte_len,
    } = activation.side::<FrameSelect>(instruction);

    // select the source frame value
    let condition = activation.load_word_at(*condition_offset).as_bool();
    let source_offset = if condition { then_offset } else { else_offset };

    // move the selected frame slot directly
    activation.copy_frame_bytes(*source_offset, *destination_offset, *byte_len);

    Ok(())
}
