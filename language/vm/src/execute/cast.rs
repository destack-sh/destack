use destack_core::{float_from_bits, float_to_bits};
use destack_mir as mir;

use super::scalar::{convert_integer_bytes, integer_bytes_to_cell};
use crate::Cell;
use crate::diagnostic::Error;
use crate::machine::Activation;
use crate::program::{
    CellLayout, FloatCast, FloatToIntCast, FrameSelect, Instruction, IntToFloatCast, IntegerCast,
    PointerCast, WideIntegerCast,
};

/// Execute one lowered cell cast.
fn execute_cell_cast(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
    cast: fn(Cell, u32) -> Result<Cell, Error>,
) -> Result<(), Error> {
    let dest = instruction.a;
    let argument = instruction.b;
    let cast_field = instruction.c;

    // cast the cell directly
    let argument = activation.load_cell_at(argument);
    let result = cast(argument, cast_field)?;

    // store result
    activation.store_cell_at(dest, result);

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
fn cast_integer_to_pointer(raw: u64, field: u32) -> Result<Cell, Error> {
    let layout = PointerCast::from_field(field).decode()?;

    Ok(layout.decode(raw))
}

/// Truncate a signed integer to a target bit width.
fn truncate_signed(value: i64, width: u8) -> i64 {
    if width >= Cell::BIT_LEN {
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
    if width >= Cell::BIT_LEN {
        return value;
    }

    let mask = (1u64 << width) - 1;

    value & mask
}

/// Compute integer bounds for a width and signedness.
fn integer_bounds(width: u8, is_signed: bool) -> Option<(i128, i128)> {
    if width == 0 || width > Cell::BIT_LEN {
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

/// Reinterpret one cell value.
fn cast_bitcast(argument: Cell, _field: u32) -> Result<Cell, Error> {
    Ok(argument)
}

/// Truncate one integer cell.
fn cast_truncate(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (width, is_signed) = IntegerCast::from_field(field).decode();

    Ok(if is_signed {
        Cell::int(truncate_signed(argument.as_i64(), width), width)
    } else {
        Cell::uint(truncate_unsigned(argument.as_u64(), width), width)
    })
}

/// Zero extend one integer cell.
fn cast_zero_extend(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (width, _) = IntegerCast::from_field(field).decode();

    Ok(Cell::uint(argument.as_u64(), width))
}

/// Sign extend one integer cell.
fn cast_sign_extend(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (width, _) = IntegerCast::from_field(field).decode();

    Ok(Cell::int(argument.as_i64(), width))
}

/// Convert one float cell to a signed integer cell.
fn cast_float_to_signed_int(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::invalid_cast())?;
    let value = float_cell_to_f64(argument, source)?;
    let converted = float_to_int_checked(value, min_bound, max_bound)
        .ok_or(Error::bad_conversion_to_integer())?;

    Ok(Cell::int(converted as i64, target_width))
}

/// Convert one float cell to an unsigned integer cell.
fn cast_float_to_unsigned_int(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::invalid_cast())?;
    let value = float_cell_to_f64(argument, source)?;
    let converted = float_to_int_checked(value, min_bound, max_bound)
        .ok_or(Error::bad_conversion_to_integer())?;

    Ok(Cell::uint(converted as u64, target_width))
}

/// Saturating convert one float cell to a signed integer cell.
fn cast_float_to_signed_int_saturating(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, true).ok_or(Error::invalid_cast())?;
    let value = float_cell_to_f64(argument, source)?;
    let converted = float_to_int_saturating(value, min_bound, max_bound);

    Ok(Cell::int(converted as i64, target_width))
}

/// Saturating convert one float cell to an unsigned integer cell.
fn cast_float_to_unsigned_int_saturating(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (source, target_width) = FloatToIntCast::from_field(field).decode()?;
    let (min_bound, max_bound) = integer_bounds(target_width, false).ok_or(Error::invalid_cast())?;
    let value = float_cell_to_f64(argument, source)?;
    let converted = float_to_int_saturating(value, min_bound, max_bound);

    Ok(Cell::uint(converted as u64, target_width))
}

/// Convert one signed integer cell to a float cell.
fn cast_signed_int_to_float(argument: Cell, field: u32) -> Result<Cell, Error> {
    let destination = IntToFloatCast::from_field(field).decode()?;

    f64_to_float_cell(argument.as_i64() as f64, destination)
}

/// Convert one unsigned integer cell to a float cell.
fn cast_unsigned_int_to_float(argument: Cell, field: u32) -> Result<Cell, Error> {
    let destination = IntToFloatCast::from_field(field).decode()?;

    f64_to_float_cell(argument.as_u64() as f64, destination)
}

/// Convert one float cell.
fn cast_float_convert(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (source, destination) = FloatCast::from_field(field).decode()?;
    let value = float_cell_to_f64(argument, source)?;

    f64_to_float_cell(value, destination)
}

/// Convert one pointer cell to an integer cell.
fn cast_pointer_to_int(argument: Cell, field: u32) -> Result<Cell, Error> {
    let (target_width, _) = IntegerCast::from_field(field).decode();

    Ok(Cell::uint(argument.as_u64(), target_width))
}

/// Convert one integer cell to a pointer cell.
fn cast_int_to_pointer(argument: Cell, field: u32) -> Result<Cell, Error> {
    cast_integer_to_pointer(argument.as_u64(), field)
}

/// Execute bitcast cell op.
pub(crate) fn execute_cast_bitcast(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_bitcast)
}

/// Execute truncate cell op.
pub(crate) fn execute_cast_truncate(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_truncate)
}

/// Execute zero extend cell op.
pub(crate) fn execute_cast_zero_extend(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_zero_extend)
}

/// Execute sign extend cell op.
pub(crate) fn execute_cast_sign_extend(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_sign_extend)
}

/// Execute float to signed integer cell op.
pub(crate) fn execute_cast_float_to_signed_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_float_to_signed_int)
}

/// Execute float to unsigned integer cell op.
pub(crate) fn execute_cast_float_to_unsigned_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_float_to_unsigned_int)
}

/// Execute saturating float to signed integer cell op.
pub(crate) fn execute_cast_float_to_signed_int_saturating(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_float_to_signed_int_saturating)
}

/// Execute saturating float to unsigned integer cell op.
pub(crate) fn execute_cast_float_to_unsigned_int_saturating(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(
        activation,
        instruction,
        cast_float_to_unsigned_int_saturating,
    )
}

/// Execute signed integer to float cell op.
pub(crate) fn execute_cast_signed_int_to_float(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_signed_int_to_float)
}

/// Execute unsigned integer to float cell op.
pub(crate) fn execute_cast_unsigned_int_to_float(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_unsigned_int_to_float)
}

/// Execute float convert cell op.
pub(crate) fn execute_cast_float_convert(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_float_convert)
}

/// Decode one float cell as f64.
fn float_cell_to_f64(cell: Cell, layout: CellLayout) -> Result<f64, Error> {
    match layout {
        CellLayout::Float16 => Ok(float_from_bits(
            mir::FloatType::Float16.format(),
            cell.bits(),
        )),
        CellLayout::Bfloat16 => Ok(float_from_bits(
            mir::FloatType::Bfloat16.format(),
            cell.bits(),
        )),
        CellLayout::Float32 => Ok(cell.as_f32() as f64),
        CellLayout::Float64 => Ok(cell.as_f64()),
        _ => Err(Error::invalid_cast()),
    }
}

/// Encode one f64 into a float cell layout.
fn f64_to_float_cell(value: f64, layout: CellLayout) -> Result<Cell, Error> {
    match layout {
        CellLayout::Float16 => Ok(Cell::from_bits(float_to_bits(
            mir::FloatType::Float16.format(),
            value,
        ))),
        CellLayout::Bfloat16 => Ok(Cell::from_bits(float_to_bits(
            mir::FloatType::Bfloat16.format(),
            value,
        ))),
        CellLayout::Float32 => Ok(Cell::float32(value as f32)),
        CellLayout::Float64 => Ok(Cell::float64(value)),
        _ => Err(Error::invalid_cast()),
    }
}

/// Execute pointer to integer cell op.
pub(crate) fn execute_cast_pointer_to_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_pointer_to_int)
}

/// Execute integer to pointer cell op.
pub(crate) fn execute_cast_int_to_pointer(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    execute_cell_cast(activation, instruction, cast_int_to_pointer)
}

/// Execute cell to wide integer cast op.
pub(crate) fn execute_cast_cell_to_wide_int(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, _) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from cell bits into frame bytes
    let source = activation.load_cell_at(arg).to_byte_array();
    let result = cast_integer_bytes(&source, source_width, source_signed, dest_width);

    // store result bytes
    activation.store_frame_bytes_at(dest, &result);

    Ok(())
}

/// Execute wide integer to cell cast op.
pub(crate) fn execute_cast_wide_int_to_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let arg = instruction.b;
    let cast = WideIntegerCast::from_fields(instruction.c, instruction.d);
    let (source_signed, dest_signed) = cast.signs();
    let (source_width, dest_width) = cast.widths_pair();

    // cast from frame bytes into cell bits
    let source = activation.frame_bytes_at(arg, integer_byte_len(source_width));
    let bytes = cast_integer_bytes(source, source_width, source_signed, dest_width);
    let result = integer_bytes_to_cell(&bytes, dest_width, dest_signed)?;

    // store result cell
    activation.store_cell_at(dest, result);

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

/// Execute cell select op.
pub(crate) fn execute_select_cell(
    activation: &mut Activation<'_>,
    instruction: &Instruction,
) -> Result<(), Error> {
    let dest = instruction.a;
    let condition = instruction.b;
    let then_value = instruction.c;
    let else_value = instruction.d;

    // select the source cell
    let condition = activation.load_cell_at(condition).as_bool();
    let source = if condition { then_value } else { else_value };
    let result = activation.load_cell_at(source);

    // store result
    activation.store_cell_at(dest, result);

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
    let condition = activation.load_cell_at(*condition_offset).as_bool();
    let source_offset = if condition { then_offset } else { else_offset };

    // move the selected frame slot directly
    activation.copy_frame_bytes(*source_offset, *destination_offset, *byte_len);

    Ok(())
}
