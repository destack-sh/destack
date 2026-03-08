use super::*;

/// Handle constant load.
pub(crate) fn handle_const(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Const { dest, value } = &block[pc].data else {
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

/// Handle binary operation.
pub(crate) fn handle_binary(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle signed integer binary operation.
#[inline(always)]
pub(crate) fn handle_binary_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_int(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle unsigned integer binary operation.
#[inline(always)]
pub(crate) fn handle_binary_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_uint(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float32 binary operation.
#[inline(always)]
pub(crate) fn handle_binary_float32(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_float32(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float64 binary operation.
#[inline(always)]
pub(crate) fn handle_binary_float64(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_float64(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle boolean binary operation.
pub(crate) fn handle_binary_bool(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Binary {
        dest,
        op,
        left,
        right,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // execute operation
    let result = match operator::execute_binary_bool(*op, lhs, rhs) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

// ============================================================================
// Specialized Integer Arithmetic Handlers
// ============================================================================

/// Execute integer addition without operator dispatch.
#[inline(always)]
pub(crate) fn handle_add_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_sub_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_mul_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_and_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_or_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_xor_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_shl_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_shr_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as u32;
    let width = state.get(*left).width();
    state.set(*dest, Value::int(a.wrapping_shr(b), width));
    next!(state, block, pc)
}

// ============================================================================
// Specialized Unsigned Integer Arithmetic Handlers
// ============================================================================

/// Execute unsigned integer addition without operator dispatch.
#[inline(always)]
pub(crate) fn handle_add_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_sub_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_mul_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_and_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_or_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_xor_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_shl_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
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
pub(crate) fn handle_shr_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data() as u32;
    let width = state.get(*left).width();
    state.set(*dest, Value::uint(a.wrapping_shr(b), width));
    next!(state, block, pc)
}

// ============================================================================
// Specialized Integer Comparison Handlers
// ============================================================================

/// Execute integer equality comparison.
#[inline(always)]
pub(crate) fn handle_eq_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a == b));
    next!(state, block, pc)
}

/// Execute integer inequality comparison.
#[inline(always)]
pub(crate) fn handle_ne_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a != b));
    next!(state, block, pc)
}

/// Execute signed less than comparison.
#[inline(always)]
pub(crate) fn handle_lt_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a < b));
    next!(state, block, pc)
}

/// Execute signed less than or equal comparison.
#[inline(always)]
pub(crate) fn handle_le_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a <= b));
    next!(state, block, pc)
}

/// Execute signed greater than comparison.
#[inline(always)]
pub(crate) fn handle_gt_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a > b));
    next!(state, block, pc)
}

/// Execute signed greater than or equal comparison.
#[inline(always)]
pub(crate) fn handle_ge_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = state.get(*right).raw_data() as i64;
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

/// Execute unsigned less than comparison.
#[inline(always)]
pub(crate) fn handle_lt_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a < b));
    next!(state, block, pc)
}

/// Execute unsigned less than or equal comparison.
#[inline(always)]
pub(crate) fn handle_le_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a <= b));
    next!(state, block, pc)
}

/// Execute unsigned greater than comparison.
#[inline(always)]
pub(crate) fn handle_gt_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a > b));
    next!(state, block, pc)
}

/// Execute unsigned greater than or equal comparison.
#[inline(always)]
pub(crate) fn handle_ge_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinarySpecialized { dest, left, right } = &block[pc].data else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = state.get(*right).raw_data();
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

// ============================================================================
// Specialized Handlers With Constant Right Operand
// ============================================================================

/// Execute integer addition with constant right operand.
#[inline(always)]
pub(crate) fn handle_add_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_sub_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_mul_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_eq_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_ne_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_lt_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_le_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_gt_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_ge_const_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data() as i64;
    let b = right_const.raw_data() as i64;
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

// ============================================================================
// Unsigned Handlers With Constant Right Operand
// ============================================================================

/// Execute unsigned addition with constant right operand.
#[inline(always)]
pub(crate) fn handle_add_const_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_sub_const_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_mul_const_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_lt_const_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_le_const_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_gt_const_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
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
pub(crate) fn handle_ge_const_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRightSpecialized {
        dest,
        left,
        right_const,
    } = &block[pc].data
    else {
        unreachable!()
    };
    let a = state.get(*left).raw_data();
    let b = right_const.raw_data();
    state.set(*dest, Value::bool(a >= b));
    next!(state, block, pc)
}

// ============================================================================
// Generic Binary With Constant Right Operand
// ============================================================================

/// Execute generic binary operation with constant right operand.
#[inline(always)]
pub(crate) fn handle_binary_const_right(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryConstRight {
        dest,
        op,
        left,
        right_const,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let left_value = state.get(*left);
    let result = match operator::execute_binary(*op, left_value, *right_const) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    state.set(*dest, result);
    next!(state, block, pc)
}

/// Execute elementwise binary operation on vector or tensor values.
pub(crate) fn handle_binary_elementwise(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::BinaryElementwise {
        dest,
        op,
        left,
        right,
        result_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let result_type_id = *result_type;
    let result_type = state.interpreter.isolate.image.tree.get(result_type_id);
    match result_type {
        mir::Type::Vector { lanes, .. } => {
            let left_value = state.get(*left);
            let right_value = state.get(*right);
            let left_slots = match aggregate_slots(state, left_value) {
                Ok(slots) => slots,
                Err(error) => return ControlFlow::Error(error),
            };
            let right_slots = match aggregate_slots(state, right_value) {
                Ok(slots) => slots,
                Err(error) => return ControlFlow::Error(error),
            };
            let expected = *lanes as usize;
            if left_slots.len() != expected || right_slots.len() != expected {
                return ControlFlow::Error(Error::TypeMismatch {
                    expected: "matching vector lanes".to_string(),
                    actual: format!("{} vs {}", left_slots.len(), right_slots.len()),
                });
            }

            let mut output = Vec::with_capacity(expected);
            for (lhs, rhs) in left_slots.iter().zip(right_slots.iter()) {
                let value = match operator::execute_binary(*op, *lhs, *rhs) {
                    Ok(value) => value,
                    Err(error) => return ControlFlow::Error(error),
                };
                output.push(value);
            }

            let result = state.interpreter.allocate_aggregate(output);
            state.set(*dest, result);
            next!(state, block, pc)
        }
        mir::Type::Tensor { .. } => {
            let layout =
                match tensor_layout_info(&state.interpreter.isolate.image.tree, result_type_id) {
                    Ok(layout) => layout,
                    Err(error) => return ControlFlow::Error(error),
                };
            let left_value = state.get(*left);
            let right_value = state.get(*right);
            let left_slots = match aggregate_slots(state, left_value) {
                Ok(slots) => slots,
                Err(error) => return ControlFlow::Error(error),
            };
            let right_slots = match aggregate_slots(state, right_value) {
                Ok(slots) => slots,
                Err(error) => return ControlFlow::Error(error),
            };
            if left_slots.len() != layout.storage_len || right_slots.len() != layout.storage_len {
                return ControlFlow::Error(Error::TypeMismatch {
                    expected: "matching tensor elements".to_string(),
                    actual: format!("{} vs {}", left_slots.len(), right_slots.len()),
                });
            }

            let mut output = Vec::with_capacity(layout.storage_len);
            for (lhs, rhs) in left_slots.iter().zip(right_slots.iter()) {
                let value = match operator::execute_binary(*op, *lhs, *rhs) {
                    Ok(value) => value,
                    Err(error) => return ControlFlow::Error(error),
                };
                output.push(value);
            }

            let result = state.interpreter.allocate_aggregate(output);
            state.set(*dest, result);
            next!(state, block, pc)
        }
        _ => ControlFlow::Error(Error::InvalidInstruction),
    }
}

/// Handle unary operation.
pub(crate) fn handle_unary(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Execute elementwise unary operation on vector or tensor values.
pub(crate) fn handle_unary_elementwise(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    let ThreadedInstructionData::UnaryElementwise {
        dest,
        op,
        arg,
        result_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    let result_type_id = *result_type;
    let result_type = state.interpreter.isolate.image.tree.get(result_type_id);
    match result_type {
        mir::Type::Vector { lanes, .. } => {
            let argument = state.get(*arg);
            let slots = match aggregate_slots(state, argument) {
                Ok(slots) => slots,
                Err(error) => return ControlFlow::Error(error),
            };
            let expected = *lanes as usize;
            if slots.len() != expected {
                return ControlFlow::Error(Error::TypeMismatch {
                    expected: "matching vector lanes".to_string(),
                    actual: slots.len().to_string(),
                });
            }

            let mut output = Vec::with_capacity(expected);
            for value in slots.iter() {
                let result = match operator::execute_unary(*op, *value) {
                    Ok(value) => value,
                    Err(error) => return ControlFlow::Error(error),
                };
                output.push(result);
            }

            let result = state.interpreter.allocate_aggregate(output);
            state.set(*dest, result);
            next!(state, block, pc)
        }
        mir::Type::Tensor { .. } => {
            let layout =
                match tensor_layout_info(&state.interpreter.isolate.image.tree, result_type_id) {
                    Ok(layout) => layout,
                    Err(error) => return ControlFlow::Error(error),
                };
            let argument = state.get(*arg);
            let slots = match aggregate_slots(state, argument) {
                Ok(slots) => slots,
                Err(error) => return ControlFlow::Error(error),
            };
            if slots.len() != layout.storage_len {
                return ControlFlow::Error(Error::TypeMismatch {
                    expected: "matching tensor elements".to_string(),
                    actual: slots.len().to_string(),
                });
            }

            let mut output = Vec::with_capacity(layout.storage_len);
            for value in slots.iter() {
                let result = match operator::execute_unary(*op, *value) {
                    Ok(value) => value,
                    Err(error) => return ControlFlow::Error(error),
                };
                output.push(result);
            }

            let result = state.interpreter.allocate_aggregate(output);
            state.set(*dest, result);
            next!(state, block, pc)
        }
        _ => ControlFlow::Error(Error::InvalidInstruction),
    }
}

/// Handle signed integer unary operation.
#[inline(always)]
pub(crate) fn handle_unary_int(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_int(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle unsigned integer unary operation.
#[inline(always)]
pub(crate) fn handle_unary_uint(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_uint(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float32 unary operation.
#[inline(always)]
pub(crate) fn handle_unary_float32(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_float32(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle float64 unary operation.
#[inline(always)]
pub(crate) fn handle_unary_float64(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_float64(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle boolean unary operation.
pub(crate) fn handle_unary_bool(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Unary { dest, op, arg } = &block[pc].data else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute operation
    let result = match operator::execute_unary_bool(*op, argument) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}
