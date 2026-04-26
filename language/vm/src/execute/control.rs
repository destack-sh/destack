use super::prelude::*;
use crate::program::Transfer;

/// Return one branch jump based on the evaluated condition.
#[inline(always)]
fn branch_transfer(
    is_truthy: bool,
    then_target: u32,
    then_moves: MoveRange,
    else_target: u32,
    else_moves: MoveRange,
) -> Transfer {
    if is_truthy {
        return Transfer::Jump {
            block: then_target,
            moves: then_moves,
        };
    }

    Transfer::Jump {
        block: else_target,
        moves: else_moves,
    }
}

/// Return one default switch jump.
#[inline(always)]
fn default_switch_transfer(default_target: u32, default_moves: MoveRange) -> Transfer {
    Transfer::Jump {
        block: default_target,
        moves: default_moves,
    }
}

/// Load one switch operand as a signed integer.
#[inline(always)]
fn load_switch_value(state: &DispatchState<'_, '_>, value: mir::Value) -> Result<i64, Error> {
    let value = state.get(value);

    Ok(value.as_i64())
}

/// Load one value as a signed integer.
#[inline(always)]
fn load_signed_value(state: &DispatchState<'_, '_>, value: mir::Value) -> Result<(i64, u8), Error> {
    let ty = state.value_type(value)?;
    let ty = scalar_layout(state.tree(), ty)?;
    let ScalarLayout::Int {
        width,
        is_signed: true,
    } = ty
    else {
        return Err(Error::TypeMismatch {
            expected: "signed integer".to_string(),
            actual: format!("{ty:?}"),
        });
    };
    let width = u8::try_from(width).map_err(|_| Error::TypeMismatch {
        expected: "integer width <= 64".to_string(),
        actual: width.to_string(),
    })?;
    let value = state.get(value);

    Ok((value.as_i64(), width))
}

/// Load one value as an unsigned integer.
#[inline(always)]
fn load_unsigned_value(
    state: &DispatchState<'_, '_>,
    value: mir::Value,
) -> Result<(u64, u8), Error> {
    let ty = state.value_type(value)?;
    let ty = scalar_layout(state.tree(), ty)?;
    let ScalarLayout::Int {
        width,
        is_signed: false,
    } = ty
    else {
        return Err(Error::TypeMismatch {
            expected: "unsigned integer".to_string(),
            actual: format!("{ty:?}"),
        });
    };
    let width = u8::try_from(width).map_err(|_| Error::TypeMismatch {
        expected: "integer width <= 64".to_string(),
        actual: width.to_string(),
    })?;
    let value = state.get(value);

    Ok((value.as_u64(), width))
}

/// Load one value as a non-negative length.
#[inline(always)]
fn load_length_value(state: &DispatchState<'_, '_>, value: mir::Value) -> Result<u64, Error> {
    let ty = state.value_type(value)?;
    let ty = scalar_layout(state.tree(), ty)?;
    let value = state.get(value);

    match ty {
        ScalarLayout::Int {
            is_signed: false, ..
        } => Ok(value.as_u64()),
        ScalarLayout::Int {
            is_signed: true, ..
        } if value.as_i64() >= 0 => Ok(value.as_i64() as u64),
        _ => Err(Error::TypeMismatch {
            expected: "non negative integer".to_string(),
            actual: format!("{value:?}"),
        }),
    }
}

/// Evaluate one overflow guard.
fn evaluate_overflow_check(
    state: &DispatchState<'_, '_>,
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
    state: &DispatchState<'_, '_>,
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

            Ok(value.bits() != 0)
        }
        mir::CheckConstraint::DivZero { divisor } => {
            let divisor = (*divisor)
                .value()
                .ok_or_else(|| Error::ConcreteMirRequired {
                    context: "divzero divisor".to_string(),
                })?;
            let ty = state.value_type(divisor)?;
            let ty = scalar_layout(state.tree(), ty)?;

            match ty {
                ScalarLayout::Int {
                    is_signed: true, ..
                } => {
                    let (value, _) = load_signed_value(state, divisor)?;

                    Ok(value != 0)
                }
                ScalarLayout::Int {
                    is_signed: false, ..
                } => {
                    let (value, _) = load_unsigned_value(state, divisor)?;

                    Ok(value != 0)
                }
                _ => Err(Error::TypeMismatch {
                    expected: "integer divisor".to_string(),
                    actual: format!("{ty:?}"),
                }),
            }
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
            let value = state.get(value);

            Ok(value.as_u64() == u64::from(expected.id))
        }
        mir::CheckConstraint::Union { value, expected } => {
            let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "union check value".to_string(),
            })?;
            let ty = state.value_type(value)?;
            let ty = scalar_layout(state.tree(), ty)?;

            match ty {
                ScalarLayout::Int {
                    is_signed: false, ..
                } => Ok(state.get(value).as_u64() == *expected),
                ScalarLayout::Int {
                    is_signed: true, ..
                } => {
                    let actual = state.get(value).as_i64();

                    Ok(actual >= 0 && actual as u64 == *expected)
                }
                _ => Err(Error::TypeMismatch {
                    expected: "union discriminator".to_string(),
                    actual: format!("{ty:?}"),
                }),
            }
        }
        mir::CheckConstraint::ReceiverType { .. } => Err(Error::UnsupportedInstruction {
            name: "receiverType check".to_string(),
        }),
        mir::CheckConstraint::Implements { .. } => Err(Error::UnsupportedInstruction {
            name: "interfaceConformance check".to_string(),
        }),
    }
}

/// Execute assume (optimizer hint).
pub(crate) fn execute_assume(
    _state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Assume = &block[pc].operands else {
        unreachable!()
    };

    // no op: assume is handled by the optimizer

    // continue to next instruction
    Transfer::Continue
}

/// Execute return (exits tail-call chain).
pub(crate) fn execute_return(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Return { value } = &block[pc].operands else {
        unreachable!()
    };

    // resolve return value
    let return_value = if is_invalid_value(*value) {
        Ok(Word::VOID)
    } else {
        state.value_operand(*value)
    };
    let return_value = match return_value {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // return to caller
    Transfer::Return(return_value)
}

/// Execute yield (exits tail-call chain).
pub(crate) fn execute_yield(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Yield {
        value,
        source,
        resume_point,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // resolve yielded value
    let yield_value = match state.value_operand(*value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // return yield control
    Transfer::Yield {
        value: yield_value,
        source: *source,
        resume_point: *resume_point,
    }
}

/// Execute unconditional jump (exits tail-call chain).
pub(crate) fn execute_jump(
    _state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Jump { target, moves } = &block[pc].operands else {
        unreachable!()
    };

    // return jump control
    Transfer::Jump {
        block: *target,
        moves: *moves,
    }
}

/// Execute conditional branch (exits tail-call chain).
pub(crate) fn execute_branch(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Branch {
        condition,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // evaluate branch condition
    let cond = state.get(*condition);
    let is_truthy = cond.as_bool();

    // record the branch and return the chosen jump

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute boolean branch (exits tail-call chain).
pub(crate) fn execute_branch_bool(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Branch {
        condition,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // evaluate branch condition
    let cond = state.get(*condition);
    let is_truthy = cond.bits() != 0;

    // record the branch and return the chosen jump

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute semantic check (exits tail-call chain).
pub(crate) fn execute_check(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Check {
        constraint,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // evaluate the semantic guard
    let is_truthy = match evaluate_check_constraint(state, constraint) {
        Ok(is_truthy) => is_truthy,
        Err(error) => return Transfer::Error(error),
    };

    // record the branch and return the chosen jump

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute fused compare-and-branch for signed integers (most common).
#[inline(always)]
pub(crate) fn execute_compare_and_branch_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values as signed integers
    let lhs = state.get(*left).bits() as i64;
    let rhs = state.get(*right).bits() as i64;

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

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute fused compare-and-branch for unsigned integers.
#[inline(always)]
pub(crate) fn execute_compare_and_branch_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values as unsigned integers
    let lhs = state.get(*left).bits();
    let rhs = state.get(*right).bits();

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::UnsignedLessThan => lhs < rhs,
        mir::BinaryOperator::UnsignedLessEqual => lhs <= rhs,
        mir::BinaryOperator::UnsignedGreaterThan => lhs > rhs,
        mir::BinaryOperator::UnsignedGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };
    // record the branch and return the chosen jump

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute fused compare-and-branch for floats.
#[inline(always)]
pub(crate) fn execute_compare_and_branch_float(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values as floats
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

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute fused compare-and-branch (generic fallback).
pub(crate) fn execute_compare_and_branch(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranch {
        left,
        right,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = state.get(*right);

    // perform comparison inline
    let is_truthy = match operator {
        mir::BinaryOperator::Equal => lhs.bits() == rhs.bits(),
        mir::BinaryOperator::NotEqual => lhs.bits() != rhs.bits(),
        mir::BinaryOperator::SignedLessThan => (lhs.bits() as i64) < (rhs.bits() as i64),
        mir::BinaryOperator::SignedLessEqual => (lhs.bits() as i64) <= (rhs.bits() as i64),
        mir::BinaryOperator::SignedGreaterThan => (lhs.bits() as i64) > (rhs.bits() as i64),
        mir::BinaryOperator::SignedGreaterEqual => (lhs.bits() as i64) >= (rhs.bits() as i64),
        mir::BinaryOperator::UnsignedLessThan => lhs.bits() < rhs.bits(),
        mir::BinaryOperator::UnsignedLessEqual => lhs.bits() <= rhs.bits(),
        mir::BinaryOperator::UnsignedGreaterThan => lhs.bits() > rhs.bits(),
        mir::BinaryOperator::UnsignedGreaterEqual => lhs.bits() >= rhs.bits(),
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

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute fused compare-and-branch with constant right operand for signed integers.
#[inline(always)]
pub(crate) fn execute_compare_and_branch_const_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values as signed integers
    let lhs = state.get(*left).bits() as i64;
    let rhs = right_const.bits() as i64;

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

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute fused compare-and-branch with constant right operand for unsigned integers.
#[inline(always)]
pub(crate) fn execute_compare_and_branch_const_uint(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values as unsigned integers
    let lhs = state.get(*left).bits();
    let rhs = right_const.bits();

    // perform comparison
    let is_truthy = match operator {
        mir::BinaryOperator::UnsignedLessThan => lhs < rhs,
        mir::BinaryOperator::UnsignedLessEqual => lhs <= rhs,
        mir::BinaryOperator::UnsignedGreaterThan => lhs > rhs,
        mir::BinaryOperator::UnsignedGreaterEqual => lhs >= rhs,
        _ => unreachable!(),
    };

    // record the branch before selecting a target

    // branch based on comparison result
    if is_truthy {
        Transfer::Jump {
            block: *then_target,
            moves: *then_moves,
        }
    } else {
        Transfer::Jump {
            block: *else_target,
            moves: *else_moves,
        }
    }
}

/// Execute fused compare-and-branch with constant right operand for floats.
#[inline(always)]
pub(crate) fn execute_compare_and_branch_const_float(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values as floats
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

    // branch based on comparison result
    if is_truthy {
        Transfer::Jump {
            block: *then_target,
            moves: *then_moves,
        }
    } else {
        Transfer::Jump {
            block: *else_target,
            moves: *else_moves,
        }
    }
}

/// Execute fused compare-and-branch with constant right operand (generic fallback).
pub(crate) fn execute_compare_and_branch_const(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::CompareAndBranchConst {
        left,
        right_const,
        operator,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load values
    let lhs = state.get(*left);
    let rhs = *right_const;

    // perform comparison inline
    let is_truthy = match operator {
        mir::BinaryOperator::Equal => lhs.bits() == rhs.bits(),
        mir::BinaryOperator::NotEqual => lhs.bits() != rhs.bits(),
        mir::BinaryOperator::SignedLessThan => (lhs.bits() as i64) < (rhs.bits() as i64),
        mir::BinaryOperator::SignedLessEqual => (lhs.bits() as i64) <= (rhs.bits() as i64),
        mir::BinaryOperator::SignedGreaterThan => (lhs.bits() as i64) > (rhs.bits() as i64),
        mir::BinaryOperator::SignedGreaterEqual => (lhs.bits() as i64) >= (rhs.bits() as i64),
        mir::BinaryOperator::UnsignedLessThan => lhs.bits() < rhs.bits(),
        mir::BinaryOperator::UnsignedLessEqual => lhs.bits() <= rhs.bits(),
        mir::BinaryOperator::UnsignedGreaterThan => lhs.bits() > rhs.bits(),
        mir::BinaryOperator::UnsignedGreaterEqual => lhs.bits() >= rhs.bits(),
        mir::BinaryOperator::FloatEqual => lhs.as_float64() == rhs.as_float64(),
        mir::BinaryOperator::FloatNotEqual => lhs.as_float64() != rhs.as_float64(),
        mir::BinaryOperator::FloatLessThan => lhs.as_float64() < rhs.as_float64(),
        mir::BinaryOperator::FloatLessEqual => lhs.as_float64() <= rhs.as_float64(),
        mir::BinaryOperator::FloatGreaterThan => lhs.as_float64() > rhs.as_float64(),
        mir::BinaryOperator::FloatGreaterEqual => lhs.as_float64() >= rhs.as_float64(),
        _ => unreachable!("compare-and-branch with non-comparison operator"),
    };

    // branch based on comparison result
    if is_truthy {
        Transfer::Jump {
            block: *then_target,
            moves: *then_moves,
        }
    } else {
        Transfer::Jump {
            block: *else_target,
            moves: *else_moves,
        }
    }
}

/// Execute switch (exits tail-call chain).
pub(crate) fn execute_switch(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Switch {
        value,
        cases,
        default_target,
        default_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load switch value
    let int_val = match load_switch_value(state, *value) {
        Ok(int_val) => int_val,
        Err(error) => return Transfer::Error(error),
    };

    // find matching case
    for case in cases.as_ref() {
        if case.value == int_val {
            // forward case moves
            return Transfer::Jump {
                block: case.target,
                moves: case.moves,
            };
        }
    }

    // otherwise jump to the default target
    default_switch_transfer(*default_target, *default_moves)
}

/// Execute switch via dense jump table (exits tail-call chain).
pub(crate) fn execute_switch_table(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::SwitchTable {
        value,
        min,
        table,
        default_target,
        default_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load switch value
    let int_val = match load_switch_value(state, *value) {
        Ok(int_val) => int_val,
        Err(error) => return Transfer::Error(error),
    };

    // record the branch before selecting a target

    // resolve jump table entry
    if int_val < *min {
        return default_switch_transfer(*default_target, *default_moves);
    }
    let offset = (int_val - *min) as usize;
    let Some(case) = table.get(offset) else {
        return default_switch_transfer(*default_target, *default_moves);
    };

    // jump to resolved case
    Transfer::Jump {
        block: case.target,
        moves: case.moves,
    }
}

/// Execute integer switch (exits tail-call chain).
pub(crate) fn execute_switch_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::Switch {
        value,
        cases,
        default_target,
        default_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.bits() as i64;

    // record the branch before selecting a target

    // find matching case
    for case in cases.as_ref() {
        if case.value == int_val {
            // forward case moves
            return Transfer::Jump {
                block: case.target,
                moves: case.moves,
            };
        }
    }

    // otherwise jump to the default target
    default_switch_transfer(*default_target, *default_moves)
}

/// Execute integer switch via dense jump table (exits tail-call chain).
pub(crate) fn execute_switch_table_int(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    // decode instruction operands
    let Operands::SwitchTable {
        value,
        min,
        table,
        default_target,
        default_moves,
    } = &block[pc].operands
    else {
        unreachable!()
    };

    // load switch value
    let switch_val = state.get(*value);
    let int_val = switch_val.bits() as i64;

    // record the branch before selecting a target

    // resolve jump table entry
    if int_val < *min {
        return default_switch_transfer(*default_target, *default_moves);
    }
    let offset = (int_val - *min) as usize;
    let Some(case) = table.get(offset) else {
        return default_switch_transfer(*default_target, *default_moves);
    };

    // jump to resolved case
    Transfer::Jump {
        block: case.target,
        moves: case.moves,
    }
}

/// Execute unreachable (errors).
pub(crate) fn execute_trap(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::Trap { kind, payload } = &block[pc].operands else {
        unreachable!()
    };

    match kind {
        mir::TrapKind::Abort => Transfer::Error(Error::Abort),
        mir::TrapKind::Panic => {
            if is_invalid_value(*payload) {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "non null readonly heap reference".to_string(),
                    actual: "missing panic payload".to_string(),
                });
            }

            let payload = state.get(*payload);
            let message = format!("panic payload: {payload:?}");

            Transfer::Error(Error::Panic { message })
        }
    }
}

/// Execute throw terminator.
pub(crate) fn execute_throw(
    state: &mut DispatchState<'_, '_>,
    block: &[Instruction],
    pc: usize,
) -> Transfer {
    let Operands::Throw { value } = &block[pc].operands else {
        unreachable!()
    };

    let value = match state.value_operand(*value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::Throw(value)
}

/// Execute unreachable (errors).
pub(crate) fn execute_unreachable(
    _state: &mut DispatchState<'_, '_>,
    _block: &[Instruction],
    _pc: usize,
) -> Transfer {
    // return unreachable error
    Transfer::Error(Error::Unreachable)
}
