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
