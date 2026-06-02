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
use crate::Cell;
use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::{
    BinaryFloat, ConstValue, ConstValueId, Instruction, ScalarLayout, UnaryFloat,
};

const INTEGER_SIGN_BIT: u32 = 1 << 16;
const INTEGER_WIDTH_MASK: u32 = INTEGER_SIGN_BIT - 1;

/// Rebuild one canonical VM cell from integer bits.
#[inline(always)]
fn integer_cell<const IS_SIGNED: bool>(raw: u64, width: u8) -> Cell {
    if IS_SIGNED {
        return Cell::int(raw as i64, width);
    }

    Cell::uint(raw, width)
}

/// Rebuild one canonical VM cell from an encoded integer layout.
#[inline(always)]
fn integer_cell_from_field(raw: u64, field: u32) -> Cell {
    let width = integer_width(field);
    let is_signed = field & INTEGER_SIGN_BIT != 0;

    if is_signed {
        return Cell::int(raw as i64, width);
    }

    Cell::uint(raw, width)
}

/// Unpack one VM integer width.
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
    activation: &mut Activation<'_>,
    dest: u32,
    value: ScalarResult,
) -> Result<(), Error> {
    match value {
        ScalarResult::Cell(value) => activation.store_cell_at(dest, value),
        ScalarResult::Bytes(bytes) => {
            let dest = activation.frame_pointer_at(dest).address() as *mut u8;
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
fn frame_bytes_at<'a>(activation: &'a Activation<'_>, offset: u32, byte_len: usize) -> &'a [u8] {
    let address = activation.frame_pointer_at(offset).address() as *const u8;

    unsafe { std::slice::from_raw_parts(address, byte_len) }
}

/// Load one lowered binary cell operation.
#[inline(always)]
fn load_binary_cell_values(
    activation: &Activation<'_>,
    instruction: &Instruction,
) -> (u32, Cell, Cell) {
    let dest = instruction.a;
    let left = instruction.b;
    let right = instruction.c;

    (
        dest,
        activation.load_cell_at(left),
        activation.load_cell_at(right),
    )
}

/// Load one lowered binary cell operation as raw payloads.
#[inline(always)]
fn load_binary_raw_values(
    activation: &Activation<'_>,
    instruction: &Instruction,
) -> (u32, u64, u64) {
    let dest = instruction.a;
    let left = activation.load_cell_at(instruction.b).bits();
    let right = activation.load_cell_at(instruction.c).bits();

    (dest, left, right)
}

/// Load one lowered unary cell operation.
#[inline(always)]
fn load_unary_cell_value(activation: &Activation<'_>, instruction: &Instruction) -> (u32, Cell) {
    let dest = instruction.a;
    let argument = instruction.b;

    (dest, activation.load_cell_at(argument))
}

/// Load one lowered unary cell operation as a raw payload.
#[inline(always)]
fn load_unary_raw_value(activation: &Activation<'_>, instruction: &Instruction) -> (u32, u64) {
    let dest = instruction.a;
    let argument = activation.load_cell_at(instruction.b).bits();

    (dest, argument)
}

/// Load one lowered binary integer operation.
#[inline(always)]
fn load_binary_integer_values(
    activation: &Activation<'_>,
    instruction: &Instruction,
) -> (u32, u64, u64, u8) {
    let dest = instruction.a;
    let left = activation.load_cell_at(instruction.b).bits();
    let right = activation.load_cell_at(instruction.c).bits();
    let width = integer_width(instruction.d);

    (dest, left, right, width)
}

/// Load one lowered unary integer operation.
#[inline(always)]
fn load_unary_integer_value(
    activation: &Activation<'_>,
    instruction: &Instruction,
) -> (u32, u64, u8) {
    let dest = instruction.a;
    let argument = activation.load_cell_at(instruction.b).bits();
    let width = integer_width(instruction.d);

    (dest, argument, width)
}

/// Execute cell constant load.
#[inline(always)]
pub(crate) fn execute_load_const_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let bits = u64::from(instruction.b) | (u64::from(instruction.c) << 32);

    // store constant bits
    activation.store_cell_at(dest, Cell::from_bits(bits));

    Ok(())
}

/// Execute byte constant load.
#[inline(always)]
pub(crate) fn execute_load_const_bytes(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let value = ConstValueId(instruction.b);

    // copy constant bytes
    let ConstValue::Bytes(bytes) = activation.machine.program.side_table.constant(value);
    let dest = activation.frame_pointer_at(dest).address() as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
    }

    Ok(())
}

/// Execute one wide integer binary operation.
fn execute_wide_binary(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, &[u8], &[u8]) -> Result<ScalarResult, Error>,
) -> Result<(), Error> {
    let dest = instruction.a;
    let left = instruction.b;
    let right = instruction.c;
    let layout = wide_integer_layout(instruction.d);
    let byte_len = wide_integer_byte_len(layout);

    // load source bytes
    let left_bytes = frame_bytes_at(activation, left, byte_len);
    let right_bytes = frame_bytes_at(activation, right, byte_len);
    let result = operation(layout, left_bytes, right_bytes)?;
    store_scalar_value(activation, dest, result)?;

    Ok(())
}

macro_rules! wide_binary_executor {
    ($(#[$doc:meta] $name:ident => $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                execute_wide_binary(activation, instruction, $operation)
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
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_bool() && right.as_bool()));

    Ok(())
}

/// Execute boolean OR.
#[inline(always)]
pub(crate) fn execute_or_bool(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_bool() || right.as_bool()));

    Ok(())
}

/// Execute boolean XOR.
#[inline(always)]
pub(crate) fn execute_xor_bool(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_bool() ^ right.as_bool()));

    Ok(())
}

/// Execute float32 addition.
#[inline(always)]
pub(crate) fn execute_add_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float32(left.as_f32() + right.as_f32()));

    Ok(())
}

/// Execute float32 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float32(left.as_f32() - right.as_f32()));

    Ok(())
}

/// Execute float32 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float32(left.as_f32() * right.as_f32()));

    Ok(())
}

/// Execute float32 division.
#[inline(always)]
pub(crate) fn execute_div_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float32(left.as_f32() / right.as_f32()));

    Ok(())
}

/// Execute float32 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f32() == right.as_f32()));

    Ok(())
}

/// Execute float32 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f32() != right.as_f32()));

    Ok(())
}

/// Execute float32 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f32() < right.as_f32()));

    Ok(())
}

/// Execute float32 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f32() <= right.as_f32()));

    Ok(())
}

/// Execute float32 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f32() > right.as_f32()));

    Ok(())
}

/// Execute float32 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f32() >= right.as_f32()));

    Ok(())
}

/// Execute float64 addition.
#[inline(always)]
pub(crate) fn execute_add_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float64(left.as_f64() + right.as_f64()));

    Ok(())
}

/// Execute float64 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float64(left.as_f64() - right.as_f64()));

    Ok(())
}

/// Execute float64 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float64(left.as_f64() * right.as_f64()));

    Ok(())
}

/// Execute float64 division.
#[inline(always)]
pub(crate) fn execute_div_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::float64(left.as_f64() / right.as_f64()));

    Ok(())
}

/// Execute float64 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f64() == right.as_f64()));

    Ok(())
}

/// Execute float64 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f64() != right.as_f64()));

    Ok(())
}

/// Execute float64 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f64() < right.as_f64()));

    Ok(())
}

/// Execute float64 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f64() <= right.as_f64()));

    Ok(())
}

/// Execute float64 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f64() > right.as_f64()));

    Ok(())
}

/// Execute float64 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left.as_f64() >= right.as_f64()));

    Ok(())
}

/// Execute one generic binary float operation.
#[inline(always)]
pub(crate) fn execute_binary_float(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let operation = BinaryFloat::from_field(instruction.d);
    let (format, kernel) = operation.decode()?;
    let layout = ScalarLayout::Float { format };
    let (dest, left, right) = load_binary_cell_values(activation, instruction);
    let result = super::scalar::binary_float(layout, kernel, left, right)?;

    activation.store_cell_at(dest, result);

    Ok(())
}

// ============================================================================
// integer arithmetic handlers
// ============================================================================

macro_rules! fixed_binary_executor {
    ($(#[$doc:meta] $name:ident => $cell:ident, $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                let (dest, left, right) = load_binary_raw_values(activation, instruction);
                let left = left as $ty;
                let right = right as $ty;
                let value = left.$operation(right);
                activation.store_cell_at(dest, Cell::$cell(value));

                Ok(())
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
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                let (dest, left, right) = load_binary_raw_values(activation, instruction);
                let left = left as $ty;
                let right = right as $ty;
                let value = left.$operation(right);
                activation.store_cell_at(dest, integer_cell_from_field(value as u64, instruction.d));

                Ok(())
            }
        )+
    };
}

macro_rules! fixed_div_executor {
    ($(#[$doc:meta] $name:ident => $cell:ident, $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                let (dest, left, right) = load_binary_raw_values(activation, instruction);
                let left = left as $ty;
                let right = right as $ty;

                // reject undefined integer division
                if right == 0 {
                    return Err(Error::division_by_zero());
                }

                let value = left.$operation(right);
                activation.store_cell_at(dest, Cell::$cell(value));

                Ok(())
            }
        )+
    };
}

macro_rules! fixed_shift_executor {
    ($(#[$doc:meta] $name:ident => $cell:ident, $ty:ty, $operation:ident,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                let (dest, left, right) = load_binary_raw_values(activation, instruction);
                let left = left as $ty;
                let right = right as u32;
                let value = left.$operation(right);
                activation.store_cell_at(dest, Cell::$cell(value));

                Ok(())
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
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                let (dest, left, right) = load_binary_raw_values(activation, instruction);
                let left = left as $ty;
                let right = right as u32;
                let value = left.$operation(right);
                activation.store_cell_at(dest, integer_cell_from_field(value as u64, instruction.d));

                Ok(())
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
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                let (dest, left, right) = load_binary_raw_values(activation, instruction);
                let left = left as $ty;
                let right = right as $ty;
                activation.store_cell_at(dest, Cell::bool(left $operation right));

                Ok(())
            }
        )+
    };
}

macro_rules! fixed_unary_executor {
    ($(#[$doc:meta] $name:ident => $cell:ident, $ty:ty, $operation:expr,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                activation: &mut Activation<'_>,
                instruction: &Instruction,
            ) -> Result<(), Error> {
                let (dest, argument) = load_unary_raw_value(activation, instruction);
                let argument = argument as $ty;
                let value = $operation(argument);
                activation.store_cell_at(dest, Cell::$cell(value));

                Ok(())
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
pub(crate) fn execute_not_32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, argument) = load_unary_raw_value(activation, instruction);
    let value = !(argument as u32);
    activation.store_cell_at(
        dest,
        integer_cell_from_field(u64::from(value), instruction.d),
    );

    Ok(())
}

/// Execute 64-bit integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, argument) = load_unary_raw_value(activation, instruction);
    let value = !argument;
    activation.store_cell_at(dest, integer_cell_from_field(value, instruction.d));

    Ok(())
}

/// Execute cell-stored signed integer addition.
#[inline(always)]
pub(crate) fn execute_add_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell::<true>(left.wrapping_add(right), width));

    Ok(())
}

/// Execute cell-stored unsigned integer addition.
#[inline(always)]
pub(crate) fn execute_add_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell::<false>(left.wrapping_add(right), width));

    Ok(())
}

/// Execute cell-stored signed integer subtraction.
#[inline(always)]
pub(crate) fn execute_sub_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell::<true>(left.wrapping_sub(right), width));

    Ok(())
}

/// Execute cell-stored unsigned integer subtraction.
#[inline(always)]
pub(crate) fn execute_sub_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell::<false>(left.wrapping_sub(right), width));

    Ok(())
}

/// Execute cell-stored signed integer multiplication.
#[inline(always)]
pub(crate) fn execute_mul_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell::<true>(left.wrapping_mul(right), width));

    Ok(())
}

/// Execute cell-stored unsigned integer multiplication.
#[inline(always)]
pub(crate) fn execute_mul_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell::<false>(left.wrapping_mul(right), width));

    Ok(())
}

/// Execute cell-stored signed integer division.
#[inline(always)]
pub(crate) fn execute_div_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    let left = left as i64;
    let right = right as i64;
    activation.store_cell_at(dest, Cell::int(left.wrapping_div(right), width));

    Ok(())
}

/// Execute cell-stored signed integer remainder.
#[inline(always)]
pub(crate) fn execute_rem_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    let left = left as i64;
    let right = right as i64;
    activation.store_cell_at(dest, Cell::int(left.wrapping_rem(right), width));

    Ok(())
}

/// Execute cell-stored unsigned integer division.
#[inline(always)]
pub(crate) fn execute_div_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    activation.store_cell_at(dest, Cell::uint(left.wrapping_div(right), width));

    Ok(())
}

/// Execute cell-stored unsigned integer remainder.
#[inline(always)]
pub(crate) fn execute_rem_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    if right == 0 {
        return Err(Error::division_by_zero());
    }

    activation.store_cell_at(dest, Cell::uint(left.wrapping_rem(right), width));

    Ok(())
}

/// Execute cell-sized integer bitwise AND.
#[inline(always)]
pub(crate) fn execute_and_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_raw_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell_from_field(left & right, instruction.d));

    Ok(())
}

/// Execute cell-sized integer bitwise OR.
#[inline(always)]
pub(crate) fn execute_or_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_raw_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell_from_field(left | right, instruction.d));

    Ok(())
}

/// Execute cell-sized integer bitwise XOR.
#[inline(always)]
pub(crate) fn execute_xor_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_raw_values(activation, instruction);
    activation.store_cell_at(dest, integer_cell_from_field(left ^ right, instruction.d));

    Ok(())
}

/// Execute cell-sized integer shift left.
#[inline(always)]
pub(crate) fn execute_shl_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right) = load_binary_raw_values(activation, instruction);
    activation.store_cell_at(
        dest,
        integer_cell_from_field(left.wrapping_shl(right as u32), instruction.d),
    );

    Ok(())
}

/// Execute cell-stored arithmetic shift right.
#[inline(always)]
pub(crate) fn execute_shr_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(
        dest,
        Cell::int((left as i64).wrapping_shr(right as u32), width),
    );

    Ok(())
}

/// Execute cell-stored logical shift right.
#[inline(always)]
pub(crate) fn execute_shr_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, width) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::uint(left.wrapping_shr(right as u32), width));

    Ok(())
}

// ============================================================================
// integer comparison handlers
// ============================================================================

/// Execute cell-stored scalar equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left == right));

    Ok(())
}

/// Execute cell-stored scalar inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left != right));

    Ok(())
}

/// Execute cell-stored signed less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool((left as i64) < (right as i64)));

    Ok(())
}

/// Execute cell-stored signed less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool((left as i64) <= (right as i64)));

    Ok(())
}

/// Execute cell-stored signed greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool((left as i64) > (right as i64)));

    Ok(())
}

/// Execute cell-stored signed greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool((left as i64) >= (right as i64)));

    Ok(())
}

/// Execute cell-stored unsigned less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left < right));

    Ok(())
}

/// Execute cell-stored unsigned less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left <= right));

    Ok(())
}

/// Execute cell-stored unsigned greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left > right));

    Ok(())
}

/// Execute cell-stored unsigned greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_cell_uint(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, left, right, _) = load_binary_integer_values(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(left >= right));

    Ok(())
}

/// Execute one wide integer unary operation.
fn execute_wide_unary(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    operation: fn(ScalarLayout, &[u8]) -> Result<Vec<u8>, Error>,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let layout = wide_integer_layout(instruction.d);
    let byte_len = wide_integer_byte_len(layout);

    // load source bytes
    let arg_bytes = frame_bytes_at(activation, arg, byte_len);
    let result = operation(layout, arg_bytes)?;
    let dest = activation.frame_pointer_at(dest).address() as *mut u8;
    unsafe {
        std::ptr::copy_nonoverlapping(result.as_ptr(), dest, result.len());
    }

    Ok(())
}

/// Execute wide integer negation.
#[inline(always)]
pub(crate) fn execute_neg_wide_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_wide_unary(activation, instruction, negate_bytes_value)
}

/// Execute wide integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_wide_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_wide_unary(activation, instruction, not_bytes_value)
}

/// Execute cell-stored signed integer negation.
#[inline(always)]
pub(crate) fn execute_neg_cell_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, argument, width) = load_unary_integer_value(activation, instruction);
    activation.store_cell_at(dest, integer_cell::<true>(argument.wrapping_neg(), width));

    Ok(())
}

/// Execute cell-sized integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, argument) = load_unary_raw_value(activation, instruction);
    activation.store_cell_at(dest, integer_cell_from_field(!argument, instruction.d));

    Ok(())
}

/// Execute float32 negation.
#[inline(always)]
pub(crate) fn execute_neg_f32(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, argument) = load_unary_cell_value(activation, instruction);
    activation.store_cell_at(dest, Cell::float32(-argument.as_f32()));

    Ok(())
}

/// Execute float64 negation.
#[inline(always)]
pub(crate) fn execute_neg_f64(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, argument) = load_unary_cell_value(activation, instruction);
    activation.store_cell_at(dest, Cell::float64(-argument.as_f64()));

    Ok(())
}

/// Execute one generic unary float operation.
#[inline(always)]
pub(crate) fn execute_unary_float(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let operation = UnaryFloat::from_field(instruction.d);
    let (format, kernel) = operation.decode()?;
    let layout = ScalarLayout::Float { format };
    let (dest, value) = load_unary_cell_value(activation, instruction);
    let result = super::scalar::unary_float(layout, kernel, value)?;

    activation.store_cell_at(dest, result);

    Ok(())
}

/// Execute boolean inversion.
pub(crate) fn execute_not_bool(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let (dest, argument) = load_unary_cell_value(activation, instruction);
    activation.store_cell_at(dest, Cell::bool(!argument.as_bool()));

    Ok(())
}
