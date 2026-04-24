use super::prelude::*;

/// Return the lane count for one vector value id.
fn vector_lane_count(state: &StepState<'_, '_>, value_id: mir::Value) -> Result<usize, Error> {
    let vector_type = state.value_type(value_id)?;

    match state.tree().get(vector_type) {
        mir::Type::Vector { lanes, .. } => Ok(*lanes as usize),
        _ => Err(Error::TypeMismatch {
            expected: "vector type".to_string(),
            actual: format!("{vector_type:?}"),
        }),
    }
}

/// Load one vector lane through indexed storage.
fn vector_lane_value(
    state: &mut StepState<'_, '_>,
    vector: Value,
    vector_type: mir::LocalNodeId<mir::Type>,
    lane_index: usize,
) -> Result<Value, Error> {
    let index = u32::try_from(lane_index).map_err(|_| Error::TypeMismatch {
        expected: "vector lane index".to_string(),
        actual: lane_index.to_string(),
    })?;
    let (element, element_count) =
        access::element_access_for_type(state, vector_type, index.into())?;

    access::get_element(state, vector, index.into(), element_count, Some(element))
}

/// Load one tensor storage slot through indexed storage.
fn tensor_slot_value(
    state: &mut StepState<'_, '_>,
    tensor: Value,
    tensor_type: mir::LocalNodeId<mir::Type>,
    slot_index: usize,
) -> Result<Value, Error> {
    let index = u32::try_from(slot_index).map_err(|_| Error::TypeMismatch {
        expected: "tensor storage slot".to_string(),
        actual: slot_index.to_string(),
    })?;
    let (element, element_count) =
        access::element_access_for_type(state, tensor_type, index.into())?;

    access::get_element(state, tensor, index.into(), element_count, Some(element))
}

/// Step constant load.
pub(crate) fn step_const(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Const { dest, value } = &block[pc].immediate else {
        unreachable!()
    };

    // resolve constant value
    let ConstValue::Value(value) = value;
    let value = *value;

    // write value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step binary opcode.
pub(crate) fn step_binary(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].immediate
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step signed integer binary opcode.
#[inline(always)]
pub(crate) fn step_binary_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].immediate
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step unsigned integer binary opcode.
#[inline(always)]
pub(crate) fn step_binary_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].immediate
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step float32 binary opcode.
#[inline(always)]
pub(crate) fn step_binary_float32(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].immediate
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step float64 binary opcode.
#[inline(always)]
pub(crate) fn step_binary_float64(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].immediate
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step boolean binary opcode.
pub(crate) fn step_binary_bool(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].immediate
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

// ============================================================================
// specialized integer arithmetic handlers
// ============================================================================

/// Execute integer addition without operator dispatch.
#[inline(always)]
pub(crate) fn step_add_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_add(b), width));
    next!(state, block, pc)
}

/// Execute integer subtraction without operator dispatch.
#[inline(always)]
pub(crate) fn step_sub_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_sub(b), width));
    next!(state, block, pc)
}

/// Execute integer multiplication without operator dispatch.
#[inline(always)]
pub(crate) fn step_mul_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_mul(b), width));
    next!(state, block, pc)
}

/// Execute integer bitwise AND without operator dispatch.
#[inline(always)]
pub(crate) fn step_and_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a & b, width));
    next!(state, block, pc)
}

/// Execute integer bitwise OR without operator dispatch.
#[inline(always)]
pub(crate) fn step_or_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a | b, width));
    next!(state, block, pc)
}

/// Execute integer bitwise XOR without operator dispatch.
#[inline(always)]
pub(crate) fn step_xor_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a ^ b, width));
    next!(state, block, pc)
}

/// Execute shift left without operator dispatch.
#[inline(always)]
pub(crate) fn step_shl_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as u32;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_shl(b), width));
    next!(state, block, pc)
}

/// Execute arithmetic shift right without operator dispatch.
#[inline(always)]
pub(crate) fn step_shr_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as u32;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_shr(b), width));
    next!(state, block, pc)
}

// ============================================================================
// specialized unsigned integer arithmetic handlers
// ============================================================================

/// Execute unsigned integer addition without operator dispatch.
#[inline(always)]
pub(crate) fn step_add_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_add(b), width));
    next!(state, block, pc)
}

/// Execute unsigned integer subtraction without operator dispatch.
#[inline(always)]
pub(crate) fn step_sub_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_sub(b), width));
    next!(state, block, pc)
}

/// Execute unsigned integer multiplication without operator dispatch.
#[inline(always)]
pub(crate) fn step_mul_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_mul(b), width));
    next!(state, block, pc)
}

/// Execute unsigned bitwise AND without operator dispatch.
#[inline(always)]
pub(crate) fn step_and_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a & b, width));
    next!(state, block, pc)
}

/// Execute unsigned bitwise OR without operator dispatch.
#[inline(always)]
pub(crate) fn step_or_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a | b, width));
    next!(state, block, pc)
}

/// Execute unsigned bitwise XOR without operator dispatch.
#[inline(always)]
pub(crate) fn step_xor_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a ^ b, width));
    next!(state, block, pc)
}

/// Execute unsigned shift left without operator dispatch.
#[inline(always)]
pub(crate) fn step_shl_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data() as u32;
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_shl(b), width));
    next!(state, block, pc)
}

/// Execute logical shift right for unsigned values.
#[inline(always)]
pub(crate) fn step_shr_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data() as u32;
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_shr(b), width));
    next!(state, block, pc)
}

// ============================================================================
// specialized integer comparison handlers
// ============================================================================

/// Execute integer equality comparison.
#[inline(always)]
pub(crate) fn step_eq_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a == b));
    next!(state, block, pc)
}

/// Execute integer inequality comparison.
#[inline(always)]
pub(crate) fn step_ne_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a != b));
    next!(state, block, pc)
}

/// Execute signed less than comparison.
#[inline(always)]
pub(crate) fn step_lt_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a < b));
    next!(state, block, pc)
}

/// Execute signed less than or equal comparison.
#[inline(always)]
pub(crate) fn step_le_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a <= b));
    next!(state, block, pc)
}

/// Execute signed greater than comparison.
#[inline(always)]
pub(crate) fn step_gt_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a > b));
    next!(state, block, pc)
}

/// Execute signed greater than or equal comparison.
#[inline(always)]
pub(crate) fn step_ge_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

/// Execute unsigned less than comparison.
#[inline(always)]
pub(crate) fn step_lt_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a < b));
    next!(state, block, pc)
}

/// Execute unsigned less than or equal comparison.
#[inline(always)]
pub(crate) fn step_le_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a <= b));
    next!(state, block, pc)
}

/// Execute unsigned greater than comparison.
#[inline(always)]
pub(crate) fn step_gt_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a > b));
    next!(state, block, pc)
}

/// Execute unsigned greater than or equal comparison.
#[inline(always)]
pub(crate) fn step_ge_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinarySpecialized { dest, left, right } = &block[pc].immediate else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

// ============================================================================
// specialized handlers with constant right operand
// ============================================================================

/// Execute integer addition with constant right operand.
#[inline(always)]
pub(crate) fn step_add_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_add(b), width));
    next!(state, block, pc)
}

/// Execute integer subtraction with constant right operand.
#[inline(always)]
pub(crate) fn step_sub_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_sub(b), width));
    next!(state, block, pc)
}

/// Execute integer multiplication with constant right operand.
#[inline(always)]
pub(crate) fn step_mul_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_mul(b), width));
    next!(state, block, pc)
}

/// Execute equality comparison with constant right operand.
#[inline(always)]
pub(crate) fn step_eq_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    state.set(*dest, Value::bool(a == b));
    next!(state, block, pc)
}

/// Execute inequality comparison with constant right operand.
#[inline(always)]
pub(crate) fn step_ne_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    state.set(*dest, Value::bool(a != b));
    next!(state, block, pc)
}

/// Execute signed less than with constant right operand.
#[inline(always)]
pub(crate) fn step_lt_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    state.set(*dest, Value::bool(a < b));
    next!(state, block, pc)
}

/// Execute signed less equal with constant right operand.
#[inline(always)]
pub(crate) fn step_le_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    state.set(*dest, Value::bool(a <= b));
    next!(state, block, pc)
}

/// Execute signed greater than with constant right operand.
#[inline(always)]
pub(crate) fn step_gt_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    state.set(*dest, Value::bool(a > b));
    next!(state, block, pc)
}

/// Execute signed greater equal with constant right operand.
#[inline(always)]
pub(crate) fn step_ge_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

// ============================================================================
// unsigned handlers with constant right operand
// ============================================================================

/// Execute unsigned addition with constant right operand.
#[inline(always)]
pub(crate) fn step_add_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_add(b), width));
    next!(state, block, pc)
}

/// Execute unsigned subtraction with constant right operand.
#[inline(always)]
pub(crate) fn step_sub_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_sub(b), width));
    next!(state, block, pc)
}

/// Execute unsigned multiplication with constant right operand.
#[inline(always)]
pub(crate) fn step_mul_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_mul(b), width));
    next!(state, block, pc)
}

/// Execute unsigned less than with constant right operand.
#[inline(always)]
pub(crate) fn step_lt_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    state.set(*dest, Value::bool(a < b));
    next!(state, block, pc)
}

/// Execute unsigned less equal with constant right operand.
#[inline(always)]
pub(crate) fn step_le_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    state.set(*dest, Value::bool(a <= b));
    next!(state, block, pc)
}

/// Execute unsigned greater than with constant right operand.
#[inline(always)]
pub(crate) fn step_gt_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    state.set(*dest, Value::bool(a > b));
    next!(state, block, pc)
}

/// Execute unsigned greater equal with constant right operand.
#[inline(always)]
pub(crate) fn step_ge_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

// ============================================================================
// generic binary with constant right operand
// ============================================================================

/// Execute generic binary opcode with constant right operand.
#[inline(always)]
pub(crate) fn step_binary_const_right(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryConstRight {
        dest,
        op,
        left,
        right_const,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    let left_value = state.get(*left);
    let result = match operator::execute_binary(*op, left_value, *right_const) {
        Ok(v) => v,
        Err(e) => return Transfer::Error(e),
    };

    state.set(*dest, result);
    next!(state, block, pc)
}

/// Execute elementwise binary opcode on vector or tensor values.
pub(crate) fn step_binary_elementwise(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::BinaryElementwise {
        dest,
        op,
        left,
        right,
        result_type,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    let result_type_id = *result_type;
    let result_type = state.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector { lanes, .. } => {
            let left_value = state.get(*left);
            let right_value = state.get(*right);
            let expected = lanes as usize;
            let left_lane_count = match vector_lane_count(state, *left) {
                Ok(lanes) => lanes,
                Err(error) => return Transfer::Error(error),
            };
            let right_lane_count = match vector_lane_count(state, *right) {
                Ok(lanes) => lanes,
                Err(error) => return Transfer::Error(error),
            };
            if left_lane_count != expected || right_lane_count != expected {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "matching vector lanes".to_string(),
                    actual: format!("{left_lane_count} vs {right_lane_count}"),
                });
            }

            let result = match materialize_composite_by_index(
                state,
                *dest,
                |state, lane_index, _value_type| {
                    let lane_index =
                        usize::try_from(lane_index).map_err(|_| Error::TypeMismatch {
                            expected: "vector lane index".to_string(),
                            actual: lane_index.to_string(),
                        })?;
                    let left_type = state.value_type(*left)?;
                    let right_type = state.value_type(*right)?;
                    let lhs = vector_lane_value(state, left_value, left_type, lane_index)?;
                    let rhs = vector_lane_value(state, right_value, right_type, lane_index)?;

                    operator::execute_binary(*op, lhs, rhs)
                },
            ) {
                Ok(result) => result,
                Err(error) => return Transfer::Error(error),
            };
            state.set(*dest, result);
            next!(state, block, pc)
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout_info(state.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let left_value = state.get(*left);
            let right_value = state.get(*right);
            let result = match materialize_composite_by_index(
                state,
                *dest,
                |state, slot_index, _value_type| {
                    let slot_index =
                        usize::try_from(slot_index).map_err(|_| Error::TypeMismatch {
                            expected: "tensor storage slot".to_string(),
                            actual: slot_index.to_string(),
                        })?;
                    if slot_index >= layout.storage_len {
                        return Err(Error::IndexOutOfBounds {
                            index: slot_index as u64,
                            length: layout.storage_len as u64,
                        });
                    }

                    let left_type = state.value_type(*left)?;
                    let right_type = state.value_type(*right)?;
                    let lhs = tensor_slot_value(state, left_value, left_type, slot_index)?;
                    let rhs = tensor_slot_value(state, right_value, right_type, slot_index)?;

                    operator::execute_binary(*op, lhs, rhs)
                },
            ) {
                Ok(result) => result,
                Err(error) => return Transfer::Error(error),
            };
            state.set(*dest, result);
            next!(state, block, pc)
        }
        _ => Transfer::Error(Error::InvalidInstruction),
    }
}

/// Step unary opcode.
pub(crate) fn step_unary(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Unary { dest, op, arg } = &block[pc].immediate else {
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute elementwise unary opcode on vector or tensor values.
pub(crate) fn step_unary_elementwise(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Immediate::UnaryElementwise {
        dest,
        op,
        arg,
        result_type,
    } = &block[pc].immediate
    else {
        unreachable!()
    };

    let result_type_id = *result_type;
    let result_type = state.tree().get(result_type_id).clone();
    match result_type {
        mir::Type::Vector { lanes, .. } => {
            let argument = state.get(*arg);
            let expected = lanes as usize;
            let lane_count = match vector_lane_count(state, *arg) {
                Ok(lanes) => lanes,
                Err(error) => return Transfer::Error(error),
            };
            if lane_count != expected {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "matching vector lanes".to_string(),
                    actual: lane_count.to_string(),
                });
            }

            let result = match materialize_composite_by_index(
                state,
                *dest,
                |state, lane_index, _value_type| {
                    let lane_index =
                        usize::try_from(lane_index).map_err(|_| Error::TypeMismatch {
                            expected: "vector lane index".to_string(),
                            actual: lane_index.to_string(),
                        })?;
                    let argument_type = state.value_type(*arg)?;
                    let value = vector_lane_value(state, argument, argument_type, lane_index)?;

                    operator::execute_unary(*op, value)
                },
            ) {
                Ok(result) => result,
                Err(error) => return Transfer::Error(error),
            };
            state.set(*dest, result);
            next!(state, block, pc)
        }
        mir::Type::Tensor { .. } => {
            let layout = match tensor_layout_info(state.tree(), result_type_id) {
                Ok(layout) => layout,
                Err(error) => return Transfer::Error(error),
            };
            let argument = state.get(*arg);
            let result = match materialize_composite_by_index(
                state,
                *dest,
                |state, slot_index, _value_type| {
                    let slot_index =
                        usize::try_from(slot_index).map_err(|_| Error::TypeMismatch {
                            expected: "tensor storage slot".to_string(),
                            actual: slot_index.to_string(),
                        })?;
                    if slot_index >= layout.storage_len {
                        return Err(Error::IndexOutOfBounds {
                            index: slot_index as u64,
                            length: layout.storage_len as u64,
                        });
                    }

                    let argument_type = state.value_type(*arg)?;
                    let value = tensor_slot_value(state, argument, argument_type, slot_index)?;

                    operator::execute_unary(*op, value)
                },
            ) {
                Ok(result) => result,
                Err(error) => return Transfer::Error(error),
            };
            state.set(*dest, result);
            next!(state, block, pc)
        }
        _ => Transfer::Error(Error::InvalidInstruction),
    }
}

/// Step signed integer unary opcode.
#[inline(always)]
pub(crate) fn step_unary_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Unary { dest, op, arg } = &block[pc].immediate else {
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step unsigned integer unary opcode.
#[inline(always)]
pub(crate) fn step_unary_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Unary { dest, op, arg } = &block[pc].immediate else {
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step float32 unary opcode.
#[inline(always)]
pub(crate) fn step_unary_float32(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Unary { dest, op, arg } = &block[pc].immediate else {
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step float64 unary opcode.
#[inline(always)]
pub(crate) fn step_unary_float64(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Unary { dest, op, arg } = &block[pc].immediate else {
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Step boolean unary opcode.
pub(crate) fn step_unary_bool(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction immediate
    let Immediate::Unary { dest, op, arg } = &block[pc].immediate else {
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
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
