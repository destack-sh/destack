use super::*;

/// Handle assume (optimizer hint).
pub(crate) fn handle_assume(
    state: &mut ThreadedState<'_, '_>,
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
/// Handle return (exits tail-call chain).
pub(crate) fn handle_return(
    state: &mut ThreadedState<'_, '_>,
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

/// Handle yield (exits tail-call chain).
pub(crate) fn handle_yield(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // decode instruction data
    let ThreadedInstructionData::Yield {
        value,
        resume_block,
        resume_copies,
        resume_value,
    } = &block[pc].data
    else {
        unreachable!()
    };

    // resolve yielded value
    let yield_value = state.get(*value);

    // return yield control
    ControlFlow::Yield {
        value: yield_value,
        resume_block: *resume_block,
        resume_copies: *resume_copies,
        resume_value: *resume_value,
    }
}

/// Handle unconditional jump (exits tail-call chain).
pub(crate) fn handle_jump(
    state: &mut ThreadedState<'_, '_>,
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
pub(crate) fn handle_branch(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_branch_bool(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch_int(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch_uint(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch_float(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch_const_int(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch_const_uint(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch_const_float(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_compare_and_branch_const(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_switch(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_switch_table(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_switch_int(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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
pub(crate) fn handle_switch_table_int(
    state: &mut ThreadedState<'_, '_>,
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
        stat_inc!(state.interpreter.engine.statistics, branches);
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

/// Handle unreachable (errors).
pub(crate) fn handle_trap(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    let ThreadedInstructionData::Trap { kind, payload } = &block[pc].data else {
        unreachable!()
    };

    match kind {
        mir::TrapKind::Abort => ControlFlow::Error(Error::Abort),
        mir::TrapKind::Panic => {
            // decode the panic payload as a managed string when present
            if is_invalid_value(*payload) {
                return ControlFlow::Error(Error::TypeMismatch {
                    expected: "non null readonly managed string".to_string(),
                    actual: "missing panic payload".to_string(),
                });
            }

            let payload = state.get(*payload);
            let message = match state.interpreter.isolate.string_interner.string_value(
                state.interpreter.heap.managed(),
                state.interpreter.heap.raw(),
                payload,
            ) {
                Ok(message) => message,
                Err(error) => return ControlFlow::Error(error),
            };

            ControlFlow::Error(Error::Panic { message })
        }
    }
}

/// Handle unreachable (errors).
pub(crate) fn handle_unreachable(
    state: &mut ThreadedState<'_, '_>,
    block: &[ThreadedInstruction],
    pc: usize,
) -> ControlFlow {
    state.maybe_profile_instruction(&block[pc]);

    // return unreachable error
    ControlFlow::Error(Error::Unreachable)
}
