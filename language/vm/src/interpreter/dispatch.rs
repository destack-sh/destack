#![allow(elided_lifetimes_in_paths)]

use std::ptr::NonNull;

use destack_mir as mir;
use smallvec::SmallVec;

use crate::diagnostic::Error;
use crate::memory::{ReferenceMeta, Value, ValueTag};

use super::call::copy_values_with_plan;
use super::statistics::stat_inc;
use super::threaded::{
    ArgumentRange, ControlFlow, INVALID_FUNCTION_INDEX, INVALID_VALUE_ID, ThreadedFunction,
    ThreadedInstruction, ThreadedInstructionData, ThreadedState, UNKNOWN_SLOT_COUNT,
    is_invalid_value,
};
use super::{Frame, instruction, operator, resize_and_clear_stack};

// helper macro: do work, then become next handler
macro_rules! next {
    ($state:expr, $block:expr, $pc:expr) => {{
        $state.maybe_profile_instruction(&$block[$pc]);
        let next_pc = $pc + 1;
        become ($block[next_pc].handler)($state, $block, next_pc)
    }};
}

/// Build a global id from a raw value.
#[inline]
fn global_id(raw: u32) -> mir::LocalNodeId<mir::Global> {
    mir::LocalNodeId::new(raw)
}

/// Build a type id from a raw value.
#[inline]
fn type_id(raw: u32) -> mir::LocalNodeId<mir::Type> {
    mir::LocalNodeId::new(raw)
}

/// Collect argument values into a smallvec.
#[inline]
fn collect_values(state: &mut ThreadedState, arguments: ArgumentRange) -> SmallVec<[Value; 16]> {
    // load argument slice
    let argument_slice = state.argument_slice(arguments);
    let mut args = SmallVec::with_capacity(argument_slice.len());

    // resolve argument values
    for arg in argument_slice {
        let value = state.get(*arg);
        args.push(value);
    }

    // return argument values
    args
}

/// Format a reference kind label for diagnostics.
fn reference_label(reference: ReferenceMeta) -> String {
    match reference.kind() {
        Some(kind) => format!("{kind:?}"),
        None => "unknown".to_string(),
    }
}

/// Validate reference kind against the pointer storage.
fn check_reference_kind(
    state: &ThreadedState,
    reference: ReferenceMeta,
    pointer: Value,
) -> Result<(), Error> {
    if !state.interpreter.options.enforce_reference_kinds {
        return Ok(());
    }

    let Some(kind) = reference.kind() else {
        return Ok(());
    };

    let is_managed = pointer.tag() == ValueTag::ManagedReference;
    match kind {
        mir::ReferenceKind::Managed if !is_managed => Err(Error::InvalidReferenceKind {
            reference: reference_label(reference),
            actual: format!("{pointer:?}"),
        }),
        mir::ReferenceKind::Owned | mir::ReferenceKind::Borrowed | mir::ReferenceKind::Raw
            if is_managed =>
        {
            Err(Error::InvalidReferenceKind {
                reference: reference_label(reference),
                actual: format!("{pointer:?}"),
            })
        }
        _ => Ok(()),
    }
}

/// Validate reference mutability for stores.
fn check_reference_mutability(
    state: &ThreadedState,
    reference: ReferenceMeta,
) -> Result<(), Error> {
    if !state.interpreter.options.enforce_reference_mutability {
        return Ok(());
    }

    let Some(mutability) = reference.mutability() else {
        return Ok(());
    };

    if matches!(mutability, mir::Mutability::Immutable) {
        return Err(Error::ImmutableReferenceWrite {
            reference: reference_label(reference),
        });
    }

    Ok(())
}

/// Handle constant load.
pub(super) fn handle_const(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Const { dest, value } = &block[pc].data else {
        unreachable!()
    };

    // write value
    state.set(*dest, *value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle binary operation.
pub(super) fn handle_binary(
    state: &mut ThreadedState,
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
pub(super) fn handle_binary_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_binary_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_binary_float32(
    state: &mut ThreadedState,
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
pub(super) fn handle_binary_float64(
    state: &mut ThreadedState,
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
pub(super) fn handle_binary_bool(
    state: &mut ThreadedState,
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
pub(super) fn handle_add_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_sub_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_mul_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_and_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_or_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_xor_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_shl_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_shr_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_add_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_sub_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_mul_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_and_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_or_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_xor_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_shl_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_shr_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_eq_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_ne_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_lt_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_le_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_gt_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_ge_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_lt_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_le_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_gt_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_ge_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_add_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_sub_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_mul_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_eq_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_ne_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_lt_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_le_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_gt_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_ge_const_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_add_const_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_sub_const_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_mul_const_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_lt_const_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_le_const_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_gt_const_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_ge_const_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_binary_const_right(
    state: &mut ThreadedState,
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

/// Handle unary operation.
pub(super) fn handle_unary(
    state: &mut ThreadedState,
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

/// Handle signed integer unary operation.
#[inline(always)]
pub(super) fn handle_unary_int(
    state: &mut ThreadedState,
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
pub(super) fn handle_unary_uint(
    state: &mut ThreadedState,
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
pub(super) fn handle_unary_float32(
    state: &mut ThreadedState,
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
pub(super) fn handle_unary_float64(
    state: &mut ThreadedState,
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
pub(super) fn handle_unary_bool(
    state: &mut ThreadedState,
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

/// Handle cast operation.
pub(super) fn handle_cast(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Cast {
        dest,
        op,
        arg,
        to_type,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operand
    let argument = state.get(*arg);

    // execute cast
    let cast_type = type_id(*to_type);
    let result = match operator::execute_cast(&state.interpreter.tree, *op, argument, cast_type) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle select operation.
pub(super) fn handle_select(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Select {
        dest,
        condition,
        then_value,
        else_value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load condition and select result
    let cond = state.get(*condition).as_bool().unwrap_or(false);
    let result = if cond {
        state.get(*then_value)
    } else {
        state.get(*else_value)
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle function call (returns to trampoline).
pub(super) fn handle_call(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Call {
        dest,
        function,
        callee_index,
        arguments,
        copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve target function id
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);

    // skip fast path when stats or step limits are active
    let allow_direct = !state.collect_stats && state.interpreter.options.max_instructions.is_none();

    // try direct threaded call when possible
    if allow_direct && !state.interpreter.threaded_functions.is_import(function_id) {
        let resolved_index = if *callee_index == INVALID_FUNCTION_INDEX {
            state.interpreter.threaded_functions.index_for(function_id)
        } else {
            Some(*callee_index)
        };
        let callee_ptr = resolved_index
            .and_then(|index| state.interpreter.threaded_functions.get_ptr_by_index(index));
        if let Some(callee_ptr) = callee_ptr {
            let callee = unsafe { callee_ptr.as_ref() };

            // check stack overflow
            if state.interpreter.call_stack.len() >= state.interpreter.options.max_stack_depth {
                return ControlFlow::Error(Error::StackOverflow);
            }

            // store return destination and resume pc on caller frame
            {
                let caller = state.current_frame_mut();
                caller.return_destination = *dest;
                caller.resume_pc = pc + 1;
            }

            // allocate new frame for callee
            let value_base = state.interpreter.value_stack.len();
            let local_base = state.interpreter.local_stack.len();
            state
                .interpreter
                .value_stack
                .resize(value_base + callee.value_count, Value::VOID);
            state
                .interpreter
                .local_stack
                .resize(local_base + callee.local_count, Value::VOID);
            let entry_block = &callee.blocks[callee.entry as usize];
            let entry_block_id = entry_block.mir_block;
            let entry_block_ptr = NonNull::from(entry_block);
            let new_frame = Frame::new(
                function_id,
                callee_ptr,
                entry_block_ptr,
                entry_block_id,
                callee.entry as usize,
                value_base,
                callee.value_count,
                local_base,
                callee.local_count,
            );

            // bind parameters from caller values
            let caller_index = state.frame_index;
            let current_threaded_ptr = {
                let frame = state.current_frame_mut();
                frame.threaded
            };
            let current_func = unsafe { current_threaded_ptr.as_ref() };
            let caller_ptr = {
                let Ok(caller) = state.frame_by_index(caller_index) else {
                    return ControlFlow::Error(Error::InvalidHeapHandle);
                };
                caller as *const Frame
            };
            let caller = unsafe { &*caller_ptr };
            copy_values_with_plan(
                &mut state.interpreter.value_stack,
                caller,
                &new_frame,
                *copies,
                current_func.copy_pool.as_slice(),
            );

            // push new frame and refresh state
            state.interpreter.call_stack.push(new_frame);
            let new_index = state.interpreter.call_stack.len() - 1;
            state.enter_frame(new_index, callee);

            // continue at entry block
            let entry_instructions = entry_block.instructions.as_slice();
            become (entry_instructions[0].handler)(state, entry_instructions, 0)
        }
    }

    // return control to trampoline
    ControlFlow::Call {
        function: *function,
        callee_index: *callee_index,
        destination: *dest,
        arguments: *arguments,
        copies: Some(*copies),
        resume_pc: pc + 1,
    }
}

/// Handle indirect call (returns to trampoline).
pub(super) fn handle_call_indirect(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CallIndirect {
        dest,
        callee,
        arguments,
        cached_function,
        cached_index,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load callee value
    let callee_val = state.get(*callee);

    // extract function pointer
    let function = match callee_val.as_function_pointer() {
        Some(f) => f.id,
        None => {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "function_pointer".to_string(),
                actual: format!("{callee_val:?}"),
            });
        }
    };

    // reuse cached callee index when possible
    if cached_function.get() == Some(function) {
        let cached_index = cached_index.get().unwrap_or(INVALID_FUNCTION_INDEX);
        return ControlFlow::Call {
            function,
            callee_index: cached_index,
            destination: *dest,
            arguments: *arguments,
            copies: None,
            resume_pc: pc + 1,
        };
    }

    // resolve callee index and update cache
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);
    let resolved_index = state
        .interpreter
        .threaded_functions
        .index_for(function_id)
        .unwrap_or(INVALID_FUNCTION_INDEX);
    cached_function.set(Some(function));
    cached_index.set(Some(resolved_index));

    // return control to trampoline
    ControlFlow::Call {
        function,
        callee_index: resolved_index,
        destination: *dest,
        arguments: *arguments,
        copies: None,
        resume_pc: pc + 1,
    }
}

/// Handle local variable load.
#[inline(always)]
pub(super) fn handle_local_get(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::LocalGet { dest, local } = &block[pc].data else {
        unreachable!()
    };

    // load local value
    let value = state.get_local_by_index(*local);

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle local variable store.
#[inline(always)]
pub(super) fn handle_local_set(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::LocalSet { local, value } = &block[pc].data else {
        unreachable!()
    };

    // load value
    let val = state.get(*value);

    // store local value
    state.set_local_by_index(*local, val);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global address.
pub(super) fn handle_global_addr(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalAddr {
        dest,
        global,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // write global pointer
    let ptr = Value::global_pointer_with_meta(global_id(*global), 0, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, ptr);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global constant load.
pub(super) fn handle_global_const(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalConst { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // load global value
    let global_id = global_id(*global);
    let value = match state.interpreter.globals.get(global_id).copied() {
        Some(v) => v,
        None => return ControlFlow::Error(Error::UndefinedGlobal { global: global_id }),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle fused global address + load.
#[inline(always)]
pub(super) fn handle_global_load(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalLoad { dest, global } = &block[pc].data else {
        unreachable!()
    };

    // track loads
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, loads);
    }

    // load global value directly
    let global_id = global_id(*global);
    let value = match state.interpreter.globals.get(global_id).copied() {
        Some(v) => v,
        None => return ControlFlow::Error(Error::UndefinedGlobal { global: global_id }),
    };

    // store value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle fused global address + store.
#[inline(always)]
pub(super) fn handle_global_store(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::GlobalStore {
        global,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // track stores
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, stores);
    }

    // load value to store
    let val = state.get(*value);

    // check mutability via reference metadata
    let global_id = global_id(*global);
    if reference.mutability() != Some(mir::Mutability::Mutable) {
        return ControlFlow::Error(Error::ImmutableGlobalWrite { global: global_id });
    }

    // store to global directly
    state.interpreter.globals.set(global_id, val);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle pointer load.
pub(super) fn handle_load(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from pointer
    let value = match instruction::load_from_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle pointer store.
pub(super) fn handle_store(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through pointer
    if let Err(e) = instruction::store_to_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed pointer load.
#[inline(always)]
pub(super) fn handle_load_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from managed pointer
    let value = match instruction::load_from_managed_reference(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw pointer load.
#[inline(always)]
pub(super) fn handle_load_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from raw pointer
    let value = match instruction::load_from_raw_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack pointer load.
#[inline(always)]
pub(super) fn handle_load_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from stack pointer
    let value = match instruction::load_from_stack_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global pointer load.
#[inline(always)]
pub(super) fn handle_load_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Load { dest, pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // load from global pointer
    let value = match instruction::load_from_global_pointer(state, ptr) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store loaded value
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed pointer store.
#[inline(always)]
pub(super) fn handle_store_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through managed pointer
    if let Err(e) = instruction::store_to_managed_reference(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw pointer store.
#[inline(always)]
pub(super) fn handle_store_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through raw pointer
    if let Err(e) = instruction::store_to_raw_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack pointer store.
#[inline(always)]
pub(super) fn handle_store_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through stack pointer
    if let Err(e) = instruction::store_to_stack_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle global pointer store.
#[inline(always)]
pub(super) fn handle_store_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Store {
        pointer,
        value,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let ptr = state.get(*pointer);
    let val = state.get(*value);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, ptr) {
        return ControlFlow::Error(error);
    }

    // write through global pointer
    if let Err(e) = instruction::store_to_global_pointer(state, ptr, val) {
        return ControlFlow::Error(e);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field get.
pub(super) fn handle_field_get(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldGet {
        dest,
        aggregate,
        index,
        field_count: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // load field value
    let value = match instruction::get_field(state, agg, *index) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field get for small inline aggregates (≤2 fields).
#[inline(always)]
pub(super) fn handle_field_get_inline(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldGet {
        dest,
        aggregate,
        index,
        field_count: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate and extract heap handle
    let agg = state.get(*aggregate);
    let handle = match agg.as_heap_handle() {
        Some(h) => h,
        None => return ControlFlow::Error(crate::diagnostic::Error::InvalidHeapHandle),
    };

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return ControlFlow::Error(crate::diagnostic::Error::NullPointerDereference);
    }

    // fast path: directly access heap cell and inline slots
    let value = unsafe {
        let cell = state.interpreter.managed_heap.get_unchecked(handle);
        let slot_index = handle.slot_index().wrapping_add(*index as usize);
        // inline storage is guaranteed for field_count ≤ 2
        *cell.slots.get_unchecked(slot_index)
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on small managed aggregates (≤2 fields, inline storage).
#[inline(always)]
pub(super) fn handle_field_store_inline(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference: _,
        field_count: _,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // track stores
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, stores);
    }

    // load aggregate and extract heap handle
    let agg = state.get(*aggregate);
    let handle = match agg.as_heap_handle() {
        Some(h) => h,
        None => return ControlFlow::Error(crate::diagnostic::Error::InvalidHeapHandle),
    };

    // reject null handles when enabled
    if state.null_checks && handle.is_null() {
        return ControlFlow::Error(crate::diagnostic::Error::NullPointerDereference);
    }

    // load value to store
    let val = state.get(*value);

    // fast path: directly access heap cell and inline slots
    unsafe {
        let cell = state.interpreter.managed_heap.get_unchecked_mut(handle);
        let slot_index = handle.slot_index().wrapping_add(*index as usize);
        // inline storage is guaranteed for field_count ≤ 2
        *cell.slots.get_unchecked_mut(slot_index) = val;
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr.
pub(super) fn handle_field_addr(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // compute field address
    let value = match instruction::field_addr(state, agg, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on aggregate values.
pub(super) fn handle_field_addr_aggregate(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::field_addr_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on managed references.
pub(super) fn handle_field_addr_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::field_addr_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on raw pointers.
pub(super) fn handle_field_addr_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = agg.as_raw_pointer().unwrap();
    let value = match instruction::field_addr_raw(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on stack pointers.
pub(super) fn handle_field_addr_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = agg.as_stack_pointer().unwrap();
    let value = match instruction::field_addr_stack(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field addr on global pointers.
pub(super) fn handle_field_addr_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldAddr {
        dest,
        aggregate,
        index,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // compute field address
    let pointer = agg.as_global_pointer().unwrap();
    let value = match instruction::field_addr_global(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load.
pub(super) fn handle_field_load(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);

    // compute field address
    let pointer = match instruction::field_addr(state, agg, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // load value
    let value = match instruction::load_from_pointer(state, pointer) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on aggregate values.
pub(super) fn handle_field_load_aggregate(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::load_field_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on managed references.
pub(super) fn handle_field_load_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let handle = agg.as_heap_handle().unwrap();
    let value = match instruction::load_field_managed(state, handle, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on raw pointers.
pub(super) fn handle_field_load_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = agg.as_raw_pointer().unwrap();
    let value = match instruction::load_field_raw(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on stack pointers.
pub(super) fn handle_field_load_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = agg.as_stack_pointer().unwrap();
    let value = match instruction::load_field_stack(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field load on global pointers.
pub(super) fn handle_field_load_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldLoad {
        dest,
        aggregate,
        index,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load aggregate
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }

    // load field value
    let pointer = agg.as_global_pointer().unwrap();
    let value = match instruction::load_field_global(state, pointer, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field set.
pub(super) fn handle_field_set(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldSet {
        dest,
        aggregate,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    let val = state.get(*value);

    // write field
    let result = match instruction::set_field(state, agg, *index, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store.
pub(super) fn handle_field_store(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    let val = state.get(*value);

    // compute field address
    let pointer = match instruction::field_addr(state, agg, *index, *field_count) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    let pointer = pointer.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_to_pointer(state, pointer, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on aggregate values.
pub(super) fn handle_field_store_aggregate(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "aggregate".to_string(),
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let handle = agg.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_field_managed(state, handle, *index, *field_count, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on managed references.
pub(super) fn handle_field_store_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let handle = agg.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_field_managed(state, handle, *index, *field_count, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on raw pointers.
pub(super) fn handle_field_store_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference kind
    let raw_pointer = agg.as_raw_pointer().unwrap();
    let pointer = Value::raw_pointer_with_meta(raw_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_field_raw(state, raw_pointer, *index, *field_count, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on stack pointers.
pub(super) fn handle_field_store_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let stack_pointer = agg.as_stack_pointer().unwrap();
    let pointer = Value::stack_pointer_with_meta(stack_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_field_stack(state, stack_pointer, *index, *field_count, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle field store on global pointers.
pub(super) fn handle_field_store_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::FieldStore {
        aggregate,
        index,
        value,
        reference,
        field_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let agg = state.get(*aggregate);
    if agg.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{agg:?}"),
        });
    }
    let val = state.get(*value);

    // validate reference semantics
    let global_pointer = agg.as_global_pointer().unwrap();
    let pointer =
        Value::global_pointer_with_meta(global_pointer.id, global_pointer.slot_offset, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_field_global(state, global_pointer, *index, *field_count, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element get.
pub(super) fn handle_element_get(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementGet { dest, array, index } = &block[pc].data else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let value = match instruction::get_element(state, arr, idx_val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr.
pub(super) fn handle_element_addr(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let value = match instruction::element_addr(state, arr, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on aggregate values.
pub(super) fn handle_element_addr_aggregate(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::element_addr_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on managed references.
pub(super) fn handle_element_addr_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::element_addr_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on raw pointers.
pub(super) fn handle_element_addr_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = arr.as_raw_pointer().unwrap();
    let value = match instruction::element_addr_raw(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on stack pointers.
pub(super) fn handle_element_addr_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = arr.as_stack_pointer().unwrap();
    let value = match instruction::element_addr_stack(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element addr on global pointers.
pub(super) fn handle_element_addr_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementAddr {
        dest,
        array,
        index,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = arr.as_global_pointer().unwrap();
    let value = match instruction::element_addr_global(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // apply reference metadata
    let value = value.with_reference_meta(*reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load.
pub(super) fn handle_element_load(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = match instruction::element_addr(state, arr, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // load value
    let value = match instruction::load_from_pointer(state, pointer) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on aggregate values.
pub(super) fn handle_element_load_aggregate(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::load_element_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on managed references.
pub(super) fn handle_element_load_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let handle = arr.as_heap_handle().unwrap();
    let value = match instruction::load_element_managed(state, handle, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on raw pointers.
pub(super) fn handle_element_load_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let pointer = arr.as_raw_pointer().unwrap();
    let value = match instruction::load_element_raw(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on stack pointers.
pub(super) fn handle_element_load_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let pointer = arr.as_stack_pointer().unwrap();
    let value = match instruction::load_element_stack(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element load on global pointers.
pub(super) fn handle_element_load_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementLoad {
        dest,
        array,
        index,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load array and index
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let idx_val = idx.as_uint().unwrap_or(0);

    // load element value
    let pointer = arr.as_global_pointer().unwrap();
    let value = match instruction::load_element_global(state, pointer, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    // store result
    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element set.
pub(super) fn handle_element_set(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementSet {
        dest,
        array,
        index,
        value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // write element
    let result = match instruction::set_element(state, arr, idx_val, val) {
        Ok(v) => v,
        Err(e) => return ControlFlow::Error(e),
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store.
pub(super) fn handle_element_store(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // compute element address
    let pointer = match instruction::element_addr(state, arr, idx_val, *array_length) {
        Ok(value) => value,
        Err(error) => return ControlFlow::Error(error),
    };

    let pointer = pointer.with_reference_meta(*reference);

    // validate reference semantics
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) = instruction::store_to_pointer(state, pointer, val) {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on aggregate values.
pub(super) fn handle_element_store_aggregate(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::Aggregate {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "array".to_string(),
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let handle = arr.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_managed(state, handle, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on managed references.
pub(super) fn handle_element_store_managed(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::ManagedReference {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let handle = arr.as_heap_handle().unwrap();
    let pointer = Value::managed_reference_with_meta(handle, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_managed(state, handle, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on raw pointers.
pub(super) fn handle_element_store_raw(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::RawPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let raw_pointer = arr.as_raw_pointer().unwrap();
    let pointer = Value::raw_pointer_with_meta(raw_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_raw(state, raw_pointer, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on stack pointers.
pub(super) fn handle_element_store_stack(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::StackPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let stack_pointer = arr.as_stack_pointer().unwrap();
    let pointer = Value::stack_pointer_with_meta(stack_pointer, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_stack(state, stack_pointer, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle element store on global pointers.
pub(super) fn handle_element_store_global(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ElementStore {
        array,
        index,
        value,
        reference,
        array_length,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let arr = state.get(*array);
    if arr.tag() != ValueTag::GlobalPointer {
        return ControlFlow::Error(Error::InvalidPointerType {
            actual: format!("{arr:?}"),
        });
    }
    let idx = state.get(*index);
    let val = state.get(*value);
    let idx_val = idx.as_uint().unwrap_or(0);

    // validate reference semantics
    let global_pointer = arr.as_global_pointer().unwrap();
    let pointer =
        Value::global_pointer_with_meta(global_pointer.id, global_pointer.slot_offset, *reference);
    if let Err(error) = check_reference_kind(state, *reference, pointer) {
        return ControlFlow::Error(error);
    }
    if let Err(error) = check_reference_mutability(state, *reference) {
        return ControlFlow::Error(error);
    }

    // store value
    if let Err(error) =
        instruction::store_element_global(state, global_pointer, idx_val, *array_length, val)
    {
        return ControlFlow::Error(error);
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed allocation.
pub(super) fn handle_managed_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ManagedAlloc {
        dest,
        reference,
        slot_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // enforce heap limit
    if state.interpreter.managed_heap.cell_count() >= state.interpreter.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    // allocate heap cell
    let handle = if *slot_count == UNKNOWN_SLOT_COUNT {
        state.interpreter.managed_heap.allocate()
    } else {
        state
            .interpreter
            .managed_heap
            .allocate_with_slots(*slot_count as usize)
    };
    if state.collect_stats {
        state.interpreter.statistics.heap_allocations += 1;
    }
    let value = Value::managed_reference_with_meta(handle, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle managed array allocation.
pub(super) fn handle_managed_alloc_array(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::ManagedAllocArray {
        dest,
        length,
        reference,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // enforce heap limit
    if state.interpreter.managed_heap.cell_count() >= state.interpreter.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    // resolve array length
    let len_val = state.get(*length);
    let length = len_val.as_uint().unwrap_or(0) as usize;

    // allocate heap cell with slots
    let handle = state.interpreter.managed_heap.allocate_with_slots(length);
    if state.collect_stats {
        state.interpreter.statistics.heap_allocations += 1;
    }
    let value = Value::managed_reference_with_meta(handle, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw allocation.
pub(super) fn handle_raw_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::RawAlloc {
        dest,
        reference,
        slot_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // enforce heap limit
    if state.interpreter.raw_heap.cell_count() >= state.interpreter.options.max_heap_cells {
        return ControlFlow::Error(Error::AllocationFailed);
    }

    // allocate raw heap cell
    let ptr = if *slot_count == UNKNOWN_SLOT_COUNT {
        state.interpreter.raw_heap.allocate()
    } else {
        state
            .interpreter
            .raw_heap
            .allocate_with_slots(*slot_count as usize)
    };
    if state.collect_stats {
        state.interpreter.statistics.heap_allocations += 1;
    }
    let value = Value::raw_pointer_with_meta(ptr, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw free.
pub(super) fn handle_raw_free(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::RawFree { pointer } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*pointer);

    // accept raw pointer values
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid handle
        if !state.interpreter.raw_heap.free(p) {
            return ControlFlow::Error(Error::InvalidHeapHandle);
        }
    }
    // otherwise report type mismatch
    else {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle raw drop (compiler-inserted deallocation at ownership end).
/// Semantically equivalent to raw_free but signals ownership transfer.
pub(super) fn handle_raw_drop(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::RawDrop { value } = &block[pc].data else {
        unreachable!()
    };

    // load pointer value
    let ptr = state.get(*value);

    // accept raw pointer values - deallocate like raw_free
    if let Some(p) = ptr.as_raw_pointer() {
        // report invalid handle
        if !state.interpreter.raw_heap.free(p) {
            return ControlFlow::Error(Error::InvalidHeapHandle);
        }
    }
    // otherwise report type mismatch
    else {
        return ControlFlow::Error(Error::TypeMismatch {
            expected: "raw_pointer".to_string(),
            actual: format!("{ptr:?}"),
        });
    }

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack allocation.
pub(super) fn handle_stack_alloc(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::StackAlloc {
        dest,
        reference,
        slot_count,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // NOTE #Broken: stack allocation requires proper layout sizing
    let frame_index = state.frame_index;
    let slot = if *slot_count == UNKNOWN_SLOT_COUNT {
        state.current_frame_mut().allocate_stack_cell()
    } else {
        state
            .current_frame_mut()
            .allocate_stack_cell_with_slots(*slot_count as usize)
    };
    let sp = crate::memory::StackPointer::new(frame_index, slot);
    let value = Value::stack_pointer_with_meta(sp, *reference);

    // validate reference kind
    if let Err(error) = check_reference_kind(state, *reference, value) {
        return ControlFlow::Error(error);
    }

    state.set(*dest, value);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle stack drop (compiler-inserted lifetime end marker).
/// Currently a no-op - stack memory is freed when the frame exits.
/// Exists for NLL support and potential future optimizations.
pub(super) fn handle_stack_drop(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data - validate it's the right instruction
    let ThreadedInstructionData::StackDrop { value: _ } = &block[pc].data else {
        unreachable!()
    };

    // no-op: stack memory is managed by frame lifetime
    // the instruction exists to mark the end of the value's lifetime for NLL

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle assume (optimizer hint).
pub(super) fn handle_assume(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Assume { condition: _ } = &block[pc].data else {
        unreachable!()
    };

    // no op: assume is handled by the optimizer

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle intrinsic call.
pub(super) fn handle_intrinsic(
    state: &mut ThreadedState<'_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Intrinsic {
        dest,
        intrinsic,
        arguments,
        ordering,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve arguments
    let args = collect_values(state, *arguments);

    // execute intrinsic
    match state
        .interpreter
        .execute_intrinsic_resolved(*intrinsic, args.as_slice(), *ordering)
    {
        // store result and continue
        Ok(result) => {
            if !is_invalid_value(*dest) {
                state.set(*dest, result);
            }
            next!(state, block, pc)
        }
        // return runtime error
        Err(e) => ControlFlow::Error(e.error),
    }
}

/// Handle return (exits tail-call chain).
pub(super) fn handle_return(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Return { value } = &block[pc].data else {
        unreachable!()
    };

    // resolve return value
    let return_value = if is_invalid_value(*value) {
        Value::VOID
    } else {
        state.get(*value)
    };

    // return to caller
    ControlFlow::Return(return_value)
}

/// Handle unconditional jump (exits tail-call chain).
pub(super) fn handle_jump(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Jump { target, copies } = &block[pc].data else {
        unreachable!()
    };

    // return jump control
    ControlFlow::Jump {
        block: *target,
        copies: *copies,
    }
}

/// Handle conditional branch (exits tail-call chain).
pub(super) fn handle_branch(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Branch {
        condition,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // evaluate branch condition
    let cond = state.get(*condition);
    let is_truthy = cond.is_truthy();

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // handle truthy branch
    if is_truthy {
        // forward then copies
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    }
    // otherwise jump to else target
    else {
        // forward else copies
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle boolean branch (exits tail-call chain).
pub(super) fn handle_branch_bool(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Branch {
        condition,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // evaluate branch condition
    let cond = state.get(*condition);
    let is_truthy = cond.raw_data() != 0;

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // handle truthy branch
    if is_truthy {
        // forward then copies
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    }
    // otherwise jump to else target
    else {
        // forward else copies
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch for signed integers (most common).
#[inline(always)]
pub(super) fn handle_compare_and_branch_int(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands as signed integers
    let lhs = state.get(*left).raw_data() as i64;
    let rhs = state.get(*right).raw_data() as i64;

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::Equal => lhs == rhs,
        mir::BinaryOperator::NotEqual => lhs != rhs,
        mir::BinaryOperator::SignedLessThan => lhs < rhs,
        mir::BinaryOperator::SignedLessEqual => lhs <= rhs,
        mir::BinaryOperator::SignedGreaterThan => lhs > rhs,
        mir::BinaryOperator::SignedGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch for unsigned integers.
#[inline(always)]
pub(super) fn handle_compare_and_branch_uint(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands as unsigned integers
    let lhs = state.get(*left).raw_data();
    let rhs = state.get(*right).raw_data();

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::UnsignedLessThan => lhs < rhs,
        mir::BinaryOperator::UnsignedLessEqual => lhs <= rhs,
        mir::BinaryOperator::UnsignedGreaterThan => lhs > rhs,
        mir::BinaryOperator::UnsignedGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch for floats.
#[inline(always)]
pub(super) fn handle_compare_and_branch_float(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands as floats
    let lhs = state.get(*left).as_float64();
    let rhs = state.get(*right).as_float64();

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::FloatEqual => lhs == rhs,
        mir::BinaryOperator::FloatNotEqual => lhs != rhs,
        mir::BinaryOperator::FloatLessThan => lhs < rhs,
        mir::BinaryOperator::FloatLessEqual => lhs <= rhs,
        mir::BinaryOperator::FloatGreaterThan => lhs > rhs,
        mir::BinaryOperator::FloatGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch (generic fallback).
pub(super) fn handle_compare_and_branch(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // perform comparison inline
    let is_truthy = match operator {
        mir::BinaryOperator::Equal => lhs.raw_data() == rhs.raw_data(),
        mir::BinaryOperator::NotEqual => lhs.raw_data() != rhs.raw_data(),
        mir::BinaryOperator::SignedLessThan => (lhs.raw_data() as i64) < (rhs.raw_data() as i64),
        mir::BinaryOperator::SignedLessEqual => (lhs.raw_data() as i64) <= (rhs.raw_data() as i64),
        mir::BinaryOperator::SignedGreaterThan => (lhs.raw_data() as i64) > (rhs.raw_data() as i64),
        mir::BinaryOperator::SignedGreaterEqual => {
            (lhs.raw_data() as i64) >= (rhs.raw_data() as i64)
        }
        mir::BinaryOperator::UnsignedLessThan => lhs.raw_data() < rhs.raw_data(),
        mir::BinaryOperator::UnsignedLessEqual => lhs.raw_data() <= rhs.raw_data(),
        mir::BinaryOperator::UnsignedGreaterThan => lhs.raw_data() > rhs.raw_data(),
        mir::BinaryOperator::UnsignedGreaterEqual => lhs.raw_data() >= rhs.raw_data(),
        mir::BinaryOperator::FloatEqual => lhs.as_float64() == rhs.as_float64(),
        mir::BinaryOperator::FloatNotEqual => lhs.as_float64() != rhs.as_float64(),
        mir::BinaryOperator::FloatLessThan => lhs.as_float64() < rhs.as_float64(),
        mir::BinaryOperator::FloatLessEqual => lhs.as_float64() <= rhs.as_float64(),
        mir::BinaryOperator::FloatGreaterThan => lhs.as_float64() > rhs.as_float64(),
        mir::BinaryOperator::FloatGreaterEqual => lhs.as_float64() >= rhs.as_float64(),
        // non-comparison operators should not reach here
        _ => unreachable!("compare-and-branch with non-comparison operator"),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch with constant right operand for signed integers.
#[inline(always)]
pub(super) fn handle_compare_and_branch_const_int(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands as signed integers
    let lhs = state.get(*left).raw_data() as i64;
    let rhs = right_const.raw_data() as i64;

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::Equal => lhs == rhs,
        mir::BinaryOperator::NotEqual => lhs != rhs,
        mir::BinaryOperator::SignedLessThan => lhs < rhs,
        mir::BinaryOperator::SignedLessEqual => lhs <= rhs,
        mir::BinaryOperator::SignedGreaterThan => lhs > rhs,
        mir::BinaryOperator::SignedGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch with constant right operand for unsigned integers.
#[inline(always)]
pub(super) fn handle_compare_and_branch_const_uint(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands as unsigned integers
    let lhs = state.get(*left).raw_data();
    let rhs = right_const.raw_data();

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::UnsignedLessThan => lhs < rhs,
        mir::BinaryOperator::UnsignedLessEqual => lhs <= rhs,
        mir::BinaryOperator::UnsignedGreaterThan => lhs > rhs,
        mir::BinaryOperator::UnsignedGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch with constant right operand for floats.
#[inline(always)]
pub(super) fn handle_compare_and_branch_const_float(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands as floats
    let lhs = state.get(*left).as_float64();
    let rhs = right_const.as_float64();

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::FloatEqual => lhs == rhs,
        mir::BinaryOperator::FloatNotEqual => lhs != rhs,
        mir::BinaryOperator::FloatLessThan => lhs < rhs,
        mir::BinaryOperator::FloatLessEqual => lhs <= rhs,
        mir::BinaryOperator::FloatGreaterThan => lhs > rhs,
        mir::BinaryOperator::FloatGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle fused compare-and-branch with constant right operand (generic fallback).
pub(super) fn handle_compare_and_branch_const(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load operands
    let lhs = state.get(*left);
    let rhs = *right_const;

    // perform comparison inline
    let is_truthy = match operator {
        mir::BinaryOperator::Equal => lhs.raw_data() == rhs.raw_data(),
        mir::BinaryOperator::NotEqual => lhs.raw_data() != rhs.raw_data(),
        mir::BinaryOperator::SignedLessThan => (lhs.raw_data() as i64) < (rhs.raw_data() as i64),
        mir::BinaryOperator::SignedLessEqual => (lhs.raw_data() as i64) <= (rhs.raw_data() as i64),
        mir::BinaryOperator::SignedGreaterThan => (lhs.raw_data() as i64) > (rhs.raw_data() as i64),
        mir::BinaryOperator::SignedGreaterEqual => {
            (lhs.raw_data() as i64) >= (rhs.raw_data() as i64)
        }
        mir::BinaryOperator::UnsignedLessThan => lhs.raw_data() < rhs.raw_data(),
        mir::BinaryOperator::UnsignedLessEqual => lhs.raw_data() <= rhs.raw_data(),
        mir::BinaryOperator::UnsignedGreaterThan => lhs.raw_data() > rhs.raw_data(),
        mir::BinaryOperator::UnsignedGreaterEqual => lhs.raw_data() >= rhs.raw_data(),
        mir::BinaryOperator::FloatEqual => lhs.as_float64() == rhs.as_float64(),
        mir::BinaryOperator::FloatNotEqual => lhs.as_float64() != rhs.as_float64(),
        mir::BinaryOperator::FloatLessThan => lhs.as_float64() < rhs.as_float64(),
        mir::BinaryOperator::FloatLessEqual => lhs.as_float64() <= rhs.as_float64(),
        mir::BinaryOperator::FloatGreaterThan => lhs.as_float64() > rhs.as_float64(),
        mir::BinaryOperator::FloatGreaterEqual => lhs.as_float64() >= rhs.as_float64(),
        _ => unreachable!("compare-and-branch with non-comparison operator"),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        ControlFlow::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        ControlFlow::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Handle switch (exits tail-call chain).
pub(super) fn handle_switch(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Switch {
        value,
        cases,
        default_target,
        default_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.as_int().unwrap_or(0);

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // find matching case
    let case_slice = state.switch_cases(*cases);
    for case in case_slice {
        if case.value == int_val {
            // forward case copies
            return ControlFlow::Jump {
                block: case.target,
                copies: case.copies,
            };
        }
    }

    // forward default copies
    // return default jump
    ControlFlow::Jump {
        block: *default_target,
        copies: *default_copies,
    }
}

/// Handle switch via dense jump table (exits tail-call chain).
pub(super) fn handle_switch_table(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::SwitchTable {
        value,
        min,
        table,
        default_target,
        default_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.as_int().unwrap_or(0);

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // resolve jump table entry
    if int_val < *min {
        return ControlFlow::Jump {
            block: *default_target,
            copies: *default_copies,
        };
    }
    let offset = (int_val - *min) as usize;
    let case_slice = state.switch_cases(*table);
    let Some(case) = case_slice.get(offset) else {
        return ControlFlow::Jump {
            block: *default_target,
            copies: *default_copies,
        };
    };

    // jump to resolved case
    ControlFlow::Jump {
        block: case.target,
        copies: case.copies,
    }
}

/// Handle integer switch (exits tail-call chain).
pub(super) fn handle_switch_int(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Switch {
        value,
        cases,
        default_target,
        default_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.raw_data() as i64;

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // find matching case
    let case_slice = state.switch_cases(*cases);
    for case in case_slice {
        if case.value == int_val {
            // forward case copies
            return ControlFlow::Jump {
                block: case.target,
                copies: case.copies,
            };
        }
    }

    // forward default copies
    // return default jump
    ControlFlow::Jump {
        block: *default_target,
        copies: *default_copies,
    }
}

/// Handle integer switch via dense jump table (exits tail-call chain).
pub(super) fn handle_switch_table_int(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::SwitchTable {
        value,
        min,
        table,
        default_target,
        default_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.raw_data() as i64;

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.interpreter.statistics, branches);
    }

    // resolve jump table entry
    if int_val < *min {
        return ControlFlow::Jump {
            block: *default_target,
            copies: *default_copies,
        };
    }
    let offset = (int_val - *min) as usize;
    let case_slice = state.switch_cases(*table);
    let Some(case) = case_slice.get(offset) else {
        return ControlFlow::Jump {
            block: *default_target,
            copies: *default_copies,
        };
    };

    // jump to resolved case
    ControlFlow::Jump {
        block: case.target,
        copies: case.copies,
    }
}

/// Handle aggregate construction.
pub(super) fn handle_aggregate(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    // decode instruction data
    let ThreadedInstructionData::Aggregate { dest, elements } = &block[pc].data else {
        unreachable!()
    };

    // resolve element values from the argument pool
    let element_slice = state.argument_slice(*elements);
    let result = match element_slice {
        // empty aggregate
        [] => state.interpreter.allocate_aggregate(Vec::new()),
        // single element aggregate
        [first] => state.interpreter.allocate_single(state.get(*first)),
        // pair aggregate fast path
        [first, second] => state
            .interpreter
            .allocate_pair(state.get(*first), state.get(*second)),
        // general aggregate
        _ => {
            // collect element values into a vec
            let mut element_values = Vec::with_capacity(element_slice.len());
            for value in element_slice {
                element_values.push(state.get(*value));
            }

            // allocate the aggregate on the heap
            state.interpreter.allocate_aggregate(element_values)
        }
    };

    // store result
    state.set(*dest, result);

    // continue to next instruction
    next!(state, block, pc)
}

/// Handle unreachable (errors).
pub(super) fn handle_unreachable(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // return unreachable error
    ControlFlow::Error(Error::Unreachable)
}

/// Handle unsupported instructions (errors).
pub(super) fn handle_unsupported(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Unsupported { name } = &block[pc].data else {
        unreachable!()
    };

    // return unsupported error
    ControlFlow::Error(Error::UnsupportedInstruction {
        name: (*name).to_string(),
    })
}

/// Enter a tail call by reusing the current frame.
fn enter_tail_call(
    state: &mut ThreadedState,
    function_id: mir::LocalNodeId<mir::Function>,
    callee: &ThreadedFunction,
    argument_values: &[Value],
) {
    // resolve frame bounds
    let (value_base, local_base) = {
        let frame = state.current_frame_mut();
        (frame.value_base, frame.local_base)
    };

    // clear frame local stack allocations
    state.current_frame_mut().stack_cells.clear();

    // resize stacks to callee requirements
    let value_end = value_base + callee.value_count;
    let local_end = local_base + callee.local_count;
    resize_and_clear_stack(&mut state.interpreter.value_stack, value_base, value_end);
    resize_and_clear_stack(&mut state.interpreter.local_stack, local_base, local_end);

    // update frame metadata
    let entry_block = &callee.blocks[callee.entry as usize];
    {
        let frame = state.current_frame_mut();
        frame.function = function_id;
        frame.threaded = NonNull::from(callee);
        frame.block_ptr = NonNull::from(entry_block);
        frame.entry_block = entry_block.mir_block;
        frame.current_block = entry_block.mir_block;
        frame.block_index = callee.entry as usize;
        frame.resume_pc = 0;
        frame.value_count = callee.value_count;
        frame.local_count = callee.local_count;
    }

    // refresh cached pointers for the new threaded function
    state.refresh_for_threaded(callee);

    // bind function parameters
    let parameter_slice = callee.parameters.slice(callee.argument_pool.as_slice());
    // use direct indexing when arguments cover parameters
    if argument_values.len() >= parameter_slice.len() {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = argument_values[index];
            state.set(*param, value);
        }
    }
    // fall back to defaulted arguments
    else {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = argument_values.get(index).copied().unwrap_or(Value::VOID);
            state.set(*param, value);
        }
    }

    // update statistics
    if state.collect_stats {
        state.interpreter.statistics.calls_made += 1;
    }

    // keep frame ready for entry execution
}

/// Handle tail call to function.
pub(super) fn handle_tail_call(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCall {
        function,
        callee_index,
        copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve callee index
    let function_id = mir::LocalNodeId::<mir::Function>::new(*function);
    let resolved_index = if *callee_index == INVALID_FUNCTION_INDEX {
        state.interpreter.threaded_functions.index_for(function_id)
    } else {
        Some(*callee_index)
    };

    // fall back to trampoline for non-threaded targets
    let Some(resolved_index) = resolved_index else {
        return ControlFlow::TailCall {
            function: *function,
            callee_index: *callee_index,
            arguments: ArgumentRange::empty(),
            copies: Some(*copies),
        };
    };
    let Some(callee_ptr) = state
        .interpreter
        .threaded_functions
        .get_ptr_by_index(resolved_index)
    else {
        return ControlFlow::TailCall {
            function: *function,
            callee_index: *callee_index,
            arguments: ArgumentRange::empty(),
            copies: Some(*copies),
        };
    };
    let callee = unsafe { callee_ptr.as_ref() };

    // collect argument values
    let argument_values = {
        let threaded_ptr = state.current_frame_mut().threaded;
        let current_func = unsafe { threaded_ptr.as_ref() };
        let copy_pairs = copies.slice(current_func.copy_pool.as_slice());
        let mut args: SmallVec<[Value; 16]> = SmallVec::with_capacity(copy_pairs.len());

        for pair in copy_pairs {
            let value = if pair.src == INVALID_VALUE_ID {
                Value::VOID
            } else {
                let arg_value = mir::Value::new(pair.src);
                state.get(arg_value)
            };
            args.push(value);
        }

        args
    };

    // enter tail call fast path
    enter_tail_call(state, function_id, callee, &argument_values);

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    become (entry_instructions[0].handler)(state, entry_instructions, 0)
}

/// Handle self tail call by reusing the current frame.
pub(super) fn handle_tail_call_self(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCallSelf { entry, arguments } = &block[pc].data else {
        unreachable!()
    };

    // collect argument values
    let args = collect_values(state, *arguments);

    if state.collect_stats {
        state.interpreter.statistics.calls_made += 1;
    }

    // resolve current function entry block
    let (threaded_ptr, value_base, value_count, local_base, local_count) = {
        let frame = state.current_frame_mut();
        (
            frame.threaded,
            frame.value_base,
            frame.value_count,
            frame.local_base,
            frame.local_count,
        )
    };
    let threaded = unsafe { threaded_ptr.as_ref() };
    let entry_block = &threaded.blocks[*entry as usize];

    // clear frame-local stack allocations
    state.current_frame_mut().stack_cells.clear();

    // clear value and local slots
    let value_end = value_base + value_count;
    state.interpreter.value_stack[value_base..value_end].fill(Value::VOID);
    let local_end = local_base + local_count;
    state.interpreter.local_stack[local_base..local_end].fill(Value::VOID);

    // update frame to entry block
    {
        let frame = state.current_frame_mut();
        frame.block_index = *entry as usize;
        frame.block_ptr = NonNull::from(entry_block);
        frame.entry_block = entry_block.mir_block;
        frame.current_block = entry_block.mir_block;
        frame.resume_pc = 0;
    }

    // bind function parameters
    let parameter_slice = threaded.parameters.slice(threaded.argument_pool.as_slice());
    // use direct indexing when arguments cover parameters
    if args.len() >= parameter_slice.len() {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = args[index];
            state.set(*param, value);
        }
    }
    // fall back to defaulted arguments
    else {
        for (index, param) in parameter_slice.iter().enumerate() {
            let value = args.get(index).copied().unwrap_or(Value::VOID);
            state.set(*param, value);
        }
    }

    // continue at entry block
    let entry_instructions = entry_block.instructions.as_slice();
    become (entry_instructions[0].handler)(state, entry_instructions, 0)
}

/// Handle indirect tail call.
pub(super) fn handle_tail_call_indirect(
    state: &mut ThreadedState,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::TailCallIndirect {
        callee,
        arguments,
        cached_function,
        cached_ptr,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load callee value
    let callee_val = state.get(*callee);

    // extract function pointer
    let function = match callee_val.as_function_pointer() {
        Some(f) => f.id,
        None => {
            return ControlFlow::Error(Error::TypeMismatch {
                expected: "function_pointer".to_string(),
                actual: format!("{callee_val:?}"),
            });
        }
    };

    // resolve callee id
    let function_id = mir::LocalNodeId::<mir::Function>::new(function);

    // reuse cached callee pointer when possible
    if cached_function.get() == Some(function) {
        if let Some(callee_ptr) = cached_ptr.get() {
            let callee = unsafe { callee_ptr.as_ref() };
            let argument_values = collect_values(state, *arguments);
            enter_tail_call(state, function_id, callee, &argument_values);
            let entry_block_ptr = state.current_frame_mut().block_ptr;
            let entry_block = unsafe { entry_block_ptr.as_ref() };
            let entry_instructions = entry_block.instructions.as_slice();
            become (entry_instructions[0].handler)(state, entry_instructions, 0)
        }

        return ControlFlow::TailCall {
            function,
            callee_index: INVALID_FUNCTION_INDEX,
            arguments: *arguments,
            copies: None,
        };
    }

    // resolve callee index
    let resolved_index = state.interpreter.threaded_functions.index_for(function_id);

    // fall back to trampoline for non-threaded targets
    let Some(resolved_index) = resolved_index else {
        cached_function.set(Some(function));
        cached_ptr.set(None);
        return ControlFlow::TailCall {
            function,
            callee_index: INVALID_FUNCTION_INDEX,
            arguments: *arguments,
            copies: None,
        };
    };
    let Some(callee_ptr) = state
        .interpreter
        .threaded_functions
        .get_ptr_by_index(resolved_index)
    else {
        cached_function.set(Some(function));
        cached_ptr.set(None);
        return ControlFlow::TailCall {
            function,
            callee_index: INVALID_FUNCTION_INDEX,
            arguments: *arguments,
            copies: None,
        };
    };
    cached_function.set(Some(function));
    cached_ptr.set(Some(callee_ptr));
    let callee = unsafe { callee_ptr.as_ref() };

    // collect argument values
    let argument_values = collect_values(state, *arguments);

    // enter tail call fast path
    enter_tail_call(state, function_id, callee, &argument_values);

    // continue at entry block
    let entry_block_ptr = state.current_frame_mut().block_ptr;
    let entry_block = unsafe { entry_block_ptr.as_ref() };
    let entry_instructions = entry_block.instructions.as_slice();
    become (entry_instructions[0].handler)(state, entry_instructions, 0)
}
