use super::operator;
use super::scalar::{convert_integer_bytes, integer_bytes_to_word};
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{Instruction, Transfer};

use destack_mir as mir;

/// Check whether one cast operator is a wide integer cast.
fn is_wide_integer_cast(operator: mir::CastOperator) -> bool {
    matches!(
        operator,
        mir::CastOperator::Bitcast
            | mir::CastOperator::Truncate
            | mir::CastOperator::ZeroExtend
            | mir::CastOperator::SignExtend
    )
}

/// Return one cast operator from an side record.
fn cast_operator(operand: u32) -> Result<mir::CastOperator, Error> {
    match operand {
        0 => Ok(mir::CastOperator::Bitcast),
        1 => Ok(mir::CastOperator::Truncate),
        2 => Ok(mir::CastOperator::ZeroExtend),
        3 => Ok(mir::CastOperator::SignExtend),
        4 => Ok(mir::CastOperator::FloatToSignedInt),
        5 => Ok(mir::CastOperator::FloatToUnsignedInt),
        6 => Ok(mir::CastOperator::FloatToSignedIntSaturating),
        7 => Ok(mir::CastOperator::FloatToUnsignedIntSaturating),
        8 => Ok(mir::CastOperator::SignedIntToFloat),
        9 => Ok(mir::CastOperator::UnsignedIntToFloat),
        10 => Ok(mir::CastOperator::FloatTruncate),
        11 => Ok(mir::CastOperator::FloatExtend),
        12 => Ok(mir::CastOperator::PointerToInt),
        13 => Ok(mir::CastOperator::IntToPointer),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Return one wide integer cast operator from an side record.
fn wide_cast_operator(operand: u32) -> Result<(mir::CastOperator, bool, bool), Error> {
    let operator = cast_operator(operand & 0xff)?;
    let source_signed = operand & (1 << 8) != 0;
    let dest_signed = operand & (1 << 9) != 0;

    Ok((operator, source_signed, dest_signed))
}

/// Return one wide integer cast layout from an side record.
fn wide_cast_layout(operand: u32) -> (u16, u16) {
    let source_width = operand as u16;
    let dest_width = (operand >> 16) as u16;

    (source_width, dest_width)
}

/// Execute one lowered wide integer cast.
fn cast_integer_bytes(
    operator: mir::CastOperator,
    source: &[u8],
    source_width: u16,
    source_signed: bool,
    dest_width: u16,
) -> Result<Vec<u8>, Error> {
    if !is_wide_integer_cast(operator) {
        return Err(Error::InvalidCast);
    }

    Ok(convert_integer_bytes(
        source,
        source_width,
        source_signed,
        dest_width,
    ))
}

/// Execute word cast op.
pub(crate) fn execute_cast_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let op = match cast_operator(instruction.b) {
        Ok(op) => op,
        Err(error) => return Transfer::Error(error),
    };
    let arg = instruction.c;
    let to_type = instruction.d;

    // cast the word directly
    let argument = machine.get_word_at(arg);
    let cast_type = mir::LocalNodeId::new(to_type);
    let result = match operator::evaluate_cast(machine.tree(), op, argument, cast_type) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    machine.set_word_at(dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute word to wide integer cast op.
pub(crate) fn execute_cast_word_to_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let arg = instruction.b;
    let (op, source_signed, _) = match wide_cast_operator(instruction.c) {
        Ok(op) => op,
        Err(error) => return Transfer::Error(error),
    };
    let (source_width, dest_width) = wide_cast_layout(instruction.d);

    // cast from word bits into frame bytes
    let source = machine.get_word_at(arg).to_byte_array();
    let result = match cast_integer_bytes(op, &source, source_width, source_signed, dest_width) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };

    // store result bytes
    let dest = match machine.value_bytes_mut(dest) {
        Ok(dest) => dest,
        Err(error) => return Transfer::Error(error),
    };
    dest.copy_from_slice(&result);

    Transfer::Continue
}

/// Execute wide integer to word cast op.
pub(crate) fn execute_cast_wide_int_to_word(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = instruction.a;
    let arg = mir::Value::new(instruction.b);
    let (op, source_signed, dest_signed) = match wide_cast_operator(instruction.c) {
        Ok(op) => op,
        Err(error) => return Transfer::Error(error),
    };
    let (source_width, dest_width) = wide_cast_layout(instruction.d);

    // cast from frame bytes into word bits
    let source = match machine.value_bytes(arg) {
        Ok(source) => source,
        Err(error) => return Transfer::Error(error),
    };
    let bytes = match cast_integer_bytes(op, source, source_width, source_signed, dest_width) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
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
    let dest = mir::Value::new(instruction.a);
    let arg = mir::Value::new(instruction.b);
    let (op, source_signed, _) = match wide_cast_operator(instruction.c) {
        Ok(op) => op,
        Err(error) => return Transfer::Error(error),
    };
    let (source_width, dest_width) = wide_cast_layout(instruction.d);

    // cast from frame bytes into frame bytes
    let source = match machine.value_bytes(arg) {
        Ok(source) => source,
        Err(error) => return Transfer::Error(error),
    };
    let result = match cast_integer_bytes(op, source, source_width, source_signed, dest_width) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };

    // store result bytes
    let dest = match machine.value_bytes_mut(dest) {
        Ok(dest) => dest,
        Err(error) => return Transfer::Error(error),
    };
    dest.copy_from_slice(&result);

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
    let dest = mir::Value::new(instruction.a);
    let condition = instruction.b;
    let then_value = mir::Value::new(instruction.c);
    let else_value = mir::Value::new(instruction.d);

    // select the source frame value
    let condition = machine.get_word_at(condition).as_bool();
    let source = if condition { then_value } else { else_value };

    // move the selected frame slot directly
    match machine.move_value_to_value(source, dest) {
        Ok(()) => {}
        Err(error) => return Transfer::Error(error),
    }

    // continue to next instruction
    Transfer::Continue
}
