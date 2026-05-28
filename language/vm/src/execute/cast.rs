use super::scalar::{convert_integer_bytes, integer_bytes_to_word};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{FrameSelect, Instruction, IntegerCast, PointerCast, WideIntegerCast};

/// Execute one lowered word cast.
fn execute_word_cast(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    cast: fn(Word, u32) -> Result<Word, Error>,
) -> Result<(), Error> {
    let dest = instruction.a;
    let argument = instruction.b;
    let cast_field = instruction.c;

    // cast the word directly
    let argument = machine.load_word_at(argument);
    let result = cast(argument, cast_field)?;

    // store result
    machine.store_word_at(dest, result);

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
    let (target_width, _) = IntegerCast::from_field(field).decode();
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::invalid_cast())?;
    let converted = float_to_int_checked(argument.as_f64(), min_bound, max_bound)
        .ok_or(Error::bad_conversion_to_integer())?;

    Ok(Word::int(converted as i64, target_width))
}

/// Convert one float word to an unsigned integer word.
fn cast_float_to_unsigned_int(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = IntegerCast::from_field(field).decode();
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::invalid_cast())?;
    let converted = float_to_int_checked(argument.as_f64(), min_bound, max_bound)
        .ok_or(Error::bad_conversion_to_integer())?;

    Ok(Word::uint(converted as u64, target_width))
}

/// Saturating convert one float word to a signed integer word.
fn cast_float_to_signed_int_saturating(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = IntegerCast::from_field(field).decode();
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::invalid_cast())?;
    let converted = float_to_int_saturating(argument.as_f64(), min_bound, max_bound);

    Ok(Word::int(converted as i64, target_width))
}

/// Saturating convert one float word to an unsigned integer word.
fn cast_float_to_unsigned_int_saturating(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = IntegerCast::from_field(field).decode();
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::invalid_cast())?;
    let converted = float_to_int_saturating(argument.as_f64(), min_bound, max_bound);

    Ok(Word::uint(converted as u64, target_width))
}

/// Convert one signed integer word to a float32 word.
fn cast_signed_int_to_f32(argument: Word, _field: u32) -> Result<Word, Error> {
    Ok(Word::float32(argument.as_i64() as f32))
}

/// Convert one signed integer word to a float64 word.
fn cast_signed_int_to_f64(argument: Word, _field: u32) -> Result<Word, Error> {
    Ok(Word::float64(argument.as_i64() as f64))
}

/// Convert one unsigned integer word to a float32 word.
fn cast_unsigned_int_to_f32(argument: Word, _field: u32) -> Result<Word, Error> {
    Ok(Word::float32(argument.as_u64() as f32))
}

/// Convert one unsigned integer word to a float64 word.
fn cast_unsigned_int_to_f64(argument: Word, _field: u32) -> Result<Word, Error> {
    Ok(Word::float64(argument.as_u64() as f64))
}

/// Truncate one float word.
fn cast_float_truncate(argument: Word, _field: u32) -> Result<Word, Error> {
    Ok(Word::float32(argument.as_f64() as f32))
}

/// Extend one float word.
fn cast_float_extend(argument: Word, _field: u32) -> Result<Word, Error> {
    Ok(Word::float64(argument.as_f32() as f64))
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
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_bitcast)
}

/// Execute truncate word op.
pub(crate) fn execute_cast_truncate(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_truncate)
}

/// Execute zero extend word op.
pub(crate) fn execute_cast_zero_extend(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_zero_extend)
}

/// Execute sign extend word op.
pub(crate) fn execute_cast_sign_extend(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_sign_extend)
}

/// Execute float to signed integer word op.
pub(crate) fn execute_cast_float_to_signed_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_float_to_signed_int)
}

/// Execute float to unsigned integer word op.
pub(crate) fn execute_cast_float_to_unsigned_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_float_to_unsigned_int)
}

/// Execute saturating float to signed integer word op.
pub(crate) fn execute_cast_float_to_signed_int_saturating(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_float_to_signed_int_saturating)
}

/// Execute saturating float to unsigned integer word op.
pub(crate) fn execute_cast_float_to_unsigned_int_saturating(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_float_to_unsigned_int_saturating)
}

/// Execute signed integer to float32 word op.
pub(crate) fn execute_cast_signed_int_to_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_signed_int_to_f32)
}

/// Execute signed integer to float64 word op.
pub(crate) fn execute_cast_signed_int_to_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_signed_int_to_f64)
}

/// Execute unsigned integer to float32 word op.
pub(crate) fn execute_cast_unsigned_int_to_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_unsigned_int_to_f32)
}

/// Execute unsigned integer to float64 word op.
pub(crate) fn execute_cast_unsigned_int_to_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_unsigned_int_to_f64)
}

/// Execute float truncate word op.
pub(crate) fn execute_cast_float_truncate(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_float_truncate)
}

/// Execute float extend word op.
pub(crate) fn execute_cast_float_extend(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_float_extend)
}

/// Execute pointer to integer word op.
pub(crate) fn execute_cast_pointer_to_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_pointer_to_int)
}

/// Execute integer to pointer word op.
pub(crate) fn execute_cast_int_to_pointer(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_word_cast(machine, instruction, cast_int_to_pointer)
}

/// Execute word to wide integer cast op.
pub(crate) fn execute_cast_word_to_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, _) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from word bits into frame bytes
    let source = machine.load_word_at(arg).to_byte_array();
    let result = cast_integer_bytes(&source, source_width, source_signed, dest_width);

    // store result bytes
    machine.store_frame_bytes_at(dest, &result);

    Ok(())
}

/// Execute wide integer to word cast op.
pub(crate) fn execute_cast_wide_int_to_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, dest_signed) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from frame bytes into word bits
    let source = machine.frame_bytes_at(arg, integer_byte_len(source_width));
    let bytes = cast_integer_bytes(source, source_width, source_signed, dest_width);
    let result = integer_bytes_to_word(&bytes, dest_width, dest_signed)?;

    // store result word
    machine.store_word_at(dest, result);

    Ok(())
}

/// Execute wide integer cast op.
pub(crate) fn execute_cast_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, _) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from frame bytes into frame bytes
    let source = machine.frame_bytes_at(arg, integer_byte_len(source_width));
    let result = cast_integer_bytes(source, source_width, source_signed, dest_width);

    // store result bytes
    machine.store_frame_bytes_at(dest, &result);

    Ok(())
}

/// Execute word select op.
pub(crate) fn execute_select_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let condition = instruction.b;
    let then_value = instruction.c;
    let else_value = instruction.d;

    // select the source word
    let condition = machine.load_word_at(condition).as_bool();
    let source = if condition { then_value } else { else_value };
    let result = machine.load_word_at(source);

    // store result
    machine.store_word_at(dest, result);

    Ok(())
}

/// Execute frame select op.
pub(crate) fn execute_select_frame(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let FrameSelect {
        destination_offset,
        condition_offset,
        then_offset,
        else_offset,
        byte_len,
    } = machine.side::<FrameSelect>(instruction);

    // select the source frame value
    let condition = machine.load_word_at(*condition_offset).as_bool();
    let source_offset = if condition { then_offset } else { else_offset };

    // move the selected frame slot directly
    machine.copy_frame_bytes(*source_offset, *destination_offset, *byte_len);

    Ok(())
}
