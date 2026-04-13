use super::prelude::*;
use crate::executable::Transfer;
use crate::telemetry::stat_inc;

/// Record one control-flow branch in the VM statistics.
#[inline(always)]
fn record_branch(state: &mut StepState<'_, '_>) {
    if state.collect_stats {
        stat_inc!(state.engine.statistics, branches);
    }
}

/// Return one branch jump based on the evaluated condition.
#[inline(always)]
fn branch_transfer(
    is_truthy: bool,
    then_target: u32,
    then_copies: CopyRange,
    else_target: u32,
    else_copies: CopyRange,
) -> Transfer {
    if is_truthy {
        return Transfer::Jump {
            block: then_target,
            copies: then_copies,
        };
    }

    Transfer::Jump {
        block: else_target,
        copies: else_copies,
    }
}

/// Return one default switch jump.
#[inline(always)]
fn default_switch_transfer(default_target: u32, default_copies: CopyRange) -> Transfer {
    Transfer::Jump {
        block: default_target,
        copies: default_copies,
    }
}

/// Load one switch operand as a signed integer.
#[inline(always)]
fn load_switch_value(state: &StepState<'_, '_>, value: mir::Value) -> Result<i64, Error> {
    let value = state.get(value);

    value.as_int().ok_or_else(|| Error::TypeMismatch {
        expected: "signed integer".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Load one value as a signed integer.
#[inline(always)]
fn load_signed_value(state: &StepState<'_, '_>, value: mir::Value) -> Result<(i64, u8), Error> {
    let value = state.get(value);

    value
        .as_int_with_width()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "signed integer".to_string(),
            actual: format!("{value:?}"),
        })
}

/// Load one value as an unsigned integer.
#[inline(always)]
fn load_unsigned_value(state: &StepState<'_, '_>, value: mir::Value) -> Result<(u64, u8), Error> {
    let value = state.get(value);

    value
        .as_uint_with_width()
        .ok_or_else(|| Error::TypeMismatch {
            expected: "unsigned integer".to_string(),
            actual: format!("{value:?}"),
        })
}

/// Load one value as a non-negative length.
#[inline(always)]
fn load_length_value(state: &StepState<'_, '_>, value: mir::Value) -> Result<u64, Error> {
    let value = state.get(value);

    if let Some((length, _)) = value.as_uint_with_width() {
        return Ok(length);
    }

    if let Some((length, _)) = value.as_int_with_width()
        && length >= 0
    {
        return Ok(length as u64);
    }

    Err(Error::TypeMismatch {
        expected: "non negative integer".to_string(),
        actual: format!("{value:?}"),
    })
}

/// Return the runtime referenced type when it can be recovered.
fn actual_reference_type(
    state: &StepState<'_, '_>,
    value_id: mir::Value,
) -> Result<Option<mir::LocalNodeId<mir::Type>>, Error> {
    let value = state.get(value_id);

    if let Some(handle) = value.as_managed_reference() {
        if handle.is_null() {
            return Ok(None);
        }

        if let Some(type_id) = state.heap().managed_type_id(handle) {
            return Ok(Some(mir::LocalNodeId::new(type_id)));
        }
    }

    let static_type = state.value_type(value_id)?;
    let pointee = match state.tree().get(static_type) {
        mir::Type::Reference { pointee, .. } => {
            Some((*pointee).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "reference pointee".to_string(),
            })?)
        }
        mir::Type::TensorReference { element, .. } => {
            Some((*element).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "tensor reference element".to_string(),
            })?)
        }
        _ => None,
    };

    Ok(pointee)
}

/// Evaluate one overflow guard.
fn evaluate_overflow_check(
    state: &StepState<'_, '_>,
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    is_signed: bool,
) -> Result<bool, Error> {
    if is_signed {
        let (left, width) = load_signed_value(state, left)?;
        let (right, right_width) = load_signed_value(state, right)?;
        if width != right_width {
            return Err(Error::InvalidInstruction);
        }

        let min_value = -(1_i128 << (u32::from(width).saturating_sub(1)));
        let max_value = (1_i128 << (u32::from(width).saturating_sub(1))) - 1;
        let left = left as i128;
        let right = right as i128;

        let overflows = match operator {
            mir::BinaryOperator::Add => {
                let result = left + right;
                result < min_value || result > max_value
            }
            mir::BinaryOperator::Subtract => {
                let result = left - right;
                result < min_value || result > max_value
            }
            mir::BinaryOperator::Multiply => {
                let result = left * right;
                result < min_value || result > max_value
            }
            mir::BinaryOperator::SignedDivide | mir::BinaryOperator::SignedRemainder => {
                if right == 0 {
                    return Err(Error::DivisionByZero);
                }

                left == min_value && right == -1
            }
            _ => return Err(Error::InvalidInstruction),
        };

        return Ok(overflows);
    }

    let (left, width) = load_unsigned_value(state, left)?;
    let (right, right_width) = load_unsigned_value(state, right)?;
    if width != right_width {
        return Err(Error::InvalidInstruction);
    }

    let max_value = if width >= 64 {
        u128::from(u64::MAX)
    } else {
        (1_u128 << u32::from(width)) - 1
    };
    let left = u128::from(left);
    let right = u128::from(right);

    let overflows = match operator {
        mir::BinaryOperator::Add => left + right > max_value,
        mir::BinaryOperator::Subtract => left < right,
        mir::BinaryOperator::Multiply => left.saturating_mul(right) > max_value,
        mir::BinaryOperator::UnsignedDivide | mir::BinaryOperator::UnsignedRemainder => {
            if right == 0 {
                return Err(Error::DivisionByZero);
            }

            false
        }
        _ => return Err(Error::InvalidInstruction),
    };

    Ok(overflows)
}

/// Evaluate one semantic check guard.
fn evaluate_check_constraint(
    state: &StepState<'_, '_>,
    constraint: &mir::CheckConstraint,
) -> Result<bool, Error> {
    match constraint {
        mir::CheckConstraint::Bounds {
            index,
            length,
            is_signed,
            ..
        } => {
            let length = load_length_value(
                state,
                (*length)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "bounds check length".to_string(),
                    })?,
            )?;

            if *is_signed {
                let (index, _) = load_signed_value(
                    state,
                    (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "bounds check index".to_string(),
                    })?,
                )?;
                Ok(index >= 0 && (index as u64) < length)
            } else {
                let (index, _) = load_unsigned_value(
                    state,
                    (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "bounds check index".to_string(),
                    })?,
                )?;
                Ok(index < length)
            }
        }
        mir::CheckConstraint::Null { value } => {
            let value = state.get((*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "null check value".to_string(),
            })?);

            if let Some(handle) = value.as_managed_reference() {
                return Ok(!handle.is_null());
            }

            if let Some(pointer) = value.as_raw_pointer() {
                return Ok(!pointer.is_null());
            }

            Ok(true)
        }
        mir::CheckConstraint::DivZero { divisor } => {
            let divisor = (*divisor)
                .value()
                .ok_or_else(|| Error::ConcreteMirRequired {
                    context: "divzero divisor".to_string(),
                })?;
            if let Ok((value, _)) = load_signed_value(state, divisor) {
                return Ok(value != 0);
            }

            let (value, _) = load_unsigned_value(state, divisor)?;
            Ok(value != 0)
        }
        mir::CheckConstraint::ShiftRange {
            value,
            bit_width,
            is_signed,
        } => {
            let bit_width = u64::from(*bit_width);
            let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "shift range value".to_string(),
            })?;

            if *is_signed {
                let (value, _) = load_signed_value(state, value)?;
                Ok(value >= 0 && (value as u64) < bit_width)
            } else {
                let (value, _) = load_unsigned_value(state, value)?;
                Ok(value < bit_width)
            }
        }
        mir::CheckConstraint::Narrow {
            value,
            to_width,
            is_signed,
        } => {
            let target_width = u32::from(*to_width);
            let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "narrow value".to_string(),
            })?;

            if *is_signed {
                let (value, _) = load_signed_value(state, value)?;
                let min_value = -(1_i128 << target_width.saturating_sub(1));
                let max_value = (1_i128 << target_width.saturating_sub(1)) - 1;
                let value = value as i128;
                Ok(value >= min_value && value <= max_value)
            } else {
                let (value, _) = load_unsigned_value(state, value)?;
                let max_value = if target_width >= 64 {
                    u128::from(u64::MAX)
                } else {
                    (1_u128 << target_width) - 1
                };
                Ok(u128::from(value) <= max_value)
            }
        }
        mir::CheckConstraint::Overflow {
            operator,
            left,
            right,
            is_signed,
        } => evaluate_overflow_check(
            state,
            *operator,
            (*left).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "overflow left".to_string(),
            })?,
            (*right).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "overflow right".to_string(),
            })?,
            *is_signed,
        ),
        mir::CheckConstraint::Type { value, expected } => {
            let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "type check value".to_string(),
            })?;
            let expected = (*expected).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "type check expected".to_string(),
            })?;
            if let Some(actual) = actual_reference_type(state, value)? {
                return Ok(actual == expected);
            }

            let value = state.get(value);
            if let Some((actual, _)) = value.as_uint_with_width() {
                return Ok(actual == u64::from(expected.id));
            }
            if let Some((actual, _)) = value.as_int_with_width()
                && actual >= 0
            {
                return Ok(actual as u64 == u64::from(expected.id));
            }

            Err(Error::TypeMismatch {
                expected: "type descriptor or typed reference".to_string(),
                actual: format!("{value:?}"),
            })
        }
        mir::CheckConstraint::Union { value, expected } => {
            let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "union check value".to_string(),
            })?;
            if let Ok((actual, _)) = load_unsigned_value(state, value) {
                return Ok(actual == *expected);
            }

            let (actual, _) = load_signed_value(state, value)?;
            Ok(actual >= 0 && actual as u64 == *expected)
        }
        mir::CheckConstraint::ReceiverType { receiver, expected } => {
            let receiver = (*receiver)
                .value()
                .ok_or_else(|| Error::ConcreteMirRequired {
                    context: "receiver type check receiver".to_string(),
                })?;
            let expected = (*expected).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "receiver type check expected".to_string(),
            })?;
            let actual = actual_reference_type(state, receiver)?;
            Ok(actual == Some(expected))
        }
        mir::CheckConstraint::Implements { receiver, expected } => {
            let receiver = (*receiver)
                .value()
                .ok_or_else(|| Error::ConcreteMirRequired {
                    context: "implements check receiver".to_string(),
                })?;
            let expected = (*expected).ty().ok_or_else(|| Error::ConcreteMirRequired {
                context: "implements check expected".to_string(),
            })?;
            let Some(actual) = actual_reference_type(state, receiver)? else {
                return Ok(false);
            };

            Ok(state
                .tree()
                .metadata
                .dispatch
                .itab_id(actual, expected)
                .is_some())
        }
    }
}

/// Step assume (optimizer hint).
pub(crate) fn step_assume(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction data
    let InstructionData::Assume = &block[pc].data else {
        unreachable!()
    };

    // no op: assume is handled by the optimizer

    // continue to next instruction
    next!(state, block, pc)
}

/// Step return (exits tail-call chain).
pub(crate) fn step_return(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Return { value } = &block[pc].data else {
        unreachable!()
    };

    // resolve return value
    let return_value = if is_invalid_value(*value) {
        Value::VOID
    } else {
        state.get(*value)
    };

    // return to caller
    Transfer::Return(return_value)
}

/// Step yield (exits tail-call chain).
pub(crate) fn step_yield(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Yield {
        value,
        resume_point,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve yielded value
    let yield_value = state.get(*value);

    // return yield control
    Transfer::Yield {
        value: yield_value,
        resume_point: *resume_point,
    }
}

/// Step unconditional jump (exits tail-call chain).
pub(crate) fn step_jump(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Jump { target, copies } = &block[pc].data else {
        unreachable!()
    };

    // return jump control
    Transfer::Jump {
        block: *target,
        copies: *copies,
    }
}

/// Step conditional branch (exits tail-call chain).
pub(crate) fn step_branch(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Branch {
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

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step boolean branch (exits tail-call chain).
pub(crate) fn step_branch_bool(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Branch {
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

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step semantic check (exits tail-call chain).
pub(crate) fn step_check(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Check {
        constraint,
        then_target,
        then_copies,
        else_target,
        else_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // evaluate the semantic guard
    let is_truthy = evaluate_check_constraint(state, constraint).unwrap_or(false);

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step fused compare-and-branch for signed integers (most common).
#[inline(always)]
pub(crate) fn step_compare_and_branch_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranch {
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

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step fused compare-and-branch for unsigned integers.
#[inline(always)]
pub(crate) fn step_compare_and_branch_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranch {
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

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step fused compare-and-branch for floats.
#[inline(always)]
pub(crate) fn step_compare_and_branch_float(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranch {
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

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step fused compare-and-branch (generic fallback).
pub(crate) fn step_compare_and_branch(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranch {
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

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step fused compare-and-branch with constant right operand for signed integers.
#[inline(always)]
pub(crate) fn step_compare_and_branch_const_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranchConst {
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

    // record the branch and return the chosen jump
    record_branch(state);

    branch_transfer(
        is_truthy,
        *then_target,
        *then_copies,
        *else_target,
        *else_copies,
    )
}

/// Step fused compare-and-branch with constant right operand for unsigned integers.
#[inline(always)]
pub(crate) fn step_compare_and_branch_const_uint(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranchConst {
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

    // record the branch before selecting a target
    record_branch(state);

    // branch based on comparison result
    if is_truthy {
        Transfer::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        Transfer::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Step fused compare-and-branch with constant right operand for floats.
#[inline(always)]
pub(crate) fn step_compare_and_branch_const_float(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranchConst {
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
        stat_inc!(state.engine.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        Transfer::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        Transfer::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Step fused compare-and-branch with constant right operand (generic fallback).
pub(crate) fn step_compare_and_branch_const(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::CompareAndBranchConst {
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
        stat_inc!(state.engine.statistics, branches);
    }

    // branch based on comparison result
    if is_truthy {
        Transfer::Jump {
            block: *then_target,
            copies: *then_copies,
        }
    } else {
        Transfer::Jump {
            block: *else_target,
            copies: *else_copies,
        }
    }
}

/// Step switch (exits tail-call chain).
pub(crate) fn step_switch(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Switch {
        value,
        cases,
        default_target,
        default_copies,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // load switch value
    let int_val = match load_switch_value(state, *value) {
        Ok(int_val) => int_val,
        Err(error) => return Transfer::Error(error),
    };

    // update branch statistics
    if state.collect_stats {
        stat_inc!(state.engine.statistics, branches);
    }

    // find matching case
    let case_slice = state.switch_cases(*cases);
    for case in case_slice {
        if case.value == int_val {
            // forward case copies
            return Transfer::Jump {
                block: case.target,
                copies: case.copies,
            };
        }
    }

    // otherwise jump to the default target
    default_switch_transfer(*default_target, *default_copies)
}

/// Step switch via dense jump table (exits tail-call chain).
pub(crate) fn step_switch_table(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::SwitchTable {
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
    let int_val = match load_switch_value(state, *value) {
        Ok(int_val) => int_val,
        Err(error) => return Transfer::Error(error),
    };

    // record the branch before selecting a target
    record_branch(state);

    // resolve jump table entry
    if int_val < *min {
        return default_switch_transfer(*default_target, *default_copies);
    }
    let offset = (int_val - *min) as usize;
    let case_slice = state.switch_cases(*table);
    let Some(case) = case_slice.get(offset) else {
        return default_switch_transfer(*default_target, *default_copies);
    };

    // jump to resolved case
    Transfer::Jump {
        block: case.target,
        copies: case.copies,
    }
}

/// Step integer switch (exits tail-call chain).
pub(crate) fn step_switch_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::Switch {
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

    // record the branch before selecting a target
    record_branch(state);

    // find matching case
    let case_slice = state.switch_cases(*cases);
    for case in case_slice {
        if case.value == int_val {
            // forward case copies
            return Transfer::Jump {
                block: case.target,
                copies: case.copies,
            };
        }
    }

    // otherwise jump to the default target
    default_switch_transfer(*default_target, *default_copies)
}

/// Step integer switch via dense jump table (exits tail-call chain).
pub(crate) fn step_switch_table_int(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let InstructionData::SwitchTable {
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

    // record the branch before selecting a target
    record_branch(state);

    // resolve jump table entry
    if int_val < *min {
        return default_switch_transfer(*default_target, *default_copies);
    }
    let offset = (int_val - *min) as usize;
    let case_slice = state.switch_cases(*table);
    let Some(case) = case_slice.get(offset) else {
        return default_switch_transfer(*default_target, *default_copies);
    };

    // jump to resolved case
    Transfer::Jump {
        block: case.target,
        copies: case.copies,
    }
}

/// Step unreachable (errors).
pub(crate) fn step_trap(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    let InstructionData::Trap { kind, payload } = &block[pc].data else {
        unreachable!()
    };

    match kind {
        mir::TrapKind::Abort => Transfer::Error(Error::Abort),
        mir::TrapKind::Panic => {
            // decode the panic payload as a managed string when present
            if is_invalid_value(*payload) {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "non null readonly managed string".to_string(),
                    actual: "missing panic payload".to_string(),
                });
            }

            let payload = state.get(*payload);
            let message = match state.string_interner.string_value(state.heap(), payload) {
                Ok(message) => message,
                Err(error) => return Transfer::Error(error),
            };

            Transfer::Error(Error::Panic { message })
        }
    }
}

/// Step throw terminator.
pub(crate) fn step_throw(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    let InstructionData::Throw { value } = &block[pc].data else {
        unreachable!()
    };

    let value = state.get(*value);

    Transfer::Throw(value)
}

/// Step unreachable (errors).
pub(crate) fn step_unreachable(
    state: &mut StepState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    state.maybe_profile_instruction(&block[pc]);

    // return unreachable error
    Transfer::Error(Error::Unreachable)
}
