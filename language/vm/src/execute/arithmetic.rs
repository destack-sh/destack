use std::ops::{BitAnd, BitOr, BitXor};

use super::scalar::{
    ScalarResult, add_bytes_value, and_bytes_value, divide_signed_bytes_value,
    divide_unsigned_bytes_value, equal_bytes_value, greater_equal_signed_bytes_value,
    greater_equal_unsigned_bytes_value, greater_signed_bytes_value, greater_unsigned_bytes_value,
    less_equal_signed_bytes_value, less_equal_unsigned_bytes_value, less_signed_bytes_value,
    less_unsigned_bytes_value, multiply_bytes_value, negate_bytes_value, not_bytes_value,
    not_equal_bytes_value, or_bytes_value, remainder_signed_bytes_value,
    remainder_unsigned_bytes_value, shift_left_bytes_value, shift_right_signed_bytes_value,
    shift_right_unsigned_bytes_value, subtract_bytes_value, xor_bytes_value,
};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{ConstValue, ConstValueId, Instruction, ScalarLayout, Transfer};

const INTEGER_SIGN_BIT: u32 = 1 << 16;
const INTEGER_WIDTH_MASK: u32 = INTEGER_SIGN_BIT - 1;

/// Rebuild one canonical VM word from integer bits.
#[inline(always)]
fn integer_word<const IS_SIGNED: bool>(raw: u64, width: u8) -> Word {
    if IS_SIGNED {
        return Word::int(raw as i64, width);
    }

    Word::uint(raw, width)
}

/// Rebuild one canonical VM word from an encoded integer layout.
#[inline(always)]
fn integer_word_from_field(raw: u64, field: u32) -> Word {
    let width = integer_width(field);
    let is_signed = field & INTEGER_SIGN_BIT != 0;

    if is_signed {
        return Word::int(raw as i64, width);
    }

    Word::uint(raw, width)
}

/// Unpack one machine integer width.
#[inline(always)]
fn integer_width(field: u32) -> u8 {
    (field & INTEGER_WIDTH_MASK) as u8
}

/// Unpack one wide integer layout.
#[inline(always)]
fn wide_integer_layout(field: u32) -> ScalarLayout {
    let width = (field & INTEGER_WIDTH_MASK) as u16;
    let is_signed = field & INTEGER_SIGN_BIT != 0;

    ScalarLayout::Int { width, is_signed }
}

/// Store one scalar operation result.
fn store_scalar_value(
    machine: &mut Machine<'_, '_>,
    dest: u32,
    value: ScalarResult,
) -> Result<(), Error> {
    match value {
        ScalarResult::Word(value) => machine.set_word_at(dest, value),
        ScalarResult::Bytes(bytes) => {
            let dest = machine.frame_pointer_at(dest).address() as *mut u8;
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
            }
        }
    }

    Ok(())
}

/// Return the byte width of one wide integer layout.
#[inline(always)]
fn wide_integer_byte_len(layout: ScalarLayout) -> usize {
    let ScalarLayout::Int { width, .. } = layout else {
        return 0;
    };

    width.div_ceil(8) as usize
}

/// Return frame bytes at one lowered frame offset.
#[inline(always)]
fn frame_bytes_at<'a>(machine: &'a Machine<'_, '_>, offset: u32, byte_len: usize) -> &'a [u8] {
    let address = machine.frame_pointer_at(offset).address() as *const u8;

    unsafe { std::slice::from_raw_parts(address, byte_len) }
}

/// Load one lowered binary word operation.
#[inline(always)]
fn load_binary_word_values(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (u32, Word, Word) {
    let dest = instruction.a;
    let left = instruction.b;
    let right = instruction.c;

    (dest, machine.get_word_at(left), machine.get_word_at(right))
}

/// Load one lowered binary word operation as raw bits.
#[inline(always)]
fn load_binary_bits(machine: &Machine<'_, '_>, instruction: &Instruction) -> (u32, u64, u64) {
    let dest = instruction.a;
    let left = machine.get_word_at(instruction.b).bits();
    let right = machine.get_word_at(instruction.c).bits();

    (dest, left, right)
}

/// Load one lowered unary word operation.
#[inline(always)]
fn load_unary_word_value(machine: &Machine<'_, '_>, instruction: &Instruction) -> (u32, Word) {
    let dest = instruction.a;
    let argument = instruction.b;

    (dest, machine.get_word_at(argument))
}

/// Load one lowered unary word operation as raw bits.
#[inline(always)]
fn load_unary_bits(machine: &Machine<'_, '_>, instruction: &Instruction) -> (u32, u64) {
    let dest = instruction.a;
    let argument = machine.get_word_at(instruction.b).bits();

    (dest, argument)
}

/// Load one lowered binary integer operation.
#[inline(always)]
fn load_binary_integer_values(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (u32, u64, u64, u8) {
    let dest = instruction.a;
    let left = machine.get_word_at(instruction.b).bits();
    let right = machine.get_word_at(instruction.c).bits();
    let width = integer_width(instruction.d);

    (dest, left, right, width)
}

/// Load one lowered unary integer operation.
#[inline(always)]
fn load_unary_integer_value(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (u32, u64, u8) {
    let dest = instruction.a;
    let argument = machine.get_word_at(instruction.b).bits();
    let width = integer_width(instruction.d);

    (dest, argument, width)
}

/// Execute word constant load.
#[inline(always)]
pub(crate) fn execute_load_const_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let bits = u64::from(instruction.b) | (u64::from(instruction.c) << 32);

    // store constant bits
    machine.set_word_at(dest, Word::from_bits(bits));

    Transfer::Continue
}

/// Execute byte constant load.
#[inline(always)]
pub(crate) fn execute_load_const_bytes(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let value = ConstValueId(instruction.b);

    // copy constant bytes
    let ConstValue::Bytes(bytes) = machine.program.side_table.constant(value);
    let dest = machine.frame_pointer_at(dest).address() as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute one wide integer binary operation.
fn execute_wide_binary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, &[u8], &[u8]) -> Result<ScalarResult, Error>,
) -> Transfer {
    let dest = instruction.a;
    let left = instruction.b;
    let right = instruction.c;
    let layout = wide_integer_layout(instruction.d);
    let byte_len = wide_integer_byte_len(layout);

    // load source bytes
    let left_bytes = frame_bytes_at(machine, left, byte_len);
    let right_bytes = frame_bytes_at(machine, right, byte_len);
    let result = match operation(layout, left_bytes, right_bytes) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = store_scalar_value(machine, dest, result) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

macro_rules! wide_binary_executor {
    ($(#[$doc:meta] $name:ident => $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                execute_wide_binary(machine, instruction, $operation)
            }
        )+
    };
}

wide_binary_executor! {
    /// Execute wide integer addition.
    execute_add_wide_int => add_bytes_value,
    /// Execute wide integer subtraction.
    execute_sub_wide_int => subtract_bytes_value,
    /// Execute wide integer multiplication.
    execute_mul_wide_int => multiply_bytes_value,
    /// Execute wide signed integer division.
    execute_div_wide_int => divide_signed_bytes_value,
    /// Execute wide unsigned integer division.
    execute_div_wide_uint => divide_unsigned_bytes_value,
    /// Execute wide signed integer remainder.
    execute_rem_wide_int => remainder_signed_bytes_value,
    /// Execute wide unsigned integer remainder.
    execute_rem_wide_uint => remainder_unsigned_bytes_value,
    /// Execute wide integer bitwise AND.
    execute_and_wide_int => and_bytes_value,
    /// Execute wide integer bitwise OR.
    execute_or_wide_int => or_bytes_value,
    /// Execute wide integer bitwise XOR.
    execute_xor_wide_int => xor_bytes_value,
    /// Execute wide integer shift left.
    execute_shl_wide_int => shift_left_bytes_value,
    /// Execute wide signed integer shift right.
    execute_shr_wide_int => shift_right_signed_bytes_value,
    /// Execute wide unsigned integer shift right.
    execute_shr_wide_uint => shift_right_unsigned_bytes_value,
    /// Execute wide integer equality comparison.
    execute_eq_wide_int => equal_bytes_value,
    /// Execute wide integer inequality comparison.
    execute_ne_wide_int => not_equal_bytes_value,
    /// Execute wide signed integer less-than comparison.
    execute_lt_wide_int => less_signed_bytes_value,
    /// Execute wide unsigned integer less-than comparison.
    execute_lt_wide_uint => less_unsigned_bytes_value,
    /// Execute wide signed integer less-or-equal comparison.
    execute_le_wide_int => less_equal_signed_bytes_value,
    /// Execute wide unsigned integer less-or-equal comparison.
    execute_le_wide_uint => less_equal_unsigned_bytes_value,
    /// Execute wide signed integer greater-than comparison.
    execute_gt_wide_int => greater_signed_bytes_value,
    /// Execute wide unsigned integer greater-than comparison.
    execute_gt_wide_uint => greater_unsigned_bytes_value,
    /// Execute wide signed integer greater-or-equal comparison.
    execute_ge_wide_int => greater_equal_signed_bytes_value,
    /// Execute wide unsigned integer greater-or-equal comparison.
    execute_ge_wide_uint => greater_equal_unsigned_bytes_value,
}

/// Execute boolean AND.
#[inline(always)]
pub(crate) fn execute_and_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_bool() && right.as_bool()));

    Transfer::Continue
}

/// Execute boolean OR.
#[inline(always)]
pub(crate) fn execute_or_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_bool() || right.as_bool()));

    Transfer::Continue
}

/// Execute boolean XOR.
#[inline(always)]
pub(crate) fn execute_xor_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_bool() ^ right.as_bool()));

    Transfer::Continue
}

/// Execute float32 addition.
#[inline(always)]
pub(crate) fn execute_add_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() + right.as_f32()));

    Transfer::Continue
}

/// Execute float32 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() - right.as_f32()));

    Transfer::Continue
}

/// Execute float32 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() * right.as_f32()));

    Transfer::Continue
}

/// Execute float32 division.
#[inline(always)]
pub(crate) fn execute_div_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() / right.as_f32()));

    Transfer::Continue
}

/// Execute float32 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() == right.as_f32()));

    Transfer::Continue
}

/// Execute float32 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() != right.as_f32()));

    Transfer::Continue
}

/// Execute float32 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() < right.as_f32()));

    Transfer::Continue
}

/// Execute float32 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() <= right.as_f32()));

    Transfer::Continue
}

/// Execute float32 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() > right.as_f32()));

    Transfer::Continue
}

/// Execute float32 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() >= right.as_f32()));

    Transfer::Continue
}

/// Execute float64 addition.
#[inline(always)]
pub(crate) fn execute_add_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() + right.as_f64()));

    Transfer::Continue
}

/// Execute float64 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() - right.as_f64()));

    Transfer::Continue
}

/// Execute float64 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() * right.as_f64()));

    Transfer::Continue
}

/// Execute float64 division.
#[inline(always)]
pub(crate) fn execute_div_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() / right.as_f64()));

    Transfer::Continue
}

/// Execute float64 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() == right.as_f64()));

    Transfer::Continue
}

/// Execute float64 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() != right.as_f64()));

    Transfer::Continue
}

/// Execute float64 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() < right.as_f64()));

    Transfer::Continue
}

/// Execute float64 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() <= right.as_f64()));

    Transfer::Continue
}

/// Execute float64 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() > right.as_f64()));

    Transfer::Continue
}

/// Execute float64 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() >= right.as_f64()));

    Transfer::Continue
}

// ============================================================================
// integer arithmetic handlers
// ============================================================================

macro_rules! fixed_binary_executor {
    ($(#[$doc:meta] $name:ident => $word:ident, $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (dest, left, right) = load_binary_bits(machine, instruction);
                let left = left as $ty;
                let right = right as $ty;
                let value = left.$operation(right);
                machine.set_word_at(dest, Word::$word(value));

                Transfer::Continue
            }
        )+
    };
}

macro_rules! fixed_binary_layout_executor {
    ($(#[$doc:meta] $name:ident => $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (dest, left, right) = load_binary_bits(machine, instruction);
                let left = left as $ty;
                let right = right as $ty;
                let value = left.$operation(right);
                machine.set_word_at(dest, integer_word_from_field(value as u64, instruction.d));

                Transfer::Continue
            }
        )+
    };
}

macro_rules! fixed_div_executor {
    ($(#[$doc:meta] $name:ident => $word:ident, $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (dest, left, right) = load_binary_bits(machine, instruction);
                let left = left as $ty;
                let right = right as $ty;

                // reject undefined integer division
                if right == 0 {
                    return Transfer::Error(Error::DivisionByZero);
                }

                let value = left.$operation(right);
                machine.set_word_at(dest, Word::$word(value));

                Transfer::Continue
            }
        )+
    };
}

macro_rules! fixed_shift_executor {
    ($(#[$doc:meta] $name:ident => $word:ident, $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (dest, left, right) = load_binary_bits(machine, instruction);
                let left = left as $ty;
                let right = right as u32;
                let value = left.$operation(right);
                machine.set_word_at(dest, Word::$word(value));

                Transfer::Continue
            }
        )+
    };
}

macro_rules! fixed_shift_layout_executor {
    ($(#[$doc:meta] $name:ident => $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (dest, left, right) = load_binary_bits(machine, instruction);
                let left = left as $ty;
                let right = right as u32;
                let value = left.$operation(right);
                machine.set_word_at(dest, integer_word_from_field(value as u64, instruction.d));

                Transfer::Continue
            }
        )+
    };
}

macro_rules! fixed_compare_executor {
    ($(#[$doc:meta] $name:ident => $ty:ty, $operation:tt,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (dest, left, right) = load_binary_bits(machine, instruction);
                let left = left as $ty;
                let right = right as $ty;
                machine.set_word_at(dest, Word::bool(left $operation right));

                Transfer::Continue
            }
        )+
    };
}

macro_rules! fixed_unary_executor {
    ($(#[$doc:meta] $name:ident => $word:ident, $ty:ty, $operation:expr,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                let (dest, argument) = load_unary_bits(machine, instruction);
                let argument = argument as $ty;
                let value = $operation(argument);
                machine.set_word_at(dest, Word::$word(value));

                Transfer::Continue
            }
        )+
    };
}

fixed_binary_executor! {
    /// Execute 32-bit signed integer addition.
    execute_add_i32 => int32, i32, wrapping_add,
    /// Execute 32-bit unsigned integer addition.
    execute_add_u32 => uint32, u32, wrapping_add,
    /// Execute 64-bit signed integer addition.
    execute_add_i64 => int64, i64, wrapping_add,
    /// Execute 64-bit unsigned integer addition.
    execute_add_u64 => uint64, u64, wrapping_add,
    /// Execute 32-bit signed integer subtraction.
    execute_sub_i32 => int32, i32, wrapping_sub,
    /// Execute 32-bit unsigned integer subtraction.
    execute_sub_u32 => uint32, u32, wrapping_sub,
    /// Execute 64-bit signed integer subtraction.
    execute_sub_i64 => int64, i64, wrapping_sub,
    /// Execute 64-bit unsigned integer subtraction.
    execute_sub_u64 => uint64, u64, wrapping_sub,
    /// Execute 32-bit signed integer multiplication.
    execute_mul_i32 => int32, i32, wrapping_mul,
    /// Execute 32-bit unsigned integer multiplication.
    execute_mul_u32 => uint32, u32, wrapping_mul,
    /// Execute 64-bit signed integer multiplication.
    execute_mul_i64 => int64, i64, wrapping_mul,
    /// Execute 64-bit unsigned integer multiplication.
    execute_mul_u64 => uint64, u64, wrapping_mul,
}

fixed_binary_layout_executor! {
    /// Execute 32-bit integer bitwise AND.
    execute_and_32 => u32, bitand,
    /// Execute 64-bit integer bitwise AND.
    execute_and_64 => u64, bitand,
    /// Execute 32-bit integer bitwise OR.
    execute_or_32 => u32, bitor,
    /// Execute 64-bit integer bitwise OR.
    execute_or_64 => u64, bitor,
    /// Execute 32-bit integer bitwise XOR.
    execute_xor_32 => u32, bitxor,
    /// Execute 64-bit integer bitwise XOR.
    execute_xor_64 => u64, bitxor,
}

fixed_shift_layout_executor! {
    /// Execute 32-bit integer shift left.
    execute_shl_32 => u32, wrapping_shl,
    /// Execute 64-bit integer shift left.
    execute_shl_64 => u64, wrapping_shl,
}

fixed_shift_executor! {
    /// Execute 32-bit signed integer shift right.
    execute_shr_i32 => int32, i32, wrapping_shr,
    /// Execute 32-bit unsigned integer shift right.
    execute_shr_u32 => uint32, u32, wrapping_shr,
    /// Execute 64-bit signed integer shift right.
    execute_shr_i64 => int64, i64, wrapping_shr,
    /// Execute 64-bit unsigned integer shift right.
    execute_shr_u64 => uint64, u64, wrapping_shr,
}

fixed_div_executor! {
    /// Execute 32-bit signed integer division.
    execute_div_i32 => int32, i32, wrapping_div,
    /// Execute 32-bit unsigned integer division.
    execute_div_u32 => uint32, u32, wrapping_div,
    /// Execute 64-bit signed integer division.
    execute_div_i64 => int64, i64, wrapping_div,
    /// Execute 64-bit unsigned integer division.
    execute_div_u64 => uint64, u64, wrapping_div,
    /// Execute 32-bit signed integer remainder.
    execute_rem_i32 => int32, i32, wrapping_rem,
    /// Execute 32-bit unsigned integer remainder.
    execute_rem_u32 => uint32, u32, wrapping_rem,
    /// Execute 64-bit signed integer remainder.
    execute_rem_i64 => int64, i64, wrapping_rem,
    /// Execute 64-bit unsigned integer remainder.
    execute_rem_u64 => uint64, u64, wrapping_rem,
}

fixed_compare_executor! {
    /// Execute 32-bit integer equality comparison.
    execute_eq_32 => u32, ==,
    /// Execute 64-bit integer equality comparison.
    execute_eq_64 => u64, ==,
    /// Execute 32-bit integer inequality comparison.
    execute_ne_32 => u32, !=,
    /// Execute 64-bit integer inequality comparison.
    execute_ne_64 => u64, !=,
    /// Execute 32-bit signed less-than comparison.
    execute_lt_i32 => i32, <,
    /// Execute 32-bit unsigned less-than comparison.
    execute_lt_u32 => u32, <,
    /// Execute 64-bit signed less-than comparison.
    execute_lt_i64 => i64, <,
    /// Execute 64-bit unsigned less-than comparison.
    execute_lt_u64 => u64, <,
    /// Execute 32-bit signed less-or-equal comparison.
    execute_le_i32 => i32, <=,
    /// Execute 32-bit unsigned less-or-equal comparison.
    execute_le_u32 => u32, <=,
    /// Execute 64-bit signed less-or-equal comparison.
    execute_le_i64 => i64, <=,
    /// Execute 64-bit unsigned less-or-equal comparison.
    execute_le_u64 => u64, <=,
    /// Execute 32-bit signed greater-than comparison.
    execute_gt_i32 => i32, >,
    /// Execute 32-bit unsigned greater-than comparison.
    execute_gt_u32 => u32, >,
    /// Execute 64-bit signed greater-than comparison.
    execute_gt_i64 => i64, >,
    /// Execute 64-bit unsigned greater-than comparison.
    execute_gt_u64 => u64, >,
    /// Execute 32-bit signed greater-or-equal comparison.
    execute_ge_i32 => i32, >=,
    /// Execute 32-bit unsigned greater-or-equal comparison.
    execute_ge_u32 => u32, >=,
    /// Execute 64-bit signed greater-or-equal comparison.
    execute_ge_i64 => i64, >=,
    /// Execute 64-bit unsigned greater-or-equal comparison.
    execute_ge_u64 => u64, >=,
}

fixed_unary_executor! {
    /// Execute 32-bit signed integer negation.
    execute_neg_i32 => int32, i32, i32::wrapping_neg,
    /// Execute 64-bit signed integer negation.
    execute_neg_i64 => int64, i64, i64::wrapping_neg,
}

/// Execute 32-bit integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, argument) = load_unary_bits(machine, instruction);
    let value = !(argument as u32);
    machine.set_word_at(
        dest,
        integer_word_from_field(u64::from(value), instruction.d),
    );

    Transfer::Continue
}

/// Execute 64-bit integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, argument) = load_unary_bits(machine, instruction);
    let value = !argument;
    machine.set_word_at(dest, integer_word_from_field(value, instruction.d));

    Transfer::Continue
}

/// Execute signed integer addition.
#[inline(always)]
pub(crate) fn execute_add_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, integer_word::<true>(left.wrapping_add(right), width));

    Transfer::Continue
}

/// Execute unsigned integer addition.
#[inline(always)]
pub(crate) fn execute_add_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, integer_word::<false>(left.wrapping_add(right), width));

    Transfer::Continue
}

/// Execute signed integer subtraction.
#[inline(always)]
pub(crate) fn execute_sub_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, integer_word::<true>(left.wrapping_sub(right), width));

    Transfer::Continue
}

/// Execute unsigned integer subtraction.
#[inline(always)]
pub(crate) fn execute_sub_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, integer_word::<false>(left.wrapping_sub(right), width));

    Transfer::Continue
}

/// Execute signed integer multiplication.
#[inline(always)]
pub(crate) fn execute_mul_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, integer_word::<true>(left.wrapping_mul(right), width));

    Transfer::Continue
}

/// Execute unsigned integer multiplication.
#[inline(always)]
pub(crate) fn execute_mul_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, integer_word::<false>(left.wrapping_mul(right), width));

    Transfer::Continue
}

/// Execute signed integer division.
#[inline(always)]
pub(crate) fn execute_div_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    if right == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    let left = left as i64;
    let right = right as i64;
    machine.set_word_at(dest, Word::int(left.wrapping_div(right), width));

    Transfer::Continue
}

/// Execute signed integer remainder.
#[inline(always)]
pub(crate) fn execute_rem_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    if right == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    let left = left as i64;
    let right = right as i64;
    machine.set_word_at(dest, Word::int(left.wrapping_rem(right), width));

    Transfer::Continue
}

/// Execute unsigned integer division.
#[inline(always)]
pub(crate) fn execute_div_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    if right == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    machine.set_word_at(dest, Word::uint(left.wrapping_div(right), width));

    Transfer::Continue
}

/// Execute unsigned integer remainder.
#[inline(always)]
pub(crate) fn execute_rem_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    if right == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    machine.set_word_at(dest, Word::uint(left.wrapping_rem(right), width));

    Transfer::Continue
}

/// Execute word-sized integer bitwise AND.
#[inline(always)]
pub(crate) fn execute_and_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_bits(machine, instruction);
    machine.set_word_at(dest, integer_word_from_field(left & right, instruction.d));

    Transfer::Continue
}

/// Execute word-sized integer bitwise OR.
#[inline(always)]
pub(crate) fn execute_or_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_bits(machine, instruction);
    machine.set_word_at(dest, integer_word_from_field(left | right, instruction.d));

    Transfer::Continue
}

/// Execute word-sized integer bitwise XOR.
#[inline(always)]
pub(crate) fn execute_xor_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_bits(machine, instruction);
    machine.set_word_at(dest, integer_word_from_field(left ^ right, instruction.d));

    Transfer::Continue
}

/// Execute word-sized integer shift left.
#[inline(always)]
pub(crate) fn execute_shl_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_bits(machine, instruction);
    machine.set_word_at(
        dest,
        integer_word_from_field(left.wrapping_shl(right as u32), instruction.d),
    );

    Transfer::Continue
}

/// Execute arithmetic shift right.
#[inline(always)]
pub(crate) fn execute_shr_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(
        dest,
        Word::int((left as i64).wrapping_shr(right as u32), width),
    );

    Transfer::Continue
}

/// Execute logical shift right for unsigned values.
#[inline(always)]
pub(crate) fn execute_shr_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::uint(left.wrapping_shr(right as u32), width));

    Transfer::Continue
}

// ============================================================================
// integer comparison handlers
// ============================================================================

/// Execute integer equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left == right));

    Transfer::Continue
}

/// Execute integer inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left != right));

    Transfer::Continue
}

/// Execute signed less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) < (right as i64)));

    Transfer::Continue
}

/// Execute signed less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) <= (right as i64)));

    Transfer::Continue
}

/// Execute signed greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) > (right as i64)));

    Transfer::Continue
}

/// Execute signed greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) >= (right as i64)));

    Transfer::Continue
}

/// Execute unsigned less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left < right));

    Transfer::Continue
}

/// Execute unsigned less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left <= right));

    Transfer::Continue
}

/// Execute unsigned greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left > right));

    Transfer::Continue
}

/// Execute unsigned greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _) = load_binary_integer_values(machine, instruction);
    machine.set_word_at(dest, Word::bool(left >= right));

    Transfer::Continue
}

/// Execute one wide integer unary operation.
fn execute_wide_unary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, &[u8]) -> Result<Vec<u8>, Error>,
) -> Transfer {
    let dest = instruction.a;
    let arg = instruction.b;
    let layout = wide_integer_layout(instruction.d);
    let byte_len = wide_integer_byte_len(layout);

    // load source bytes
    let arg_bytes = frame_bytes_at(machine, arg, byte_len);
    let result = match operation(layout, arg_bytes) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    let dest = machine.frame_pointer_at(dest).address() as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(result.as_ptr(), dest, result.len());
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute wide integer negation.
#[inline(always)]
pub(crate) fn execute_neg_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_wide_unary(machine, instruction, negate_bytes_value)
}

/// Execute wide integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_wide_unary(machine, instruction, not_bytes_value)
}

/// Execute integer negation.
#[inline(always)]
pub(crate) fn execute_neg_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument, width) = load_unary_integer_value(machine, instruction);
    machine.set_word_at(dest, integer_word::<true>(argument.wrapping_neg(), width));

    Transfer::Continue
}

/// Execute word-sized integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_bits(machine, instruction);
    machine.set_word_at(dest, integer_word_from_field(!argument, instruction.d));

    Transfer::Continue
}

/// Execute float32 negation.
#[inline(always)]
pub(crate) fn execute_neg_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_value(machine, instruction);
    machine.set_word_at(dest, Word::float32(-argument.as_f32()));

    Transfer::Continue
}

/// Execute float64 negation.
#[inline(always)]
pub(crate) fn execute_neg_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_value(machine, instruction);
    machine.set_word_at(dest, Word::float64(-argument.as_f64()));

    Transfer::Continue
}

/// Execute boolean inversion.
pub(crate) fn execute_not_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_value(machine, instruction);
    machine.set_word_at(dest, Word::bool(!argument.as_bool()));

    Transfer::Continue
}
