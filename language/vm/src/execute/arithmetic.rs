use super::prelude::*;

/// Return the integer width carried by one lowered integer opcode.
fn binary_integer_width(operands: &Operands) -> u8 {
    match operands {
        Operands::BinarySpecialized { width, .. }
        | Operands::BinaryConstRightSpecialized { width, .. } => *width,
        _ => u64::BITS as u8,
    }
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
    let (element, element_count) =
        access::element_access_for_type(state, vector_type, index.into())?;

    access::get_element(
        state,
        vector,
        index.into(),
        Some(element_count),
        Some(element),
    )
}

/// Execute constant load.
pub(crate) fn execute_const(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Const { dest, value } = &block[pc].operands else {
        unreachable!()
    };

    // resolve constant value
    let ConstValue::Word(value) = value;
    let value = *value;

    // write value
    state.set_word(*dest, value);

    // continue to next instruction
    Transfer::Continue
}

/// Execute binary opcode.
pub(crate) fn execute_binary(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute opcode
    let result = match operator::execute_binary(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute signed integer binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute opcode
    let result = match operator::execute_binary_int(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute unsigned integer binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute opcode
    let result = match operator::execute_binary_uint(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute float32 binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_float32(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute opcode
    let result = match operator::execute_binary_float32(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute float64 binary opcode.
#[inline(always)]
pub(crate) fn execute_binary_float64(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute opcode
    let result = match operator::execute_binary_float64(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute boolean binary opcode.
pub(crate) fn execute_binary_bool(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute opcode
    let result = match operator::execute_binary_bool(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

// ============================================================================
// specialized integer arithmetic handlers
// ============================================================================

/// Execute integer addition without operator dispatch.
#[inline(always)]
pub(crate) fn execute_add_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_add(b), width));
    Transfer::Continue
}

/// Execute integer subtraction without operator dispatch.
#[inline(always)]
pub(crate) fn execute_sub_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_sub(b), width));
    Transfer::Continue
}

/// Execute integer multiplication without operator dispatch.
#[inline(always)]
pub(crate) fn execute_mul_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_mul(b), width));
    Transfer::Continue
}

/// Execute integer bitwise AND without operator dispatch.
#[inline(always)]
pub(crate) fn execute_and_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a & b, width));
    Transfer::Continue
}

/// Execute integer bitwise OR without operator dispatch.
#[inline(always)]
pub(crate) fn execute_or_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a | b, width));
    Transfer::Continue
}

/// Execute integer bitwise XOR without operator dispatch.
#[inline(always)]
pub(crate) fn execute_xor_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a ^ b, width));
    Transfer::Continue
}

/// Execute shift left without operator dispatch.
#[inline(always)]
pub(crate) fn execute_shl_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as u32;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_shl(b), width));
    Transfer::Continue
}

/// Execute arithmetic shift right without operator dispatch.
#[inline(always)]
pub(crate) fn execute_shr_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as u32;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_shr(b), width));
    Transfer::Continue
}

// ============================================================================
// specialized unsigned integer arithmetic handlers
// ============================================================================

/// Execute unsigned integer addition without operator dispatch.
#[inline(always)]
pub(crate) fn execute_add_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_add(b), width));
    Transfer::Continue
}

/// Execute unsigned integer subtraction without operator dispatch.
#[inline(always)]
pub(crate) fn execute_sub_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_sub(b), width));
    Transfer::Continue
}

/// Execute unsigned integer multiplication without operator dispatch.
#[inline(always)]
pub(crate) fn execute_mul_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_mul(b), width));
    Transfer::Continue
}

/// Execute unsigned bitwise AND without operator dispatch.
#[inline(always)]
pub(crate) fn execute_and_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a & b, width));
    Transfer::Continue
}

/// Execute unsigned bitwise OR without operator dispatch.
#[inline(always)]
pub(crate) fn execute_or_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a | b, width));
    Transfer::Continue
}

/// Execute unsigned bitwise XOR without operator dispatch.
#[inline(always)]
pub(crate) fn execute_xor_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a ^ b, width));
    Transfer::Continue
}

/// Execute unsigned shift left without operator dispatch.
#[inline(always)]
pub(crate) fn execute_shl_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits() as u32;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_shl(b), width));
    Transfer::Continue
}

/// Execute logical shift right for unsigned values.
#[inline(always)]
pub(crate) fn execute_shr_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits() as u32;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_shr(b), width));
    Transfer::Continue
}

// ============================================================================
// specialized integer comparison handlers
// ============================================================================

/// Execute integer equality comparison.
#[inline(always)]
pub(crate) fn execute_eq_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    state.set_word(*dest, Word::bool(a == b));
    Transfer::Continue
}

/// Execute integer inequality comparison.
#[inline(always)]
pub(crate) fn execute_ne_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    state.set_word(*dest, Word::bool(a != b));
    Transfer::Continue
}

/// Execute signed less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    state.set_word(*dest, Word::bool(a < b));
    Transfer::Continue
}

/// Execute signed less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    state.set_word(*dest, Word::bool(a <= b));
    Transfer::Continue
}

/// Execute signed greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    state.set_word(*dest, Word::bool(a > b));
    Transfer::Continue
}

/// Execute signed greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = state.get(*right).bits() as i64;
    state.set_word(*dest, Word::bool(a >= b));
    Transfer::Continue
}

/// Execute unsigned less than comparison.
#[inline(always)]
pub(crate) fn execute_lt_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    state.set_word(*dest, Word::bool(a < b));
    Transfer::Continue
}

/// Execute unsigned less than or equal comparison.
#[inline(always)]
pub(crate) fn execute_le_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    state.set_word(*dest, Word::bool(a <= b));
    Transfer::Continue
}

/// Execute unsigned greater than comparison.
#[inline(always)]
pub(crate) fn execute_gt_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    state.set_word(*dest, Word::bool(a > b));
    Transfer::Continue
}

/// Execute unsigned greater than or equal comparison.
#[inline(always)]
pub(crate) fn execute_ge_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinarySpecialized {
        dest, left, right, ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = state.get(*right).bits();
    state.set_word(*dest, Word::bool(a >= b));
    Transfer::Continue
}

// ============================================================================
// specialized handlers with constant right operand
// ============================================================================

/// Execute integer addition with constant right operand.
#[inline(always)]
pub(crate) fn execute_add_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_add(b), width));
    Transfer::Continue
}

/// Execute integer subtraction with constant right operand.
#[inline(always)]
pub(crate) fn execute_sub_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_sub(b), width));
    Transfer::Continue
}

/// Execute integer multiplication with constant right operand.
#[inline(always)]
pub(crate) fn execute_mul_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::int(a.wrapping_mul(b), width));
    Transfer::Continue
}

/// Execute equality comparison with constant right operand.
#[inline(always)]
pub(crate) fn execute_eq_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    state.set_word(*dest, Word::bool(a == b));
    Transfer::Continue
}

/// Execute inequality comparison with constant right operand.
#[inline(always)]
pub(crate) fn execute_ne_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    state.set_word(*dest, Word::bool(a != b));
    Transfer::Continue
}

/// Execute signed less than with constant right operand.
#[inline(always)]
pub(crate) fn execute_lt_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    state.set_word(*dest, Word::bool(a < b));
    Transfer::Continue
}

/// Execute signed less equal with constant right operand.
#[inline(always)]
pub(crate) fn execute_le_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    state.set_word(*dest, Word::bool(a <= b));
    Transfer::Continue
}

/// Execute signed greater than with constant right operand.
#[inline(always)]
pub(crate) fn execute_gt_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    state.set_word(*dest, Word::bool(a > b));
    Transfer::Continue
}

/// Execute signed greater equal with constant right operand.
#[inline(always)]
pub(crate) fn execute_ge_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits() as i64;
    let b = right_const.bits() as i64;
    state.set_word(*dest, Word::bool(a >= b));
    Transfer::Continue
}

// ============================================================================
// unsigned handlers with constant right operand
// ============================================================================

/// Execute unsigned addition with constant right operand.
#[inline(always)]
pub(crate) fn execute_add_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = right_const.bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_add(b), width));
    Transfer::Continue
}

/// Execute unsigned subtraction with constant right operand.
#[inline(always)]
pub(crate) fn execute_sub_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = right_const.bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_sub(b), width));
    Transfer::Continue
}

/// Execute unsigned multiplication with constant right operand.
#[inline(always)]
pub(crate) fn execute_mul_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = right_const.bits();
    let width = binary_integer_width(&block[pc].operands);
    state.set_word(*dest, Word::uint(a.wrapping_mul(b), width));
    Transfer::Continue
}

/// Execute unsigned less than with constant right operand.
#[inline(always)]
pub(crate) fn execute_lt_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = right_const.bits();
    state.set_word(*dest, Word::bool(a < b));
    Transfer::Continue
}

/// Execute unsigned less equal with constant right operand.
#[inline(always)]
pub(crate) fn execute_le_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = right_const.bits();
    state.set_word(*dest, Word::bool(a <= b));
    Transfer::Continue
}

/// Execute unsigned greater than with constant right operand.
#[inline(always)]
pub(crate) fn execute_gt_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = right_const.bits();
    state.set_word(*dest, Word::bool(a > b));
    Transfer::Continue
}

/// Execute unsigned greater equal with constant right operand.
#[inline(always)]
pub(crate) fn execute_ge_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
        ..
    } = &block[pc].operands
    else {
        unreachable!()
    };
    let a = state.get(*left).bits();
    let b = right_const.bits();
    state.set_word(*dest, Word::bool(a >= b));
    Transfer::Continue
}

/// Execute elementwise binary opcode on vector or tensor values.
pub(crate) fn execute_binary_elementwise(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::BinaryElementwise {
        dest,
        op,
        left,
        right,
        result_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let result_type_id = *result_type;
    let result_type = state.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector {
            lanes: elements, ..
        } => {
            let left_value = state.get(*left);
            let right_value = state.get(*right);
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

            if let Err(error) = super::bytes::write_frame_elements(
                state,
                *dest,
                |state, element_index, _value_type| {
                    let left_type = state.value_type(*left)?;
                    let right_type = state.value_type(*right)?;
                    let lhs = load_vector_element_at(state, left_value, left_type, element_index)?;
                    let rhs =
                        load_vector_element_at(state, right_value, right_type, element_index)?;

                    operator::execute_binary(*op, lhs, rhs)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout_info(state.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let left_value = state.get(*left);
            let right_value = state.get(*right);
            if let Err(error) = super::bytes::write_frame_elements(
                state,
                *dest,
                |state, element_index, _value_type| {
                    if element_index >= layout.storage_len {
                        return Err(Error::IndexOutOfBounds {
                            index: element_index as u64,
                            length: layout.storage_len as u64,
                        });
                    }

                    let left_type = state.value_type(*left)?;
                    let right_type = state.value_type(*right)?;
                    let lhs = load_tensor_element_at(state, left_value, left_type, element_index)?;
                    let rhs =
                        load_tensor_element_at(state, right_value, right_type, element_index)?;

                    operator::execute_binary(*op, lhs, rhs)
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
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Unary { dest, op, arg } = &block[pc].operands else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute opcode
    let result = match operator::execute_unary(*op, argument) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute elementwise unary opcode on vector or tensor values.
pub(crate) fn execute_unary_elementwise(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::UnaryElementwise {
        dest,
        op,
        arg,
        result_type,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    let result_type_id = *result_type;
    let result_type = state.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector {
            lanes: elements, ..
        } => {
            let argument = state.get(*arg);
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

            if let Err(error) = super::bytes::write_frame_elements(
                state,
                *dest,
                |state, element_index, _value_type| {
                    let argument_type = state.value_type(*arg)?;
                    let value =
                        load_vector_element_at(state, argument, argument_type, element_index)?;

                    operator::execute_unary(*op, value)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout_info(state.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let argument = state.get(*arg);
            if let Err(error) = super::bytes::write_frame_elements(
                state,
                *dest,
                |state, element_index, _value_type| {
                    if element_index >= layout.storage_len {
                        return Err(Error::IndexOutOfBounds {
                            index: element_index as u64,
                            length: layout.storage_len as u64,
                        });
                    }

                    let argument_type = state.value_type(*arg)?;
                    let value =
                        load_tensor_element_at(state, argument, argument_type, element_index)?;

                    operator::execute_unary(*op, value)
                },
            ) {
                return Transfer::Error(error);
            }
            Transfer::Continue
        }
        _ => Transfer::Error(Error::InvalidInstruction),
    }
}

/// Execute signed integer unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Unary { dest, op, arg } = &block[pc].operands else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute opcode
    let result = match operator::execute_unary_int(*op, argument) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute unsigned integer unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Unary { dest, op, arg } = &block[pc].operands else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute opcode
    let result = match operator::execute_unary_uint(*op, argument) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute float32 unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_float32(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Unary { dest, op, arg } = &block[pc].operands else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute opcode
    let result = match operator::execute_unary_float32(*op, argument) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute float64 unary opcode.
#[inline(always)]
pub(crate) fn execute_unary_float64(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Unary { dest, op, arg } = &block[pc].operands else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute opcode
    let result = match operator::execute_unary_float64(*op, argument) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}

/// Execute boolean unary opcode.
pub(crate) fn execute_unary_bool(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Unary { dest, op, arg } = &block[pc].operands else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute opcode
    let result = match operator::execute_unary_bool(*op, argument) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    // store result
    state.set_word(*dest, result);

    // continue to next instruction
    Transfer::Continue
}
