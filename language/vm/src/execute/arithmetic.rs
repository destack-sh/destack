use super::access;
use super::element::element_access_for_type;
use super::scalar::{
    ScalarResult, binary_bytes, binary_operator, scalar_layout, unary_bytes, unary_operator,
};
use super::tensor::{load_tensor_element_at, tensor_layout};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{ConstValue, ConstValueId, Instruction, Transfer};
use destack_mir as mir;

const INTEGER_SIGN_BIT: u32 = 1 << 16;
const INTEGER_WIDTH_MASK: u32 = INTEGER_SIGN_BIT - 1;

/// Return one binary operator from an side record.
pub(super) fn binary_operator_from_operand(operand: u32) -> Result<mir::BinaryOperator, Error> {
    match operand {
        0 => Ok(mir::BinaryOperator::Add),
        1 => Ok(mir::BinaryOperator::Subtract),
        2 => Ok(mir::BinaryOperator::Multiply),
        3 => Ok(mir::BinaryOperator::SignedDivide),
        4 => Ok(mir::BinaryOperator::UnsignedDivide),
        5 => Ok(mir::BinaryOperator::SignedRemainder),
        6 => Ok(mir::BinaryOperator::UnsignedRemainder),
        7 => Ok(mir::BinaryOperator::FloatAdd),
        8 => Ok(mir::BinaryOperator::FloatSubtract),
        9 => Ok(mir::BinaryOperator::FloatMultiply),
        10 => Ok(mir::BinaryOperator::FloatDivide),
        11 => Ok(mir::BinaryOperator::And),
        12 => Ok(mir::BinaryOperator::Or),
        13 => Ok(mir::BinaryOperator::Xor),
        14 => Ok(mir::BinaryOperator::ShiftLeft),
        15 => Ok(mir::BinaryOperator::ArithmeticShiftRight),
        16 => Ok(mir::BinaryOperator::LogicalShiftRight),
        17 => Ok(mir::BinaryOperator::Equal),
        18 => Ok(mir::BinaryOperator::NotEqual),
        19 => Ok(mir::BinaryOperator::SignedLessThan),
        20 => Ok(mir::BinaryOperator::SignedLessEqual),
        21 => Ok(mir::BinaryOperator::SignedGreaterThan),
        22 => Ok(mir::BinaryOperator::SignedGreaterEqual),
        23 => Ok(mir::BinaryOperator::UnsignedLessThan),
        24 => Ok(mir::BinaryOperator::UnsignedLessEqual),
        25 => Ok(mir::BinaryOperator::UnsignedGreaterThan),
        26 => Ok(mir::BinaryOperator::UnsignedGreaterEqual),
        27 => Ok(mir::BinaryOperator::FloatEqual),
        28 => Ok(mir::BinaryOperator::FloatNotEqual),
        29 => Ok(mir::BinaryOperator::FloatLessThan),
        30 => Ok(mir::BinaryOperator::FloatLessEqual),
        31 => Ok(mir::BinaryOperator::FloatGreaterThan),
        32 => Ok(mir::BinaryOperator::FloatGreaterEqual),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Return one unary operator from an side record.
fn unary_operator_from_operand(operand: u32) -> Result<mir::UnaryOperator, Error> {
    match operand {
        0 => Ok(mir::UnaryOperator::Negate),
        1 => Ok(mir::UnaryOperator::FloatNegate),
        2 => Ok(mir::UnaryOperator::Not),
        _ => Err(Error::InvalidInstruction),
    }
}

/// Rebuild one canonical VM word from integer bits.
#[inline(always)]
fn integer_word(raw: u64, width: u8, is_signed: bool) -> Word {
    if is_signed {
        return Word::int(raw as i64, width);
    }

    Word::uint(raw, width)
}

/// Unpack one machine integer layout.
#[inline(always)]
fn integer_layout(operand: u32) -> (u8, bool) {
    let width = (operand & INTEGER_WIDTH_MASK) as u8;
    let is_signed = operand & INTEGER_SIGN_BIT != 0;

    (width, is_signed)
}

/// Return the vector element count for one vector value id.
fn vector_element_count(machine: &Machine<'_, '_>, value_id: mir::Value) -> Result<usize, Error> {
    let vector_type = machine.value_type(value_id)?;

    match machine.tree().get(vector_type) {
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
    machine: &mut Machine<'_, '_>,
    vector: Word,
    vector_type: mir::LocalNodeId<mir::Type>,
    element_index: usize,
) -> Result<Word, Error> {
    let index = u32::try_from(element_index).map_err(|_| Error::TypeMismatch {
        expected: "vector element index".to_string(),
        actual: element_index.to_string(),
    })?;
    let (element, element_count) = element_access_for_type(machine, vector_type, index.into())?;

    access::load_frame_element(machine, vector, index.into(), element_count, element)
}

/// Store one scalar operation result.
fn store_scalar_value(
    machine: &mut Machine<'_, '_>,
    dest: mir::Value,
    value: ScalarResult,
) -> Result<(), Error> {
    match value {
        ScalarResult::Word(value) => machine.set_word(dest, value),
        ScalarResult::Bytes(bytes) => machine.value_bytes_mut(dest)?.copy_from_slice(&bytes),
    }

    Ok(())
}

/// Load one lowered binary word operation.
#[inline(always)]
fn load_binary_word_operands(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (u32, Word, Word) {
    let dest = instruction.a;
    let left = instruction.b;
    let right = instruction.c;

    (dest, machine.get_word_at(left), machine.get_word_at(right))
}

/// Load one lowered unary word operation.
#[inline(always)]
fn load_unary_word_operand(machine: &Machine<'_, '_>, instruction: &Instruction) -> (u32, Word) {
    let dest = instruction.a;
    let argument = instruction.b;

    (dest, machine.get_word_at(argument))
}

/// Load one lowered binary integer operation.
#[inline(always)]
fn load_binary_integer_operands(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (u32, u64, u64, u8, bool) {
    let dest = instruction.a;
    let left = machine.get_word_at(instruction.b).bits();
    let right = machine.get_word_at(instruction.c).bits();
    let (width, is_signed) = integer_layout(instruction.d);

    (dest, left, right, width, is_signed)
}

/// Load one lowered unary integer operation.
#[inline(always)]
fn load_unary_integer_operand(
    machine: &Machine<'_, '_>,
    instruction: &Instruction,
) -> (u32, u64, u8, bool) {
    let dest = instruction.a;
    let argument = machine.get_word_at(instruction.b).bits();
    let (width, is_signed) = integer_layout(instruction.d);

    (dest, argument, width, is_signed)
}

/// Execute constant load.
#[inline(always)]
pub(crate) fn execute_load_const(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let value = ConstValueId(instruction.b);

    // store constant value
    let value = machine.program.side_table.constant(value);
    match value {
        ConstValue::Word(value) => machine.set_word(dest, *value),
        ConstValue::Bytes(bytes) => {
            let dest = match machine.value_bytes_mut(dest) {
                Ok(dest) => dest,
                Err(error) => return Transfer::Error(error),
            };

            dest.copy_from_slice(bytes);
        }
    }

    // continue to next instruction
    Transfer::Continue
}

/// Execute one wide integer binary operation.
fn execute_wide_binary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    op: mir::BinaryOperator,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let left = mir::Value::new(instruction.b);
    let right = mir::Value::new(instruction.c);

    // execute the wide integer byte path
    let left_type = match machine.value_type(left) {
        Ok(left_type) => left_type,
        Err(error) => return Transfer::Error(error),
    };
    let layout = match scalar_layout(machine.tree(), left_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let left_bytes = match machine.value_bytes(left) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
    let right_bytes = match machine.value_bytes(right) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
    let result = match binary_bytes(layout, op, left_bytes, right_bytes) {
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
    ($(#[$doc:meta] $name:ident => $operator:path,)+) => {
        $(
            #[$doc]
            #[inline(always)]
            pub(crate) fn $name(
                machine: &mut Machine<'_, '_>,
                instruction: &Instruction,
            ) -> Transfer {
                execute_wide_binary(machine, instruction, $operator)
            }
        )+
    };
}

wide_binary_executor! {
    /// Execute wide integer addition.
    execute_add_wide_int => mir::BinaryOperator::Add,
    /// Execute wide integer subtraction.
    execute_sub_wide_int => mir::BinaryOperator::Subtract,
    /// Execute wide integer multiplication.
    execute_mul_wide_int => mir::BinaryOperator::Multiply,
    /// Execute wide signed integer division.
    execute_div_wide_int => mir::BinaryOperator::SignedDivide,
    /// Execute wide unsigned integer division.
    execute_div_wide_uint => mir::BinaryOperator::UnsignedDivide,
    /// Execute wide signed integer remainder.
    execute_rem_wide_int => mir::BinaryOperator::SignedRemainder,
    /// Execute wide unsigned integer remainder.
    execute_rem_wide_uint => mir::BinaryOperator::UnsignedRemainder,
    /// Execute wide integer bitwise AND.
    execute_and_wide_int => mir::BinaryOperator::And,
    /// Execute wide integer bitwise OR.
    execute_or_wide_int => mir::BinaryOperator::Or,
    /// Execute wide integer bitwise XOR.
    execute_xor_wide_int => mir::BinaryOperator::Xor,
    /// Execute wide integer shift left.
    execute_shl_wide_int => mir::BinaryOperator::ShiftLeft,
    /// Execute wide signed integer shift right.
    execute_shr_wide_int => mir::BinaryOperator::ArithmeticShiftRight,
    /// Execute wide unsigned integer shift right.
    execute_shr_wide_uint => mir::BinaryOperator::LogicalShiftRight,
    /// Execute wide integer equality comparison.
    execute_eq_wide_int => mir::BinaryOperator::Equal,
    /// Execute wide integer inequality comparison.
    execute_ne_wide_int => mir::BinaryOperator::NotEqual,
    /// Execute wide signed integer less-than comparison.
    execute_lt_wide_int => mir::BinaryOperator::SignedLessThan,
    /// Execute wide unsigned integer less-than comparison.
    execute_lt_wide_uint => mir::BinaryOperator::UnsignedLessThan,
    /// Execute wide signed integer less-or-equal comparison.
    execute_le_wide_int => mir::BinaryOperator::SignedLessEqual,
    /// Execute wide unsigned integer less-or-equal comparison.
    execute_le_wide_uint => mir::BinaryOperator::UnsignedLessEqual,
    /// Execute wide signed integer greater-than comparison.
    execute_gt_wide_int => mir::BinaryOperator::SignedGreaterThan,
    /// Execute wide unsigned integer greater-than comparison.
    execute_gt_wide_uint => mir::BinaryOperator::UnsignedGreaterThan,
    /// Execute wide signed integer greater-or-equal comparison.
    execute_ge_wide_int => mir::BinaryOperator::SignedGreaterEqual,
    /// Execute wide unsigned integer greater-or-equal comparison.
    execute_ge_wide_uint => mir::BinaryOperator::UnsignedGreaterEqual,
}

/// Execute boolean AND.
#[inline(always)]
pub(crate) fn execute_and_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_bool() && right.as_bool()));

    Transfer::Continue
}

/// Execute boolean OR.
#[inline(always)]
pub(crate) fn execute_or_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_bool() || right.as_bool()));

    Transfer::Continue
}

/// Execute boolean XOR.
#[inline(always)]
pub(crate) fn execute_xor_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_bool() ^ right.as_bool()));

    Transfer::Continue
}

/// Execute float32 addition.
#[inline(always)]
pub(crate) fn execute_add_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() + right.as_f32()));

    Transfer::Continue
}

/// Execute float32 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() - right.as_f32()));

    Transfer::Continue
}

/// Execute float32 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() * right.as_f32()));

    Transfer::Continue
}

/// Execute float32 division.
#[inline(always)]
pub(crate) fn execute_div_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float32(left.as_f32() / right.as_f32()));

    Transfer::Continue
}

/// Execute float32 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() == right.as_f32()));

    Transfer::Continue
}

/// Execute float32 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() != right.as_f32()));

    Transfer::Continue
}

/// Execute float32 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() < right.as_f32()));

    Transfer::Continue
}

/// Execute float32 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() <= right.as_f32()));

    Transfer::Continue
}

/// Execute float32 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() > right.as_f32()));

    Transfer::Continue
}

/// Execute float32 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f32(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f32() >= right.as_f32()));

    Transfer::Continue
}

/// Execute float64 addition.
#[inline(always)]
pub(crate) fn execute_add_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() + right.as_f64()));

    Transfer::Continue
}

/// Execute float64 subtraction.
#[inline(always)]
pub(crate) fn execute_sub_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() - right.as_f64()));

    Transfer::Continue
}

/// Execute float64 multiplication.
#[inline(always)]
pub(crate) fn execute_mul_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() * right.as_f64()));

    Transfer::Continue
}

/// Execute float64 division.
#[inline(always)]
pub(crate) fn execute_div_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::float64(left.as_f64() / right.as_f64()));

    Transfer::Continue
}

/// Execute float64 equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() == right.as_f64()));

    Transfer::Continue
}

/// Execute float64 inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() != right.as_f64()));

    Transfer::Continue
}

/// Execute float64 less-than comparison.
#[inline(always)]
pub(crate) fn execute_lt_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() < right.as_f64()));

    Transfer::Continue
}

/// Execute float64 less-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_le_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() <= right.as_f64()));

    Transfer::Continue
}

/// Execute float64 greater-than comparison.
#[inline(always)]
pub(crate) fn execute_gt_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() > right.as_f64()));

    Transfer::Continue
}

/// Execute float64 greater-or-equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_f64(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right) = load_binary_word_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left.as_f64() >= right.as_f64()));

    Transfer::Continue
}

// ============================================================================
// specialized integer arithmetic handlers
// ============================================================================

/// Execute integer addition.
#[inline(always)]
pub(crate) fn execute_add_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, is_signed) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(
        dest,
        integer_word(left.wrapping_add(right), width, is_signed),
    );

    Transfer::Continue
}

/// Execute integer subtraction.
#[inline(always)]
pub(crate) fn execute_sub_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, is_signed) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(
        dest,
        integer_word(left.wrapping_sub(right), width, is_signed),
    );

    Transfer::Continue
}

/// Execute integer multiplication.
#[inline(always)]
pub(crate) fn execute_mul_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, is_signed) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(
        dest,
        integer_word(left.wrapping_mul(right), width, is_signed),
    );

    Transfer::Continue
}

/// Execute signed integer division.
#[inline(always)]
pub(crate) fn execute_div_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, _) = load_binary_integer_operands(machine, instruction);
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
    let (dest, left, right, width, _) = load_binary_integer_operands(machine, instruction);
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
    let (dest, left, right, width, _) = load_binary_integer_operands(machine, instruction);
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
    let (dest, left, right, width, _) = load_binary_integer_operands(machine, instruction);
    if right == 0 {
        return Transfer::Error(Error::DivisionByZero);
    }

    machine.set_word_at(dest, Word::uint(left.wrapping_rem(right), width));

    Transfer::Continue
}

/// Execute integer bitwise AND.
#[inline(always)]
pub(crate) fn execute_and_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, is_signed) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, integer_word(left & right, width, is_signed));

    Transfer::Continue
}

/// Execute integer bitwise OR.
#[inline(always)]
pub(crate) fn execute_or_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, width, is_signed) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, integer_word(left | right, width, is_signed));

    Transfer::Continue
}

/// Execute integer bitwise XOR.
#[inline(always)]
pub(crate) fn execute_xor_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, is_signed) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, integer_word(left ^ right, width, is_signed));

    Transfer::Continue
}

/// Execute integer shift left.
#[inline(always)]
pub(crate) fn execute_shl_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, is_signed) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(
        dest,
        integer_word(left.wrapping_shl(right as u32), width, is_signed),
    );

    Transfer::Continue
}

/// Execute arithmetic shift right.
#[inline(always)]
pub(crate) fn execute_shr_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, width, _) = load_binary_integer_operands(machine, instruction);
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
    let (dest, left, right, width, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::uint(left.wrapping_shr(right as u32), width));

    Transfer::Continue
}

// ============================================================================
// specialized integer comparison handlers
// ============================================================================

/// Execute integer equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left == right));

    Transfer::Continue
}

/// Execute integer inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left != right));

    Transfer::Continue
}

/// Execute signed less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) < (right as i64)));

    Transfer::Continue
}

/// Execute signed less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) <= (right as i64)));

    Transfer::Continue
}

/// Execute signed greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) > (right as i64)));

    Transfer::Continue
}

/// Execute signed greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_int(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool((left as i64) >= (right as i64)));

    Transfer::Continue
}

/// Execute unsigned less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left < right));

    Transfer::Continue
}

/// Execute unsigned less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left <= right));

    Transfer::Continue
}

/// Execute unsigned greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left > right));

    Transfer::Continue
}

/// Execute unsigned greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, left, right, _, _) = load_binary_integer_operands(machine, instruction);
    machine.set_word_at(dest, Word::bool(left >= right));

    Transfer::Continue
}

/// Execute elementwise binary op on vector or tensor values.
pub(crate) fn execute_binary_elementwise(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let left = mir::Value::new(instruction.b);
    let right = mir::Value::new(instruction.c);
    let op = match binary_operator_from_operand(instruction.d) {
        Ok(op) => op,
        Err(error) => return Transfer::Error(error),
    };
    let result_type_id = match machine.value_type(dest) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let result_type = machine.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector {
            lanes: elements, ..
        } => {
            let left_value = machine.get_word(left);
            let right_value = machine.get_word(right);
            let expected = elements as usize;
            let left_element_count = match vector_element_count(machine, left) {
                Ok(elements) => elements,
                Err(error) => return Transfer::Error(error),
            };
            let right_element_count = match vector_element_count(machine, right) {
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
                machine,
                dest,
                |machine, element_index, value_type| {
                    let left_type = machine.value_type(left)?;
                    let right_type = machine.value_type(right)?;
                    let lhs =
                        load_vector_element_at(machine, left_value, left_type, element_index)?;
                    let rhs =
                        load_vector_element_at(machine, right_value, right_type, element_index)?;
                    let layout = scalar_layout(machine.tree(), value_type)?;

                    binary_operator(layout, op, lhs, rhs)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout(machine.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let left_value = machine.get_word(left);
            let right_value = machine.get_word(right);
            if let Err(error) = super::frame::store_frame_elements(
                machine,
                dest,
                |machine, element_index, value_type| {
                    if element_index >= layout.element_span_len {
                        return Err(Error::IndexOutOfBounds {
                            index: element_index as u64,
                            length: layout.element_span_len as u64,
                        });
                    }

                    let left_type = machine.value_type(left)?;
                    let right_type = machine.value_type(right)?;
                    let lhs =
                        load_tensor_element_at(machine, left_value, left_type, element_index)?;
                    let rhs =
                        load_tensor_element_at(machine, right_value, right_type, element_index)?;
                    let layout = scalar_layout(machine.tree(), value_type)?;

                    binary_operator(layout, op, lhs, rhs)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        _ => Transfer::Error(Error::InvalidInstruction),
    }
}

/// Execute one wide integer unary operation.
fn execute_wide_unary(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
    op: mir::UnaryOperator,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let arg = mir::Value::new(instruction.b);

    // execute the wide integer byte path
    let arg_type = match machine.value_type(arg) {
        Ok(arg_type) => arg_type,
        Err(error) => return Transfer::Error(error),
    };
    let layout = match scalar_layout(machine.tree(), arg_type) {
        Ok(layout) => layout,
        Err(error) => return Transfer::Error(error),
    };
    let arg_bytes = match machine.value_bytes(arg) {
        Ok(bytes) => bytes,
        Err(error) => return Transfer::Error(error),
    };
    let result = match unary_bytes(layout, op, arg_bytes) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };
    if let Err(error) = machine
        .value_bytes_mut(dest)
        .map(|dest| dest.copy_from_slice(&result))
    {
        return Transfer::Error(error);
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
    execute_wide_unary(machine, instruction, mir::UnaryOperator::Negate)
}

/// Execute wide integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_wide_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    execute_wide_unary(machine, instruction, mir::UnaryOperator::Not)
}

/// Execute elementwise unary op on vector or tensor values.
pub(crate) fn execute_unary_elementwise(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let dest = mir::Value::new(instruction.a);
    let arg = mir::Value::new(instruction.b);
    let op = match unary_operator_from_operand(instruction.c) {
        Ok(op) => op,
        Err(error) => return Transfer::Error(error),
    };
    let result_type_id = match machine.value_type(dest) {
        Ok(ty) => ty,
        Err(error) => return Transfer::Error(error),
    };
    let result_type = machine.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector {
            lanes: elements, ..
        } => {
            let argument = machine.get_word(arg);
            let expected = elements as usize;
            let element_count = match vector_element_count(machine, arg) {
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
                machine,
                dest,
                |machine, element_index, value_type| {
                    let argument_type = machine.value_type(arg)?;
                    let value =
                        load_vector_element_at(machine, argument, argument_type, element_index)?;
                    let layout = scalar_layout(machine.tree(), value_type)?;

                    unary_operator(layout, op, value)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout(machine.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let argument = machine.get_word(arg);
            if let Err(error) = super::frame::store_frame_elements(
                machine,
                dest,
                |machine, element_index, value_type| {
                    if element_index >= layout.element_span_len {
                        return Err(Error::IndexOutOfBounds {
                            index: element_index as u64,
                            length: layout.element_span_len as u64,
                        });
                    }

                    let argument_type = machine.value_type(arg)?;
                    let value =
                        load_tensor_element_at(machine, argument, argument_type, element_index)?;
                    let layout = scalar_layout(machine.tree(), value_type)?;

                    unary_operator(layout, op, value)
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
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument, width, is_signed) = load_unary_integer_operand(machine, instruction);
    machine.set_word_at(
        dest,
        integer_word(argument.wrapping_neg(), width, is_signed),
    );

    Transfer::Continue
}

/// Execute integer bit inversion.
#[inline(always)]
pub(crate) fn execute_not_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument, width, is_signed) = load_unary_integer_operand(machine, instruction);
    machine.set_word_at(dest, integer_word(!argument, width, is_signed));

    Transfer::Continue
}

/// Execute float32 negation.
#[inline(always)]
pub(crate) fn execute_neg_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_operand(machine, instruction);
    machine.set_word_at(dest, Word::float32(-argument.as_f32()));

    Transfer::Continue
}

/// Execute float64 negation.
#[inline(always)]
pub(crate) fn execute_neg_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_operand(machine, instruction);
    machine.set_word_at(dest, Word::float64(-argument.as_f64()));

    Transfer::Continue
}

/// Execute boolean inversion.
pub(crate) fn execute_not_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (dest, argument) = load_unary_word_operand(machine, instruction);
    machine.set_word_at(dest, Word::bool(!argument.as_bool()));

    Transfer::Continue
}
