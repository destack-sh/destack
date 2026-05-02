use super::scalar::{ScalarLayout, scalar_layout};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::DispatchState;
use crate::program::{
    Assume, Branch, Check, CompareAndBranch, Instruction, Jump, MoveRange, Return, Switch,
    TableSwitch, Throw, Transfer, Trap, Yield,
};
use destack_mir as mir;

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

/// Load one fused comparison branch.
#[inline(always)]
fn compare_branch_words<'a>(
    state: &DispatchState<'_, '_>,
    instruction: &'a Instruction,
) -> (&'a CompareAndBranch, Word, Word) {
    let branch = instruction.payload_as::<CompareAndBranch>();
    let left = state.get_word(branch.left);
    let right = state.get_word(branch.right);

    (branch, left, right)
}

/// Return one fused comparison branch transfer.
#[inline(always)]
fn compare_branch_transfer(branch: &CompareAndBranch, is_truthy: bool) -> Transfer {
    branch_transfer(
        is_truthy,
        branch.then_target,
        branch.then_moves,
        branch.else_target,
        branch.else_moves,
    )
}

/// Return one default switch jump.
#[inline(always)]
fn default_switch_transfer(default_target: u32, default_moves: MoveRange) -> Transfer {
    Transfer::Jump {
        block: default_target,
        moves: default_moves,
    }
}

/// Load one switch operand as an integer case value.
#[inline(always)]
fn load_switch_value(
    state: &DispatchState<'_, '_>,
    value: mir::Value,
) -> Result<Option<i128>, Error> {
    let value_type = state.value_type(value)?;
    let layout = scalar_layout(state.tree(), value_type)?;
    let ScalarLayout::Int { width, is_signed } = layout else {
        return Err(Error::TypeMismatch {
            expected: "integer switch value".to_string(),
            actual: format!("{layout:?}"),
        });
    };

    integer_bytes_to_case_value(state.value_bytes(value)?, width, is_signed)
}

/// Decode integer bytes into the switch case domain.
fn integer_bytes_to_case_value(
    bytes: &[u8],
    width: u16,
    is_signed: bool,
) -> Result<Option<i128>, Error> {
    let byte_len = usize::from(width.div_ceil(8));
    let copied = byte_len.min(bytes.len()).min(16);
    let mut value = [0u8; 16];

    value[..copied].copy_from_slice(&bytes[..copied]);

    if width == 0 {
        return Ok(Some(0));
    }

    let sign_bit = 1u8 << ((width - 1) % 8);
    let sign_byte = usize::from((width - 1) / 8);
    let is_negative = is_signed && sign_byte < value.len() && value[sign_byte] & sign_bit != 0;

    if is_negative {
        value[copied..].fill(0xff);
    }

    let excess_bits = byte_len * 8 - usize::from(width);
    if excess_bits > 0 && sign_byte < value.len() {
        let mask = 0xffu8 >> excess_bits;
        value[sign_byte] &= mask;
        if is_negative {
            value[sign_byte] |= !mask;
        }
    }

    if is_signed {
        return Ok(Some(i128::from_le_bytes(value)));
    }

    let value = u128::from_le_bytes(value);

    Ok(i128::try_from(value).ok())
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

/// Evaluate one runtime check guard.
fn evaluate_check(
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
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "bounds check length".to_string(),
                    })?,
            )?;

            if *is_signed {
                let (index, _) = load_signed_value(
                    state,
                    (*index)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "bounds check index".to_string(),
                        })?,
                )?;
                Ok(index >= 0 && (index as u64) < length)
            } else {
                let (index, _) = load_unsigned_value(
                    state,
                    (*index)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "bounds check index".to_string(),
                        })?,
                )?;
                Ok(index < length)
            }
        }
        mir::CheckConstraint::Null { value } => {
            let value =
                state.get(
                    (*value)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "null check value".to_string(),
                        })?,
                );

            Ok(value.bits() != 0)
        }
        mir::CheckConstraint::DivZero { divisor } => {
            let divisor = (*divisor)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
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
            let value = (*value)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
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
            let value = (*value)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
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
            (*left)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "overflow left".to_string(),
                })?,
            (*right)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "overflow right".to_string(),
                })?,
            *is_signed,
        ),
        mir::CheckConstraint::Type { value, expected } => {
            let value = (*value)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "type check value".to_string(),
                })?;
            let expected = (*expected)
                .ty()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "type check expected".to_string(),
                })?;
            let value = state.get(value);

            Ok(value.as_u64() == u64::from(expected.id))
        }
        mir::CheckConstraint::Union { value, expected } => {
            let value = (*value)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let _ = instruction.payload_as::<Assume>();

    // no op: assume is handled by the optimizer

    // continue to next instruction
    Transfer::Continue
}

/// Execute return (exits tail-call chain).
pub(crate) fn execute_return(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Return { value } = instruction.payload_as::<Return>();

    // resolve return value
    let return_value = value.map_or(Ok(Word::VOID), |value| state.value_operand(value));
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Yield {
        value,
        source,
        frame_state,
    } = instruction.payload_as::<Yield>();

    // resolve yielded value
    let yield_value = match state.value_operand(*value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // return yield control
    Transfer::Yield {
        value: yield_value,
        source: *source,
        frame_state: *frame_state,
    }
}

/// Execute unconditional jump (exits tail-call chain).
pub(crate) fn execute_jump(
    _state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Jump { target, moves } = instruction.payload_as::<Jump>();

    // return jump control
    Transfer::Jump {
        block: *target,
        moves: *moves,
    }
}

/// Execute boolean branch (exits tail-call chain).
pub(crate) fn execute_branch_bool(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Branch {
        condition,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = instruction.payload_as::<Branch>();

    // evaluate branch condition
    let cond = state.get(*condition);
    let is_truthy = cond.bits() != 0;

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute runtime check (exits tail-call chain).
pub(crate) fn execute_check(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Check {
        constraint,
        then_target,
        then_moves,
        else_target,
        else_moves,
    } = instruction.payload_as::<Check>();
    let table = state.operand_table_ptr();
    let constraint = unsafe { (*table).check(*constraint) };

    // evaluate the runtime guard
    let is_truthy = match evaluate_check(state, constraint) {
        Ok(is_truthy) => is_truthy,
        Err(error) => return Transfer::Error(error),
    };

    branch_transfer(
        is_truthy,
        *then_target,
        *then_moves,
        *else_target,
        *else_moves,
    )
}

/// Execute integer equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.bits() == right.bits();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute integer inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.bits() != right.bits();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute signed integer less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = (left.bits() as i64) < (right.bits() as i64);

    compare_branch_transfer(branch, is_truthy)
}

/// Execute signed integer less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = (left.bits() as i64) <= (right.bits() as i64);

    compare_branch_transfer(branch, is_truthy)
}

/// Execute signed integer greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = (left.bits() as i64) > (right.bits() as i64);

    compare_branch_transfer(branch, is_truthy)
}

/// Execute signed integer greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_int(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = (left.bits() as i64) >= (right.bits() as i64);

    compare_branch_transfer(branch, is_truthy)
}

/// Execute unsigned integer less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.bits() < right.bits();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute unsigned integer less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.bits() <= right.bits();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute unsigned integer greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.bits() > right.bits();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute unsigned integer greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_uint(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.bits() >= right.bits();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float32 equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f32() == right.as_f32();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float32 inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f32() != right.as_f32();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float32 less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f32() < right.as_f32();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float32 less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f32() <= right.as_f32();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float32 greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f32() > right.as_f32();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float32 greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_f32(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f32() >= right.as_f32();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float64 equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f64() == right.as_f64();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float64 inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f64() != right.as_f64();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float64 less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f64() < right.as_f64();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float64 less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f64() <= right.as_f64();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float64 greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f64() > right.as_f64();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute float64 greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_f64(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (branch, left, right) = compare_branch_words(state, instruction);
    let is_truthy = left.as_f64() >= right.as_f64();

    compare_branch_transfer(branch, is_truthy)
}

/// Execute switch (exits tail-call chain).
pub(crate) fn execute_switch(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let Switch {
        value,
        cases,
        default_target,
        default_moves,
    } = instruction.payload_as::<Switch>();
    let table = state.operand_table_ptr();
    let cases = unsafe { (*table).switch_cases(*cases) };

    // load switch value
    let int_val = match load_switch_value(state, *value) {
        Ok(Some(int_val)) => int_val,
        Ok(None) => return default_switch_transfer(*default_target, *default_moves),
        Err(error) => return Transfer::Error(error),
    };

    // find matching case
    for case in cases {
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
    instruction: &Instruction,
) -> Transfer {
    // decode instruction operands
    let TableSwitch {
        value,
        table,
        default_target,
        default_moves,
    } = instruction.payload_as::<TableSwitch>();
    let operand_table = state.operand_table_ptr();
    let table = unsafe { (*operand_table).switch_table(*table) };

    // load switch value
    let int_val = match load_switch_value(state, *value) {
        Ok(Some(int_val)) => int_val,
        Ok(None) => return default_switch_transfer(*default_target, *default_moves),
        Err(error) => return Transfer::Error(error),
    };

    // resolve jump table entry
    if int_val < table.min {
        return default_switch_transfer(*default_target, *default_moves);
    }
    let offset = (int_val - table.min) as usize;
    let Some(case) = table.cases.get(offset) else {
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
    instruction: &Instruction,
) -> Transfer {
    let Trap { kind, payload } = instruction.payload_as::<Trap>();

    match kind {
        mir::TrapKind::Abort => Transfer::Error(Error::Abort),
        mir::TrapKind::Panic => {
            let Some(payload) = *payload else {
                return Transfer::Error(Error::TypeMismatch {
                    expected: "non null readonly heap reference".to_string(),
                    actual: "missing panic payload".to_string(),
                });
            };
            let payload = state.get(payload);
            let message = format!("panic payload: {payload:?}");

            Transfer::Error(Error::Panic { message })
        }
    }
}

/// Execute throw terminator.
pub(crate) fn execute_throw(
    state: &mut DispatchState<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let Throw { value } = instruction.payload_as::<Throw>();

    let value = match state.value_operand(*value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::Throw(value)
}

/// Execute unreachable (errors).
pub(crate) fn execute_unreachable(
    _state: &mut DispatchState<'_, '_>,
    _instruction: &Instruction,
) -> Transfer {
    // return unreachable error
    Transfer::Error(Error::Unreachable)
}
