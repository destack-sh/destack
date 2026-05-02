use super::operator;
use super::scalar::{convert_integer_bytes, integer_bytes_to_word};
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    CastWideInt, CastWideIntToWord, CastWord, CastWordToWideInt, Instruction, SelectFrame,
    SelectWord, Transfer,
};

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

/// Execute word cast opcode.
pub(crate) fn execute_cast_word(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let CastWord {
        dest,
        op,
        arg,
        to_type,
    } = instruction.payload_as::<CastWord>();

    // cast the word directly
    let argument = state.get_word_at(*arg);
    let cast_type = mir::LocalNodeId::new(*to_type);
    let result = match operator::evaluate_cast(state.tree(), *op, argument, cast_type) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // store result
    state.set_word_at(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute word to wide integer cast opcode.
pub(crate) fn execute_cast_word_to_wide_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CastWordToWideInt {
        dest,
        op,
        arg,
        source_width,
        source_signed,
        dest_width,
    } = instruction.payload_as::<CastWordToWideInt>();

    // cast from word bits into frame bytes
    let source = state.get_word_at(*arg).to_byte_array();
    let result = match cast_integer_bytes(*op, &source, *source_width, *source_signed, *dest_width)
    {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };

    // store result bytes
    let dest = match state.value_bytes_mut(*dest) {
        Ok(dest) => dest,
        Err(error) => return Transfer::Error(error),
    };
    dest.copy_from_slice(&result);

    Transfer::Continue
}

/// Execute wide integer to word cast opcode.
pub(crate) fn execute_cast_wide_int_to_word(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CastWideIntToWord {
        dest,
        op,
        arg,
        source_width,
        source_signed,
        dest_width,
        dest_signed,
    } = instruction.payload_as::<CastWideIntToWord>();

    // cast from frame bytes into word bits
    let source = match state.value_bytes(*arg) {
        Ok(source) => source,
        Err(error) => return Transfer::Error(error),
    };
    let bytes = match cast_integer_bytes(*op, source, *source_width, *source_signed, *dest_width) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
    let result = match integer_bytes_to_word(&bytes, *dest_width, *dest_signed) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };

    // store result word
    state.set_word_at(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute wide integer cast opcode.
pub(crate) fn execute_cast_wide_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let CastWideInt {
        dest,
        op,
        arg,
        source_width,
        source_signed,
        dest_width,
    } = instruction.payload_as::<CastWideInt>();

    // cast from frame bytes into frame bytes
    let source = match state.value_bytes(*arg) {
        Ok(source) => source,
        Err(error) => return Transfer::Error(error),
    };
    let result = match cast_integer_bytes(*op, source, *source_width, *source_signed, *dest_width) {
        Ok(result) => result,
        Err(error) => return Transfer::Error(error),
    };

    // store result bytes
    let dest = match state.value_bytes_mut(*dest) {
        Ok(dest) => dest,
        Err(error) => return Transfer::Error(error),
    };
    dest.copy_from_slice(&result);

    Transfer::Continue
}

/// Execute word select opcode.
pub(crate) fn execute_select_word(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let SelectWord {
        dest,
        condition,
        then_value,
        else_value,
    } = instruction.payload_as::<SelectWord>();

    // select the source word
    let condition = state.get_word_at(*condition).as_bool();
    let source = if condition { *then_value } else { *else_value };
    let result = state.get_word_at(source);

    // store result
    state.set_word_at(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute frame select opcode.
pub(crate) fn execute_select_frame(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let SelectFrame {
        dest,
        condition,
        then_value,
        else_value,
    } = instruction.payload_as::<SelectFrame>();

    // select the source frame value
    let condition = state.get_word_at(*condition).as_bool();
    let source = if condition { *then_value } else { *else_value };

    // move source bytes into the destination frame region
    let source_bytes = match state.value_bytes(source) {
        Ok(bytes) => bytes.to_vec(),
        Err(error) => return Transfer::Error(error),
    };
    let dest = match state.value_bytes_mut(*dest) {
        Ok(dest) => dest,
        Err(error) => return Transfer::Error(error),
    };
    dest.copy_from_slice(&source_bytes);

    // continue to next instruction
    Transfer::Continue
}
