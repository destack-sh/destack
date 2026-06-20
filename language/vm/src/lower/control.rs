use destack_mir as mir;

use crate::lower::allocation::AllocationInitialization;
use crate::{Error, Result};
use destack_program::vm::{
    BoundsCheck, Check, Instruction, NarrowCheck, Op, OverflowCheck, ShiftRangeCheck, ValueShape,
    VariantCheck,
};

use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one MIR check into VM data.
    fn lower_check(&self, constraint: &mir::CheckConstraint) -> Result<Check> {
        match constraint {
            mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } => {
                let index = *index;
                let length = *length;
                let (_, length_signed) = checked_integer(self, length)?;
                let check = BoundsCheck {
                    index: cell_offset(self, index)?,
                    length: cell_offset(self, length)?,
                };

                Ok(bounds_check(*is_signed, length_signed, check))
            }
            mir::CheckConstraint::Null { value } => {
                let value = *value;

                Ok(Check::Null {
                    value: cell_offset(self, value)?,
                })
            }
            mir::CheckConstraint::DivZero { divisor } => {
                let divisor = *divisor;
                let (_, is_signed) = checked_integer(self, divisor)?;
                let divisor = cell_offset(self, divisor)?;

                Ok(div_zero_check(is_signed, divisor))
            }
            mir::CheckConstraint::ShiftRange {
                value,
                bit_width,
                is_signed,
            } => {
                let value = *value;
                if *bit_width == 0 {
                    return Err(Error::invalid_instruction());
                }
                let check = ShiftRangeCheck {
                    value: cell_offset(self, value)?,
                    bit_width: *bit_width,
                };

                Ok(shift_range_check(*is_signed, check))
            }
            mir::CheckConstraint::Narrow {
                value,
                to_width,
                is_signed,
            } => {
                let value = *value;
                if *to_width == 0 {
                    return Err(Error::invalid_instruction());
                }
                let check = NarrowCheck {
                    value: cell_offset(self, value)?,
                    to_width: *to_width,
                };

                Ok(narrow_check(*is_signed, check))
            }
            mir::CheckConstraint::Overflow {
                operator,
                left,
                right,
                is_signed,
            } => {
                let left = *left;
                let right = *right;
                let (width, _) = checked_integer(self, left)?;
                let (right_width, _) = checked_integer(self, right)?;
                if width != right_width {
                    return Err(Error::invalid_instruction());
                }

                let check = OverflowCheck {
                    left: cell_offset(self, left)?,
                    right: cell_offset(self, right)?,
                    width,
                };

                overflow_check(*operator, *is_signed, check)
            }
            mir::CheckConstraint::Type { value, expected } => {
                let value = *value;

                Ok(Check::Type {
                    value: cell_offset(self, value)?,
                    expected: expected.id,
                })
            }
            mir::CheckConstraint::Variant { value, expected } => {
                let value = *value;
                let check = VariantCheck {
                    value: cell_offset(self, value)?,
                    expected: constant_cell_bits(expected)?,
                };

                Ok(Check::Variant(check))
            }
            mir::CheckConstraint::ReceiverType { .. } => {
                Err(Error::unsupported_instruction("receiver.type check"))
            }
            mir::CheckConstraint::Implements { .. } => Err(Error::unsupported_instruction(
                "interface.conformance check",
            )),
        }
    }

    /// Convert a MIR terminator to lowered VM form.
    pub(super) fn lower_terminator(
        &self,
        term: &mir::Terminator,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        Ok(match term {
            mir::Terminator::Error => {
                return Err(Error::invalid_program("terminator"));
            }
            mir::Terminator::Return { value } => {
                let Some(value) = value else {
                    return Ok(Instruction::new(Op::ReturnVoid, 0, 0, 0, 0));
                };
                let value = *value;

                let value_type = self.value_type_for_value(value)?;
                let is_cell = self.layout_for_type(value_type)?.is_cell();

                let op = if is_cell {
                    Op::ReturnCell
                } else {
                    Op::ReturnAddress
                };

                Instruction::new(op, value_offset(self, value)?, 0, 0, 0)
            }

            mir::Terminator::Jump { target } => {
                let target_block = target.block;
                let arguments = self.target_arguments(target);
                let target_index = self.block_index_by_id[&target_block];
                let target_parameters = self.block_parameter[target_index].as_slice();
                let moves = pool.edge_moves(target_parameters, arguments)?;

                Instruction::new(Op::Jump, target_index as u32, moves.start, moves.len, 0)
            }

            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let condition = *condition;
                let then_target_block = then_target.block;
                let else_target_block = else_target.block;
                let then_arguments = self.target_arguments(then_target);
                let else_arguments = self.target_arguments(else_target);
                let then_index = self.block_index_by_id[&then_target_block];
                let else_index = self.block_index_by_id[&else_target_block];
                let then_parameters = self.block_parameter[then_index].as_slice();
                let else_parameters = self.block_parameter[else_index].as_slice();
                let then_moves = pool.edge_moves(then_parameters, then_arguments)?;
                let else_moves = pool.edge_moves(else_parameters, else_arguments)?;
                let then_edge = pool.edge(then_index as u32, then_moves);
                let else_edge = pool.edge(else_index as u32, else_moves);

                Instruction::new(
                    Op::BranchBool,
                    cell_offset(self, condition)?,
                    then_edge.0,
                    else_edge.0,
                    0,
                )
            }

            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let success_block = success.block;
                let failure_block = failure.block;
                let success_arguments = self.target_arguments(success);
                let failure_arguments = self.target_arguments(failure);
                let success_index = self.block_index_by_id[&success_block];
                let failure_index = self.block_index_by_id[&failure_block];
                let success_parameters = self.block_parameter[success_index].as_slice();
                let failure_parameters = self.block_parameter[failure_index].as_slice();
                let success_moves = pool.edge_moves(success_parameters, success_arguments)?;
                let failure_moves = pool.edge_moves(failure_parameters, failure_arguments)?;
                let constraint = pool.check(self.lower_check(constraint)?);
                let success_edge = pool.edge(success_index as u32, success_moves);
                let failure_edge = pool.edge(failure_index as u32, failure_moves);

                Instruction::new(Op::Check, constraint.0, success_edge.0, failure_edge.0, 0)
            }

            mir::Terminator::Switch {
                value,
                cases,
                default,
            } => {
                let value = *value;
                let default_block = default.block;
                let default_arguments = self.target_arguments(default);
                let default_index = self.block_index_by_id[&default_block];
                let default_parameters = self.block_parameter[default_index].as_slice();
                let default_moves = pool.edge_moves(default_parameters, default_arguments)?;
                let default_edge = pool.edge(default_index as u32, default_moves);
                let cases = self.switch_cases(*cases);

                let (is_cell, width, is_signed) = switch_layout(self.value_shape_map().get(value));
                let switch_layout = switch_layout_field(width, is_signed);
                if is_cell
                    && let Some(table) = pool.switch_table_range(
                        self.tree,
                        &self.block_index_by_id,
                        &self.block_parameter,
                        cases,
                        default_index as u32,
                        default_moves,
                    )?
                {
                    Instruction::new(
                        Op::SwitchTable,
                        cell_offset(self, value)?,
                        table.0,
                        default_edge.0,
                        switch_layout,
                    )
                } else {
                    let cases = pool.switch_case_range(
                        self.tree,
                        &self.block_index_by_id,
                        &self.block_parameter,
                        cases,
                    )?;
                    let value_offset = if is_cell {
                        cell_offset(self, value)?
                    } else {
                        value_offset(self, value)?
                    };

                    Instruction::new(
                        Op::Switch,
                        value_offset,
                        cases.0,
                        default_edge.0,
                        switch_layout,
                    )
                }
            }

            mir::Terminator::NewZeroedTry {
                layout,
                success,
                failure,
            } => self.lower_new_try(
                pool,
                *layout,
                success,
                failure,
                AllocationInitialization::Zeroed,
            )?,

            mir::Terminator::NewUninitTry {
                layout,
                success,
                failure,
            } => self.lower_new_try(
                pool,
                *layout,
                success,
                failure,
                AllocationInitialization::Uninit,
            )?,

            mir::Terminator::NewSliceZeroedTry {
                element,
                length,
                success,
                failure,
            } => self.lower_new_slice_try(
                pool,
                *element,
                *length,
                success,
                failure,
                AllocationInitialization::Zeroed,
            )?,

            mir::Terminator::NewSliceUninitTry {
                element,
                length,
                success,
                failure,
            } => self.lower_new_slice_try(
                pool,
                *element,
                *length,
                success,
                failure,
                AllocationInitialization::Uninit,
            )?,

            mir::Terminator::Panic { payload } => {
                let Some(payload) = payload else {
                    return Ok(Instruction::new(Op::Panic, 0, 0, 0, 0));
                };
                let payload = *payload;

                Instruction::new(Op::PanicValue, cell_offset(self, payload)?, 0, 0, 0)
            }

            mir::Terminator::UnwindResume => Instruction::new(Op::UnwindResume, 0, 0, 0, 0),

            mir::Terminator::Trap { .. } => Instruction::new(Op::Abort, 0, 0, 0, 0),

            mir::Terminator::Unreachable => Instruction::new(Op::Unreachable, 0, 0, 0, 0),

            mir::Terminator::Yield { value, .. } => {
                let value = *value;
                let frame_state = self
                    .yield_frame_states
                    .get(&self.block_id())
                    .copied()
                    .ok_or_else(|| {
                        Error::internal(format!(
                            "missing yield frame state for block: {:?}",
                            self.block_id()
                        ))
                    })?;

                let value_type = self.value_type_for_value(value)?;
                let is_cell = self.layout_for_type(value_type)?.is_cell();

                let op = if is_cell {
                    Op::YieldCell
                } else {
                    Op::YieldAddress
                };

                Instruction::new(
                    op,
                    value_offset(self, value)?,
                    self.index.type_id(value_type).0,
                    frame_state.0,
                    0,
                )
            }

            mir::Terminator::Call { .. }
            | mir::Terminator::CallIndirect { .. }
            | mir::Terminator::CallVirtual { .. }
            | mir::Terminator::CallDynamic { .. }
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallIndirect { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallDynamic { .. } => self.lower_call_terminator(term, pool)?,
        })
    }
}

/// Return the lowered switch integer layout.
fn switch_layout(layout: Option<ValueShape>) -> (bool, u16, bool) {
    match layout {
        Some(ValueShape::Int { width, signed }) if width <= u64::BITS as u16 => {
            (true, width, signed)
        }
        Some(ValueShape::Int { width, signed }) => (false, width, signed),
        _ => (false, 0, false),
    }
}

/// Pack one switch integer layout into an instruction operand.
fn switch_layout_field(width: u16, is_signed: bool) -> u32 {
    let sign = if is_signed { 1 << 16 } else { 0 };

    u32::from(width) | sign
}

/// Return one checked cell integer layout.
fn checked_integer(lowerer: &BlockLowerer<'_>, value: mir::Value) -> Result<(u8, bool)> {
    let value_type = lowerer.value_type_for_value(value)?;
    let value_type = lowerer.tree.repr_type(value_type);

    match lowerer.tree.get(value_type) {
        mir::Type::Int { width, is_signed } if *width > 0 && *width <= u64::BITS as u16 => {
            Ok((*width as u8, *is_signed))
        }
        mir::Type::Usize => Ok((lowerer.tree.pointer_bytes() * 8, false)),
        _ => Err(Error::type_mismatch(
            "cell integer",
            format!("{value_type:?}"),
        )),
    }
}

/// Return one constant as VM cell bits.
fn constant_cell_bits(value: &mir::Constant) -> Result<u64> {
    match value {
        mir::Constant::Null => Ok(0),
        mir::Constant::Boolean { value } => Ok(u64::from(*value)),
        mir::Constant::Int { value, width, .. } if *width <= u64::BITS as u16 => {
            Ok((*value as i64) as u64)
        }
        mir::Constant::UInt { value, width } if *width <= u64::BITS as u16 => {
            u64::try_from(*value).map_err(|_| Error::invalid_instruction())
        }
        mir::Constant::Float { bits, .. } => Ok(*bits),
        mir::Constant::Char { value } => Ok(u64::from(*value as u32)),
        mir::Constant::Int { .. } | mir::Constant::UInt { .. } => Err(Error::invalid_instruction()),
    }
}

/// Select one concrete overflow check.
fn overflow_check(
    operator: mir::BinaryOperator,
    is_signed: bool,
    check: OverflowCheck,
) -> Result<Check> {
    match (operator, is_signed) {
        (mir::BinaryOperator::Add, true) => Ok(Check::OverflowAddInt(check)),
        (mir::BinaryOperator::Add, false) => Ok(Check::OverflowAddUint(check)),
        (mir::BinaryOperator::Subtract, true) => Ok(Check::OverflowSubInt(check)),
        (mir::BinaryOperator::Subtract, false) => Ok(Check::OverflowSubUint(check)),
        (mir::BinaryOperator::Multiply, true) => Ok(Check::OverflowMulInt(check)),
        (mir::BinaryOperator::Multiply, false) => Ok(Check::OverflowMulUint(check)),
        (mir::BinaryOperator::SignedDivide | mir::BinaryOperator::SignedRemainder, true) => {
            Ok(Check::OverflowDivInt(check))
        }
        (mir::BinaryOperator::UnsignedDivide | mir::BinaryOperator::UnsignedRemainder, false) => {
            Ok(Check::OverflowDivUint(check))
        }
        _ => Err(Error::invalid_instruction()),
    }
}

/// Select one concrete bounds check.
fn bounds_check(index_signed: bool, length_signed: bool, check: BoundsCheck) -> Check {
    match (index_signed, length_signed) {
        (true, true) => Check::BoundsIntInt(check),
        (true, false) => Check::BoundsIntUint(check),
        (false, true) => Check::BoundsUintInt(check),
        (false, false) => Check::BoundsUintUint(check),
    }
}

/// Select one concrete div-zero check.
fn div_zero_check(is_signed: bool, divisor: u32) -> Check {
    if is_signed {
        return Check::DivZeroInt { divisor };
    }

    Check::DivZeroUint { divisor }
}

/// Select one concrete shift range check.
fn shift_range_check(is_signed: bool, check: ShiftRangeCheck) -> Check {
    if is_signed {
        return Check::ShiftRangeInt(check);
    }

    Check::ShiftRangeUint(check)
}

/// Select one concrete narrow check.
fn narrow_check(is_signed: bool, check: NarrowCheck) -> Check {
    if is_signed {
        return Check::NarrowInt(check);
    }

    Check::NarrowUint(check)
}
