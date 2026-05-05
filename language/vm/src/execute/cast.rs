use super::scalar::{convert_integer_bytes, integer_bytes_to_word};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{FrameSelect, Instruction, Transfer, WordLayout};

const CAST_SIGN_BIT: u32 = 1 << 8;
const WORD_LAYOUT_HEAP_REFERENCE: u32 = 1;
const WORD_LAYOUT_SHARED_HEAP_REFERENCE: u32 = 2;
const WORD_LAYOUT_RAW_POINTER: u32 = 3;
const WORD_LAYOUT_SHARED_RAW_POINTER: u32 = 4;
const WORD_LAYOUT_STACK_POINTER: u32 = 5;
const WORD_LAYOUT_FRAME_POINTER: u32 = 6;
const WORD_LAYOUT_STATIC_POINTER: u32 = 7;
const WORD_LAYOUT_FUNCTION_POINTER: u32 = 8;

/// Return one wide integer cast flag from an instruction field.
fn wide_cast_flags(field: u32) -> (bool, bool) {
    let source_signed = field & (1 << 8) != 0;
    let dest_signed = field & (1 << 9) != 0;

    (source_signed, dest_signed)
}

/// Return one wide integer cast layout from an instruction field.
fn wide_cast_layout(field: u32) -> (u16, u16) {
    let source_width = field as u16;
    let dest_width = (field >> 16) as u16;

    (source_width, dest_width)
}

/// Return one word integer cast layout from an instruction field.
fn integer_cast_layout(field: u32) -> (u8, bool) {
    let width = field as u8;
    let signed = field & CAST_SIGN_BIT != 0;

    (width, signed)
}

/// Return one pointer word layout from an instruction field.
fn pointer_cast_layout(field: u32) -> Result<WordLayout, Error> {
    match field {
        WORD_LAYOUT_HEAP_REFERENCE => Ok(WordLayout::HeapReference),
        WORD_LAYOUT_SHARED_HEAP_REFERENCE => Ok(WordLayout::SharedHeapReference),
        WORD_LAYOUT_RAW_POINTER => Ok(WordLayout::RawPointer),
        WORD_LAYOUT_SHARED_RAW_POINTER => Ok(WordLayout::SharedRawPointer),
        WORD_LAYOUT_STACK_POINTER => Ok(WordLayout::StackPointer),
        WORD_LAYOUT_FRAME_POINTER => Ok(WordLayout::FramePointer),
        WORD_LAYOUT_STATIC_POINTER => Ok(WordLayout::StaticPointer),
        WORD_LAYOUT_FUNCTION_POINTER => Ok(WordLayout::FunctionPointer),
        _ => Err(Error::InvalidCast),
    }
}

/// Execute one lowered word cast.
fn execute_word_cast(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    cast: fn(Word, u32) -> Result<Word, Error>,
) -> Transfer {
    let dest = instruction.a;
    let argument = instruction.b;
    let cast_field = instruction.c;

    // cast the word directly
    let argument = machine.get_word_at(argument);
    let result = match cast(argument, cast_field) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    machine.set_word_at(dest, result);

    Transfer::Continue
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

/// Return frame bytes at one lowered frame offset.
#[inline(always)]
fn frame_bytes_at<'a>(machine: &'a Machine<'_, '_>, offset: u32, byte_len: usize) -> &'a [u8] {
    let address = machine.frame_pointer_at(offset).address() as *const u8;

    unsafe { std::slice::from_raw_parts(address, byte_len) }
}

/// Store bytes at one lowered frame offset.
#[inline(always)]
fn store_frame_bytes_at(machine: &mut Machine<'_, '_>, offset: u32, bytes: &[u8]) {
    let destination = machine.frame_pointer_at(offset).address() as *mut u8;

    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len());
    }
}

/// Cast one integer bit pattern into the requested pointer-shaped target type.
fn cast_integer_to_pointer(raw: u64, field: u32) -> Result<Word, Error> {
    let layout = pointer_cast_layout(field)?;

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
    let (width, is_signed) = integer_cast_layout(field);

    Ok(if is_signed {
        Word::int(truncate_signed(argument.as_i64(), width), width)
    } else {
        Word::uint(truncate_unsigned(argument.as_u64(), width), width)
    })
}

/// Zero extend one integer word.
fn cast_zero_extend(argument: Word, field: u32) -> Result<Word, Error> {
    let (width, _) = integer_cast_layout(field);

    Ok(Word::uint(argument.as_u64(), width))
}

/// Sign extend one integer word.
fn cast_sign_extend(argument: Word, field: u32) -> Result<Word, Error> {
    let (width, _) = integer_cast_layout(field);

    Ok(Word::int(argument.as_i64(), width))
}

/// Convert one float word to a signed integer word.
fn cast_float_to_signed_int(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = integer_cast_layout(field);
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
    let converted = float_to_int_checked(argument.as_f64(), min_bound, max_bound)
        .ok_or(Error::BadConversionToInteger)?;

    Ok(Word::int(converted as i64, target_width))
}

/// Convert one float word to an unsigned integer word.
fn cast_float_to_unsigned_int(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = integer_cast_layout(field);
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
    let converted = float_to_int_checked(argument.as_f64(), min_bound, max_bound)
        .ok_or(Error::BadConversionToInteger)?;

    Ok(Word::uint(converted as u64, target_width))
}

/// Saturating convert one float word to a signed integer word.
fn cast_float_to_signed_int_saturating(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = integer_cast_layout(field);
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::InvalidCast)?;
    let converted = float_to_int_saturating(argument.as_f64(), min_bound, max_bound);

    Ok(Word::int(converted as i64, target_width))
}

/// Saturating convert one float word to an unsigned integer word.
fn cast_float_to_unsigned_int_saturating(argument: Word, field: u32) -> Result<Word, Error> {
    let (target_width, _) = integer_cast_layout(field);
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::InvalidCast)?;
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
    let (target_width, _) = integer_cast_layout(field);

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
) -> Transfer {
    execute_word_cast(machine, instruction, cast_bitcast)
}

/// Execute truncate word op.
pub(crate) fn execute_cast_truncate(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_truncate)
}

/// Execute zero extend word op.
pub(crate) fn execute_cast_zero_extend(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_zero_extend)
}

/// Execute sign extend word op.
pub(crate) fn execute_cast_sign_extend(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_sign_extend)
}

/// Execute float to signed integer word op.
pub(crate) fn execute_cast_float_to_signed_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_float_to_signed_int)
}

/// Execute float to unsigned integer word op.
pub(crate) fn execute_cast_float_to_unsigned_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_float_to_unsigned_int)
}

/// Execute saturating float to signed integer word op.
pub(crate) fn execute_cast_float_to_signed_int_saturating(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_float_to_signed_int_saturating)
}

/// Execute saturating float to unsigned integer word op.
pub(crate) fn execute_cast_float_to_unsigned_int_saturating(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_float_to_unsigned_int_saturating)
}

/// Execute signed integer to float32 word op.
pub(crate) fn execute_cast_signed_int_to_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_signed_int_to_f32)
}

/// Execute signed integer to float64 word op.
pub(crate) fn execute_cast_signed_int_to_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_signed_int_to_f64)
}

/// Execute unsigned integer to float32 word op.
pub(crate) fn execute_cast_unsigned_int_to_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_unsigned_int_to_f32)
}

/// Execute unsigned integer to float64 word op.
pub(crate) fn execute_cast_unsigned_int_to_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_unsigned_int_to_f64)
}

/// Execute float truncate word op.
pub(crate) fn execute_cast_float_truncate(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_float_truncate)
}

/// Execute float extend word op.
pub(crate) fn execute_cast_float_extend(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_float_extend)
}

/// Execute pointer to integer word op.
pub(crate) fn execute_cast_pointer_to_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_pointer_to_int)
}

/// Execute integer to pointer word op.
pub(crate) fn execute_cast_int_to_pointer(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_word_cast(machine, instruction, cast_int_to_pointer)
}

/// Execute word to wide integer cast op.
pub(crate) fn execute_cast_word_to_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let arg = instruction.b;
    let (source_signed, _) = wide_cast_flags(instruction.c);
    let (source_width, dest_width) = wide_cast_layout(instruction.d);

    // cast from word bits into frame bytes
    let source = machine.get_word_at(arg).to_byte_array();
    let result = cast_integer_bytes(&source, source_width, source_signed, dest_width);

    // store result bytes
    store_frame_bytes_at(machine, dest, &result);

    Transfer::Continue
}

/// Execute wide integer to word cast op.
pub(crate) fn execute_cast_wide_int_to_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let arg = instruction.b;
    let (source_signed, dest_signed) = wide_cast_flags(instruction.c);
    let (source_width, dest_width) = wide_cast_layout(instruction.d);

    // cast from frame bytes into word bits
    let source = frame_bytes_at(machine, arg, integer_byte_len(source_width));
    let bytes = cast_integer_bytes(source, source_width, source_signed, dest_width);
    let result = match integer_bytes_to_word(&bytes, dest_width, dest_signed) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };

    // store result word
    machine.set_word_at(dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute wide integer cast op.
pub(crate) fn execute_cast_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let arg = instruction.b;
    let (source_signed, _) = wide_cast_flags(instruction.c);
    let (source_width, dest_width) = wide_cast_layout(instruction.d);

    // cast from frame bytes into frame bytes
    let source = frame_bytes_at(machine, arg, integer_byte_len(source_width));
    let result = cast_integer_bytes(source, source_width, source_signed, dest_width);

    // store result bytes
    store_frame_bytes_at(machine, dest, &result);

    Transfer::Continue
}

/// Execute word select op.
pub(crate) fn execute_select_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let condition = instruction.b;
    let then_value = instruction.c;
    let else_value = instruction.d;

    // select the source word
    let condition = machine.get_word_at(condition).as_bool();
    let source = if condition { then_value } else { else_value };
    let result = machine.get_word_at(source);

    // store result
    machine.set_word_at(dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute frame select op.
pub(crate) fn execute_select_frame(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let FrameSelect {
        destination_offset,
        condition_offset,
        then_offset,
        else_offset,
        byte_len,
    } = machine.side::<FrameSelect>(instruction);

    // select the source frame value
    let condition = machine.get_word_at(*condition_offset).as_bool();
    let source_offset = if condition { then_offset } else { else_offset };

    // move the selected frame slot directly
    machine.copy_frame_bytes(*source_offset, *destination_offset, *byte_len);

    // continue to next instruction
    Transfer::Continue
}
