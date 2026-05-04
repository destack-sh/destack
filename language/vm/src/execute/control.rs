use super::scalar::{ScalarLayout, scalar_layout};
use crate::Word;
use crate::diagnostic::Error;
use crate::interpreter::Machine;
use crate::program::{
    CheckId, Edge, EdgeId, Instruction, MoveRange, SwitchCasesId, SwitchTableId, Transfer,
};
use {destack_engine as engine, destack_mir as mir};

/// Return one pooled control edge.
#[inline(always)]
fn control_edge(machine: &Machine<'_, '_>, id: u32) -> Edge {
    let table = machine.side_table_ptr();

    unsafe { (*table).edge(EdgeId(id)) }
}

/// Return one branch jump based on the evaluated condition.
#[inline(always)]
fn branch_transfer(is_truthy: bool, then_edge: Edge, else_edge: Edge) -> Transfer {
    let edge = if is_truthy { then_edge } else { else_edge };

    Transfer::Jump {
        block: edge.target,
        moves: edge.moves,
    }
}

/// Load one fused comparison branch.
#[inline(always)]
fn compare_branch_words<'a>(
    machine: &Machine<'_, 'a>,
    instruction: &'a Instruction,
) -> (Word, Word, Edge, Edge) {
    let left = machine.get_word(mir::Value::new(instruction.a));
    let right = machine.get_word(mir::Value::new(instruction.b));
    let then_edge = control_edge(machine, instruction.c);
    let else_edge = control_edge(machine, instruction.d);

    (left, right, then_edge, else_edge)
}

/// Return one fused comparison branch transfer.
#[inline(always)]
fn compare_branch_transfer(then_edge: Edge, else_edge: Edge, is_truthy: bool) -> Transfer {
    branch_transfer(is_truthy, then_edge, else_edge)
}

/// Return one default switch jump.
#[inline(always)]
fn default_switch_transfer(edge: Edge) -> Transfer {
    Transfer::Jump {
        block: edge.target,
        moves: edge.moves,
    }
}

/// Load one switch operand as an integer case value.
#[inline(always)]
fn load_switch_value(machine: &Machine<'_, '_>, value: mir::Value) -> Result<Option<i128>, Error> {
    let value_type = machine.value_type(value)?;
    let layout = scalar_layout(machine.tree(), value_type)?;
    let ScalarLayout::Int { width, is_signed } = layout else {
        return Err(Error::TypeMismatch {
            expected: "integer switch value".to_string(),
            actual: format!("{layout:?}"),
        });
    };

    integer_bytes_to_case_value(machine.value_bytes(value)?, width, is_signed)
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
fn load_signed_value(machine: &Machine<'_, '_>, value: mir::Value) -> Result<(i64, u8), Error> {
    let ty = machine.value_type(value)?;
    let ty = scalar_layout(machine.tree(), ty)?;
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
    let value = machine.get(value);

    Ok((value.as_i64(), width))
}

/// Load one value as an unsigned integer.
#[inline(always)]
fn load_unsigned_value(machine: &Machine<'_, '_>, value: mir::Value) -> Result<(u64, u8), Error> {
    let ty = machine.value_type(value)?;
    let ty = scalar_layout(machine.tree(), ty)?;
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
    let value = machine.get(value);

    Ok((value.as_u64(), width))
}

/// Load one value as a non-negative length.
#[inline(always)]
fn load_length_value(machine: &Machine<'_, '_>, value: mir::Value) -> Result<u64, Error> {
    let ty = machine.value_type(value)?;
    let ty = scalar_layout(machine.tree(), ty)?;
    let value = machine.get(value);

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
    machine: &Machine<'_, '_>,
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    is_signed: bool,
) -> Result<bool, Error> {
    if is_signed {
        let (left, width) = load_signed_value(machine, left)?;
        let (right, right_width) = load_signed_value(machine, right)?;
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

    let (left, width) = load_unsigned_value(machine, left)?;
    let (right, right_width) = load_unsigned_value(machine, right)?;
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
    machine: &Machine<'_, '_>,
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
                machine,
                (*length)
                    .value()
                    .ok_or_else(|| Error::MissingRepresentation {
                        context: "bounds check length".to_string(),
                    })?,
            )?;

            if *is_signed {
                let (index, _) = load_signed_value(
                    machine,
                    (*index)
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "bounds check index".to_string(),
                        })?,
                )?;
                Ok(index >= 0 && (index as u64) < length)
            } else {
                let (index, _) = load_unsigned_value(
                    machine,
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
                machine.get(
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
            let ty = machine.value_type(divisor)?;
            let ty = scalar_layout(machine.tree(), ty)?;

            match ty {
                ScalarLayout::Int {
                    is_signed: true, ..
                } => {
                    let (value, _) = load_signed_value(machine, divisor)?;

                    Ok(value != 0)
                }
                ScalarLayout::Int {
                    is_signed: false, ..
                } => {
                    let (value, _) = load_unsigned_value(machine, divisor)?;

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
                let (value, _) = load_signed_value(machine, value)?;
                Ok(value >= 0 && (value as u64) < bit_width)
            } else {
                let (value, _) = load_unsigned_value(machine, value)?;
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
                let (value, _) = load_signed_value(machine, value)?;
                let min_value = -(1_i128 << target_width.saturating_sub(1));
                let max_value = (1_i128 << target_width.saturating_sub(1)) - 1;
                let value = value as i128;
                Ok(value >= min_value && value <= max_value)
            } else {
                let (value, _) = load_unsigned_value(machine, value)?;
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
            machine,
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
            let value = machine.get(value);

            Ok(value.as_u64() == u64::from(expected.id))
        }
        mir::CheckConstraint::Union { value, expected } => {
            let value = (*value)
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "union check value".to_string(),
                })?;
            let ty = machine.value_type(value)?;
            let ty = scalar_layout(machine.tree(), ty)?;

            match ty {
                ScalarLayout::Int {
                    is_signed: false, ..
                } => Ok(machine.get(value).as_u64() == *expected),
                ScalarLayout::Int {
                    is_signed: true, ..
                } => {
                    let actual = machine.get(value).as_i64();

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
    _machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let _ = instruction;

    Transfer::Continue
}

/// Execute return (exits tail-call chain).
pub(crate) fn execute_return(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    // resolve return value
    let return_value = machine.value_operand(mir::Value::new(instruction.a));
    let return_value = match return_value {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // return to caller
    Transfer::Return(return_value)
}

/// Execute void return.
pub(crate) fn execute_return_void(
    _machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let _ = instruction;

    Transfer::Return(Word::VOID)
}

/// Execute yield (exits tail-call chain).
pub(crate) fn execute_yield(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let value = mir::Value::new(instruction.a);
    let source = mir::Value::new(instruction.b);
    let frame_state = engine::FrameStateId(instruction.c);

    // resolve yielded value
    let yield_value = match machine.value_operand(value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    // return yield control
    Transfer::Yield {
        value: yield_value,
        source,
        frame_state,
    }
}

/// Execute unconditional jump (exits tail-call chain).
#[inline(always)]
pub(crate) fn execute_jump(_machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let moves = MoveRange {
        start: instruction.b,
        len: instruction.c,
    };

    // return jump control
    Transfer::Jump {
        block: instruction.a,
        moves,
    }
}

/// Execute boolean branch (exits tail-call chain).
#[inline(always)]
pub(crate) fn execute_branch_bool(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let condition = mir::Value::new(instruction.a);
    let then_edge = control_edge(machine, instruction.b);
    let else_edge = control_edge(machine, instruction.c);

    // evaluate branch condition
    let cond = machine.get(condition);
    let is_truthy = cond.bits() != 0;

    branch_transfer(is_truthy, then_edge, else_edge)
}

/// Execute runtime check (exits tail-call chain).
pub(crate) fn execute_check(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let table = machine.side_table_ptr();
    let constraint = unsafe { (*table).check(CheckId(instruction.a)) };
    let then_edge = control_edge(machine, instruction.b);
    let else_edge = control_edge(machine, instruction.c);

    // evaluate the runtime guard
    let is_truthy = match evaluate_check(machine, constraint) {
        Ok(is_truthy) => is_truthy,
        Err(error) => return Transfer::Error(error),
    };

    branch_transfer(is_truthy, then_edge, else_edge)
}

/// Execute integer equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() == right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute integer inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() != right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) < (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) <= (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) > (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute signed integer greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_int(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = (left.bits() as i64) >= (right.bits() as i64);

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() < right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() <= right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() > right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute unsigned integer greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_uint(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.bits() >= right.bits();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() == right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() != right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() < right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() <= right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() > right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float32 greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_f32(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f32() >= right.as_f32();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 equality branch.
#[inline(always)]
pub(crate) fn execute_branch_eq_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() == right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 inequality branch.
#[inline(always)]
pub(crate) fn execute_branch_ne_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() != right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 less-than branch.
#[inline(always)]
pub(crate) fn execute_branch_lt_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() < right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 less-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_le_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() <= right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 greater-than branch.
#[inline(always)]
pub(crate) fn execute_branch_gt_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() > right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute float64 greater-or-equal branch.
#[inline(always)]
pub(crate) fn execute_branch_ge_f64(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let (left, right, then_edge, else_edge) = compare_branch_words(machine, instruction);
    let is_truthy = left.as_f64() >= right.as_f64();

    compare_branch_transfer(then_edge, else_edge, is_truthy)
}

/// Execute switch (exits tail-call chain).
pub(crate) fn execute_switch(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let table = machine.side_table_ptr();
    let value = mir::Value::new(instruction.a);
    let cases = unsafe { (*table).switch_cases(SwitchCasesId(instruction.b)) };
    let default_edge = control_edge(machine, instruction.c);

    // load switch value
    let int_val = match load_switch_value(machine, value) {
        Ok(Some(int_val)) => int_val,
        Ok(None) => return default_switch_transfer(default_edge),
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
    default_switch_transfer(default_edge)
}

/// Execute switch via dense jump table (exits tail-call chain).
pub(crate) fn execute_switch_table(
    machine: &mut Machine<'_, '_>,
    instruction: &Instruction,
) -> Transfer {
    let side_table = machine.side_table_ptr();
    let value = mir::Value::new(instruction.a);
    let table = unsafe { (*side_table).switch_table(SwitchTableId(instruction.b)) };
    let default_edge = control_edge(machine, instruction.c);

    // load switch value
    let int_val = match load_switch_value(machine, value) {
        Ok(Some(int_val)) => int_val,
        Ok(None) => return default_switch_transfer(default_edge),
        Err(error) => return Transfer::Error(error),
    };

    // resolve jump table entry
    if int_val < table.min {
        return default_switch_transfer(default_edge);
    }
    let offset = (int_val - table.min) as usize;
    let Some(case) = table.cases.get(offset) else {
        return default_switch_transfer(default_edge);
    };

    // jump to resolved case
    Transfer::Jump {
        block: case.target,
        moves: case.moves,
    }
}

/// Execute abort.
pub(crate) fn execute_abort(
    _machine: &mut Machine<'_, '_>,
    _instruction: &Instruction,
) -> Transfer {
    Transfer::Error(Error::Abort)
}

/// Execute panic.
pub(crate) fn execute_panic(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let payload = machine.get(mir::Value::new(instruction.a));
    let message = format!("panic payload: {payload:?}");

    Transfer::Error(Error::Panic { message })
}

/// Execute throw terminator.
pub(crate) fn execute_throw(machine: &mut Machine<'_, '_>, instruction: &Instruction) -> Transfer {
    let value = mir::Value::new(instruction.a);

    let value = match machine.value_operand(value) {
        Ok(value) => value,
        Err(error) => return Transfer::Error(error),
    };

    Transfer::Throw(value)
}

/// Execute unreachable (errors).
pub(crate) fn execute_unreachable(
    _machine: &mut Machine<'_, '_>,
    _instruction: &Instruction,
) -> Transfer {
    // return unreachable error
    Transfer::Error(Error::Unreachable)
}
