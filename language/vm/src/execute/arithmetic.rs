use super::access;
use super::element::element_access_for_type;
use super::scalar::{
    ScalarResult, binary_bytes, binary_operator, scalar_layout, unary_bytes, unary_operator,
};
use super::tensor::{load_tensor_element_at, tensor_layout};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    Binary, BinaryElementwise, BinaryInteger, BinaryWord, ConstValue, Instruction, LoadConst,
    Transfer, Unary, UnaryElementwise, UnaryInteger, UnaryWord,
};
use destack_mir as mir;

/// Return the integer layout carried by one lowered integer opcode.
fn binary_integer_layout(operands: &BinaryInteger) -> (u8, bool) {
    (operands.width, operands.is_signed)
}

/// Return the integer layout carried by one lowered unary opcode.
fn unary_integer_layout(operands: &UnaryInteger) -> (u8, bool) {
    (operands.width, operands.is_signed)
}

/// Rebuild one canonical VM word from integer bits.
#[inline(always)]
fn integer_word(raw: u64, width: u8, is_signed: bool) -> Word {
    if is_signed {
        return Word::int(raw as i64, width);
    }

    Word::uint(raw, width)
}

/// Return the vector element count for one vector value id.
fn vector_element_count(
    state: &DispatchState<'_, '_>,
    value_id: mir::Value,
) -> Result<usize, Error> {
    let vector_type = state.value_type(value_id)?;

    match state.tree().get(vector_type) {
        mir::Type::Vector {
            lanes: elements, ..
        } => Ok(*elements as usize),
        _ => Err(Error::TypeMismatch {
            expected: "vector type".to_string(),
            actual: format!("{vector_type:?}"),
        }),
    }
}

/// Load one vector element through indexed access.
fn load_vector_element_at(
    state: &mut DispatchState<'_, '_>,
    vector: Word,
    vector_type: mir::LocalNodeId<mir::Type>,
    element_index: usize,
) -> Result<Word, Error> {
    let index = u32::try_from(element_index).map_err(|_| Error::TypeMismatch {
        expected: "vector element index".to_string(),
        actual: element_index.to_string(),
    })?;
    let (element, element_count) = element_access_for_type(state, vector_type, index.into())?;

    access::load_frame_element(state, vector, index.into(), element_count, element)
}

/// Store one scalar operation result.
fn store_scalar_value(
    state: &mut DispatchState<'_, '_>,
    dest: mir::Value,
    value: ScalarResult,
) -> Result<(), Error> {
    match value {
        ScalarResult::Word(value) => state.set_word(dest, value),
        ScalarResult::Bytes(bytes) => state.value_bytes_mut(dest)?.copy_from_slice(&bytes),
    }

    Ok(())
}

/// Load one lowered binary word operation.
#[inline(always)]
fn load_binary_word_operands(
    state: &DispatchState<'_, '_>,
    instruction: &Instruction,
) -> (u32, Word, Word) {
    let BinaryWord { dest, left, right } = instruction.payload_as::<BinaryWord>();

    (*dest, state.get_word_at(*left), state.get_word_at(*right))
}

/// Load one lowered unary word operation.
#[inline(always)]
fn load_unary_word_operand(
    state: &DispatchState<'_, '_>,
    instruction: &Instruction,
) -> (u32, Word) {
    let UnaryWord { dest, arg } = instruction.payload_as::<UnaryWord>();

    (*dest, state.get_word_at(*arg))
}

/// Execute constant load.
pub(crate) fn execute_load_const(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let LoadConst { dest, value } = instruction.payload_as::<LoadConst>();

    // store constant value
    let value = state.program.operand_table.constant(*value);
    match value {
        ConstValue::Word(value) => state.set_word(*dest, *value),
        ConstValue::Bytes(bytes) => {
            let dest = match state.value_bytes_mut(*dest) {
                Ok(dest) => dest,
                Err(error) => return Transfer::Error(error),
            };

            dest.copy_from_slice(bytes);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute binary opcode.
pub(crate) fn execute_binary(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Binary {
        dest,
        op,
        left,
        right,
    } = instruction.payload_as::<Binary>();

    // execute the wide integer byte path
    let left_type = match state.value_type(*left) {
        Ok(left_type) => left_type,
        Err(error) => return Transfer::Error(error),
    };
    let layout = match scalar_layout(state.tree(), left_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let left_bytes = match state.value_bytes(*left) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
    let right_bytes = match state.value_bytes(*right) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
    let result = match binary_bytes(layout, *op, left_bytes, right_bytes) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = store_scalar_value(state, *dest, result) {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute boolean AND.
#[inline(always)]
pub(crate) fn execute_and_bool(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_bool() && right.as_bool()));

    Transfer::Continue
}

/// Execute boolean OR.
#[inline(always)]
pub(crate) fn execute_or_bool(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_bool() || right.as_bool()));

    Transfer::Continue
}

/// Execute boolean XOR.
#[inline(always)]
pub(crate) fn execute_xor_bool(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_bool() ^ right.as_bool()));

    Transfer::Continue
}

/// Execute float32 addition.
#[inline(always)]
pub(crate) fn execute_add_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float32(left.as_f32() + right.as_f32()));

    Transfer::Continue
}

/// Execute float32 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float32(left.as_f32() - right.as_f32()));

    Transfer::Continue
}

/// Execute float32 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float32(left.as_f32() * right.as_f32()));

    Transfer::Continue
}

/// Execute float32 division.
#[inline(always)]
pub(crate) fn execute_div_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float32(left.as_f32() / right.as_f32()));

    Transfer::Continue
}

/// Execute float32 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f32() == right.as_f32()));

    Transfer::Continue
}

/// Execute float32 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f32() != right.as_f32()));

    Transfer::Continue
}

/// Execute float32 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f32() < right.as_f32()));

    Transfer::Continue
}

/// Execute float32 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f32() <= right.as_f32()));

    Transfer::Continue
}

/// Execute float32 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f32() > right.as_f32()));

    Transfer::Continue
}

/// Execute float32 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f32() >= right.as_f32()));

    Transfer::Continue
}

/// Execute float64 addition.
#[inline(always)]
pub(crate) fn execute_add_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float64(left.as_f64() + right.as_f64()));

    Transfer::Continue
}

/// Execute float64 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float64(left.as_f64() - right.as_f64()));

    Transfer::Continue
}

/// Execute float64 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float64(left.as_f64() * right.as_f64()));

    Transfer::Continue
}

/// Execute float64 division.
#[inline(always)]
pub(crate) fn execute_div_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::float64(left.as_f64() / right.as_f64()));

    Transfer::Continue
}

/// Execute float64 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f64() == right.as_f64()));

    Transfer::Continue
}

/// Execute float64 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f64() != right.as_f64()));

    Transfer::Continue
}

/// Execute float64 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f64() < right.as_f64()));

    Transfer::Continue
}

/// Execute float64 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f64() <= right.as_f64()));

    Transfer::Continue
}

/// Execute float64 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f64() > right.as_f64()));

    Transfer::Continue
}

/// Execute float64 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(state, instruction);
    state.set_word_at(dest, Word::bool(left.as_f64() >= right.as_f64()));

    Transfer::Continue
}

// ============================================================================
// specialized integer arithmetic handlers
// ============================================================================

/// Execute integer addition.
#[inline(always)]
pub(crate) fn execute_add_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    let (width, is_signed) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, integer_word(a.wrapping_add(b), width, is_signed));
    Transfer::Continue
}

/// Execute integer subtraction.
#[inline(always)]
pub(crate) fn execute_sub_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    let (width, is_signed) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, integer_word(a.wrapping_sub(b), width, is_signed));
    Transfer::Continue
}

/// Execute integer multiplication.
#[inline(always)]
pub(crate) fn execute_mul_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    let (width, is_signed) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, integer_word(a.wrapping_mul(b), width, is_signed));
    Transfer::Continue
}

/// Execute signed integer division.
#[inline(always)]
pub(crate) fn execute_div_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    if b == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    let (width, _) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, Word::int(a.wrapping_div(b), width));
    Transfer::Continue
}

/// Execute signed integer remainder.
#[inline(always)]
pub(crate) fn execute_rem_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    if b == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    let (width, _) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, Word::int(a.wrapping_rem(b), width));
    Transfer::Continue
}

/// Execute unsigned integer division.
#[inline(always)]
pub(crate) fn execute_div_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    if b == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    let (width, _) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, Word::uint(a.wrapping_div(b), width));
    Transfer::Continue
}

/// Execute unsigned integer remainder.
#[inline(always)]
pub(crate) fn execute_rem_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    if b == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    let (width, _) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, Word::uint(a.wrapping_rem(b), width));
    Transfer::Continue
}

/// Execute integer bitwise AND.
#[inline(always)]
pub(crate) fn execute_and_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    let (width, is_signed) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, integer_word(a & b, width, is_signed));
    Transfer::Continue
}

/// Execute integer bitwise OR.
#[inline(always)]
pub(crate) fn execute_or_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    let (width, is_signed) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, integer_word(a | b, width, is_signed));
    Transfer::Continue
}

/// Execute integer bitwise XOR.
#[inline(always)]
pub(crate) fn execute_xor_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    let (width, is_signed) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, integer_word(a ^ b, width, is_signed));
    Transfer::Continue
}

/// Execute integer shift left.
#[inline(always)]
pub(crate) fn execute_shl_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits() as u32;
    let (width, is_signed) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, integer_word(a.wrapping_shl(b), width, is_signed));
    Transfer::Continue
}

/// Execute arithmetic shift right.
#[inline(always)]
pub(crate) fn execute_shr_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as u32;
    let (width, _) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, Word::int(a.wrapping_shr(b), width));
    Transfer::Continue
}

/// Execute logical shift right for unsigned values.
#[inline(always)]
pub(crate) fn execute_shr_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits() as u32;
    let (width, _) = binary_integer_layout(instruction.payload_as::<BinaryInteger>());
    state.set_word_at(*dest, Word::uint(a.wrapping_shr(b), width));
    Transfer::Continue
}

// ============================================================================
// specialized integer comparison handlers
// ============================================================================

/// Execute integer equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    state.set_word_at(*dest, Word::bool(a == b));
    Transfer::Continue
}

/// Execute integer inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    state.set_word_at(*dest, Word::bool(a != b));
    Transfer::Continue
}

/// Execute signed less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    state.set_word_at(*dest, Word::bool(a < b));
    Transfer::Continue
}

/// Execute signed less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    state.set_word_at(*dest, Word::bool(a <= b));
    Transfer::Continue
}

/// Execute signed greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    state.set_word_at(*dest, Word::bool(a > b));
    Transfer::Continue
}

/// Execute signed greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits() as i64;
    let b = state.get_word_at(*right).bits() as i64;
    state.set_word_at(*dest, Word::bool(a >= b));
    Transfer::Continue
}

/// Execute unsigned less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    state.set_word_at(*dest, Word::bool(a < b));
    Transfer::Continue
}

/// Execute unsigned less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    state.set_word_at(*dest, Word::bool(a <= b));
    Transfer::Continue
}

/// Execute unsigned greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    state.set_word_at(*dest, Word::bool(a > b));
    Transfer::Continue
}

/// Execute unsigned greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryInteger {
        dest, left, right, ..
    } = instruction.payload_as::<BinaryInteger>();
    let a = state.get_word_at(*left).bits();
    let b = state.get_word_at(*right).bits();
    state.set_word_at(*dest, Word::bool(a >= b));
    Transfer::Continue
}

/// Execute elementwise binary opcode on vector or tensor values.
pub(crate) fn execute_binary_elementwise(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let BinaryElementwise {
        dest,
        op,
        left,
        right,
        result_type,
    } = instruction.payload_as::<BinaryElementwise>();

    let result_type_id = *result_type;
    let result_type = state.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector {
            lanes: elements, ..
        } => {
            let left_value = state.get_word(*left);
            let right_value = state.get_word(*right);
            let expected = elements as usize;
            let left_element_count = match vector_element_count(state, *left) {
                Ok(elements) => elements,
                Err(error) => return Transfer::Error(error),
            };
            let right_element_count = match vector_element_count(state, *right) {
                Ok(elements) => elements,
                Err(error) => return Transfer::Error(error),
            };
            if left_element_count != expected || right_element_count != expected {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "matching vector elements".to_string(),
                    actual: format!("{left_element_count} vs {right_element_count}"),
                });
            }

            if let Err(error) = super::frame::store_frame_elements(
                state,
                *dest,
                |state, element_index, value_type| {
                    let left_type = state.value_type(*left)?;
                    let right_type = state.value_type(*right)?;
                    let lhs = load_vector_element_at(state, left_value, left_type, element_index)?;
                    let rhs =
                        load_vector_element_at(state, right_value, right_type, element_index)?;
                    let layout = scalar_layout(state.tree(), value_type)?;

                    binary_operator(layout, *op, lhs, rhs)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout(state.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let left_value = state.get_word(*left);
            let right_value = state.get_word(*right);
            if let Err(error) = super::frame::store_frame_elements(
                state,
                *dest,
                |state, element_index, value_type| {
                    if element_index >= layout.element_span_len {
                        return Err(Error::IndexOutOfBounds {
                            index: element_index as u64,
                            length: layout.element_span_len as u64,
                        });
                    }

                    let left_type = state.value_type(*left)?;
                    let right_type = state.value_type(*right)?;
                    let lhs = load_tensor_element_at(state, left_value, left_type, element_index)?;
                    let rhs =
                        load_tensor_element_at(state, right_value, right_type, element_index)?;
                    let layout = scalar_layout(state.tree(), value_type)?;

                    binary_operator(layout, *op, lhs, rhs)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        _ => Transfer::Error(Error::InvalidInstruction),
    }
}

/// Execute unary opcode.
pub(crate) fn execute_unary(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Unary { dest, op, arg } = instruction.payload_as::<Unary>();

    // execute the wide integer byte path
    let arg_type = match state.value_type(*arg) {
        Ok(arg_type) => arg_type,
        Err(error) => return Transfer::Error(error),
    };
    let layout = match scalar_layout(state.tree(), arg_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let arg_bytes = match state.value_bytes(*arg) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
    let result = match unary_bytes(layout, *op, arg_bytes) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = state
        .value_bytes_mut(*dest)
        .map(|dest| dest.copy_from_slice(&result))
    {
        return Transfer::Error(error);
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute elementwise unary opcode on vector or tensor values.
pub(crate) fn execute_unary_elementwise(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let UnaryElementwise {
        dest,
        op,
        arg,
        result_type,
    } = instruction.payload_as::<UnaryElementwise>();

    let result_type_id = *result_type;
    let result_type = state.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector {
            lanes: elements, ..
        } => {
            let argument = state.get_word(*arg);
            let expected = elements as usize;
            let element_count = match vector_element_count(state, *arg) {
                Ok(elements) => elements,
                Err(error) => return Transfer::Error(error),
            };
            if element_count != expected {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "matching vector elements".to_string(),
                    actual: element_count.to_string(),
                });
            }

            if let Err(error) = super::frame::store_frame_elements(
                state,
                *dest,
                |state, element_index, value_type| {
                    let argument_type = state.value_type(*arg)?;
                    let value =
                        load_vector_element_at(state, argument, argument_type, element_index)?;
                    let layout = scalar_layout(state.tree(), value_type)?;

                    unary_operator(layout, *op, value)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout(state.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let argument = state.get_word(*arg);
            if let Err(error) = super::frame::store_frame_elements(
                state,
                *dest,
                |state, element_index, value_type| {
                    if element_index >= layout.element_span_len {
                        return Err(Error::IndexOutOfBounds {
                            index: element_index as u64,
                            length: layout.element_span_len as u64,
                        });
                    }

                    let argument_type = state.value_type(*arg)?;
                    let value =
                        load_tensor_element_at(state, argument, argument_type, element_index)?;
                    let layout = scalar_layout(state.tree(), value_type)?;

                    unary_operator(layout, *op, value)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        _ => Transfer::Error(Error::InvalidInstruction),
    }
}

/// Execute integer negation.
#[inline(always)]
pub(crate) fn execute_neg_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let UnaryInteger { dest, arg, .. } = instruction.payload_as::<UnaryInteger>();
    let (width, is_signed) = unary_integer_layout(instruction.payload_as::<UnaryInteger>());

    let argument = state.get_word_at(*arg).as_u64();
    state.set_word_at(
        *dest,
        integer_word(argument.wrapping_neg(), width, is_signed),
    );

    Transfer::Continue
}

/// Execute integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let UnaryInteger { dest, arg, .. } = instruction.payload_as::<UnaryInteger>();
    let (width, is_signed) = unary_integer_layout(instruction.payload_as::<UnaryInteger>());

    let argument = state.get_word_at(*arg).as_u64();
    state.set_word_at(*dest, integer_word(!argument, width, is_signed));

    Transfer::Continue
}

/// Execute float32 negation.
#[inline(always)]
pub(crate) fn execute_neg_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_operand(state, instruction);
    state.set_word_at(dest, Word::float32(-argument.as_f32()));

    Transfer::Continue
}

/// Execute float64 negation.
#[inline(always)]
pub(crate) fn execute_neg_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_operand(state, instruction);
    state.set_word_at(dest, Word::float64(-argument.as_f64()));

    Transfer::Continue
}

/// Execute boolean inversion.
pub(crate) fn execute_not_bool(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_operand(state, instruction);
    state.set_word_at(dest, Word::bool(!argument.as_bool()));

    Transfer::Continue
}
