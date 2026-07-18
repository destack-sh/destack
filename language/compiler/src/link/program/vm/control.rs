use destack_mir as mir;

use destack_program::AllocationInitialization;
use destack_program::vm::{
    BoundsCheck, Check, CheckKind, Instruction, NarrowCheck, Op, OverflowCheck, ShiftRangeCheck,
};

use crate::LinkResult;

use super::lower::BlockLowerer;
use super::pool::Pool;
use super::value::Operand;

impl<'a> BlockLowerer<'a> {
    /// Lower one MIR check into VM data.
    fn lower_check(&self, constraint: &mir::CheckConstraint) -> LinkResult<Check> {
        match constraint {
            mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } => {
                let index = *index;
                let length = *length;
                let (_, length_signed) = self.checked_integer(length)?;
                let check = BoundsCheck {
                    index: self.cell_offset(index)?,
                    length: self.cell_offset(length)?,
                };

                Ok(bounds_check(*is_signed, length_signed, check))
            }
            mir::CheckConstraint::Null { value } => {
                let value = *value;

                Ok(Check::null(self.cell_offset(value)?))
            }
            mir::CheckConstraint::DivZero { divisor } => {
                let divisor = *divisor;
                let (_, is_signed) = self.checked_integer(divisor)?;
                let divisor = self.cell_offset(divisor)?;

                Ok(div_zero_check(is_signed, divisor))
            }
            mir::CheckConstraint::ShiftRange {
                value,
                bit_width,
                is_signed,
            } => {
                let value = *value;
                if *bit_width == 0 {
                    return Err(self.invalid_instruction("shift range bit width"));
                }
                let check = ShiftRangeCheck {
                    value: self.cell_offset(value)?,
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
                    return Err(self.invalid_instruction("narrow check width"));
                }
                let check = NarrowCheck {
                    value: self.cell_offset(value)?,
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
                let (width, _) = self.checked_integer(left)?;
                let (right_width, _) = self.checked_integer(right)?;
                if width != right_width {
                    return Err(self.invalid_instruction("overflow operand width"));
                }

                let check = OverflowCheck {
                    left: self.cell_offset(left)?,
                    right: self.cell_offset(right)?,
                    width,
                };

                self.overflow_check(*operator, *is_signed, check)
            }
            mir::CheckConstraint::IsType { value, expected } => {
                let value = *value;
                let expected = self.function.program.type_id(*expected);

                Ok(Check::type_id(
                    CheckKind::TypeId,
                    self.type_id_cell_offset(value)?,
                    expected.0,
                ))
            }
            mir::CheckConstraint::IsSubtype { value, expected } => {
                let value = *value;
                let expected = self.function.program.type_id(*expected);

                Ok(Check::type_id(
                    CheckKind::SubtypeId,
                    self.type_id_cell_offset(value)?,
                    expected.0,
                ))
            }
        }
    }

    /// Convert a MIR terminator to lowered VM form.
    pub(super) fn lower_terminator(
        &self,
        term: &mir::Terminator,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        Ok(match term {
            mir::Terminator::Error => {
                return Err(self.invalid_input("terminator"));
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

                Instruction::new(op, self.value_offset(value)?, 0, 0, 0)
            }

            mir::Terminator::Jump { target } => {
                let target_block = target.block;
                let arguments = self.function.target_arguments(target);
                let target_index = self.function.block_index_by_id[&target_block];
                let target_parameters = self.function.block_parameters[target_index].as_slice();
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
                let then_arguments = self.function.target_arguments(then_target);
                let else_arguments = self.function.target_arguments(else_target);
                let then_index = self.function.block_index_by_id[&then_target_block];
                let else_index = self.function.block_index_by_id[&else_target_block];
                let then_parameters = self.function.block_parameters[then_index].as_slice();
                let else_parameters = self.function.block_parameters[else_index].as_slice();
                let then_moves = pool.edge_moves(then_parameters, then_arguments)?;
                let else_moves = pool.edge_moves(else_parameters, else_arguments)?;
                let then_edge = pool.edge(then_index as u32, then_moves);
                let else_edge = pool.edge(else_index as u32, else_moves);

                Instruction::new(
                    Op::BranchBool,
                    self.cell_offset(condition)?,
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
                let success_arguments = self.function.target_arguments(success);
                let failure_arguments = self.function.target_arguments(failure);
                let success_index = self.function.block_index_by_id[&success_block];
                let failure_index = self.function.block_index_by_id[&failure_block];
                let success_parameters = self.function.block_parameters[success_index].as_slice();
                let failure_parameters = self.function.block_parameters[failure_index].as_slice();
                let success_moves = pool.edge_moves(success_parameters, success_arguments)?;
                let failure_moves = pool.edge_moves(failure_parameters, failure_arguments)?;
                let constraint = pool.check(self.lower_check(constraint)?);
                let success_edge = pool.edge(success_index as u32, success_moves);
                let failure_edge = pool.edge(failure_index as u32, failure_moves);

                Instruction::new(Op::Check, constraint.0, success_edge.0, failure_edge.0, 0)
            }

            mir::Terminator::VariantSwitch { .. } => {
                return Err(self.unsupported_instruction("variant.switch"));
            }

            mir::Terminator::Switch {
                value,
                cases,
                default,
            } => {
                let value = *value;
                let default_block = default.block;
                let default_arguments = self.function.target_arguments(default);
                let default_index = self.function.block_index_by_id[&default_block];
                let default_parameters = self.function.block_parameters[default_index].as_slice();
                let default_moves = pool.edge_moves(default_parameters, default_arguments)?;
                let default_edge = pool.edge(default_index as u32, default_moves);
                let cases = self.function.switch_cases(*cases);

                let (is_cell, width, is_signed) = switch_layout(self.operand_map().get(value));
                let switch_layout = switch_layout_field(width, is_signed);
                if is_cell
                    && let Some(table) = pool.switch_table_range(
                        self.function.tree,
                        &self.function.block_index_by_id,
                        &self.function.block_parameters,
                        cases,
                        default_index as u32,
                        default_moves,
                    )?
                {
                    Instruction::new(
                        Op::SwitchTable,
                        self.cell_offset(value)?,
                        table.0,
                        default_edge.0,
                        switch_layout,
                    )
                } else {
                    let cases = pool.switch_case_range(
                        self.function.tree,
                        &self.function.block_index_by_id,
                        &self.function.block_parameters,
                        cases,
                    )?;
                    let value_offset = if is_cell {
                        self.cell_offset(value)?
                    } else {
                        self.value_offset(value)?
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

                Instruction::new(Op::PanicValue, self.cell_offset(payload)?, 0, 0, 0)
            }

            mir::Terminator::UnwindResume => Instruction::new(Op::UnwindResume, 0, 0, 0, 0),

            mir::Terminator::Trap { .. } => Instruction::new(Op::Abort, 0, 0, 0, 0),

            mir::Terminator::Unreachable => Instruction::new(Op::Unreachable, 0, 0, 0, 0),

            mir::Terminator::Yield { value, .. } => {
                let value = *value;
                let frame_state = self
                    .function
                    .yield_frame_states
                    .get(&self.block_id())
                    .copied()
                    .ok_or_else(|| {
                        self.internal(format!(
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
                    self.value_offset(value)?,
                    self.function.program.type_id(value_type).0,
                    frame_state.0,
                    0,
                )
            }

            mir::Terminator::Invoke { call, .. } => self.lower_invoke(call, pool)?,
            mir::Terminator::TailCall { call } => self.lower_tail_call(call, pool)?,
        })
    }

    /// Return one checked cell integer layout.
    fn checked_integer(&self, value: mir::Value) -> LinkResult<(u8, bool)> {
        let value_type = self.value_type_for_value(value)?;
        let value_type = self.function.tree.repr_type(value_type);

        match self.function.tree.get(value_type) {
            mir::Type::Int { width, is_signed } if *width > 0 && *width <= u64::BITS as u16 => {
                Ok((*width as u8, *is_signed))
            }
            mir::Type::Usize => Ok((self.function.pointer_bytes() * 8, false)),
            _ => Err(self.type_mismatch("cell integer", format!("{value_type:?}"))),
        }
    }

    /// Return one checked type-id cell offset.
    fn type_id_cell_offset(&self, value: mir::Value) -> LinkResult<u32> {
        let value_type = self.value_type_for_value(value)?;
        let value_type = self.function.tree.repr_type(value_type);

        if matches!(self.function.tree.get(value_type), mir::Type::TypeId) {
            self.cell_offset(value)
        } else {
            Err(self.type_mismatch("type-id value", format!("{value_type:?}")))
        }
    }

    /// Select one concrete overflow check.
    fn overflow_check(
        &self,
        operator: mir::BinaryOperator,
        is_signed: bool,
        check: OverflowCheck,
    ) -> LinkResult<Check> {
        match (operator, is_signed) {
            (mir::BinaryOperator::Add, true) => {
                Ok(Check::overflow(CheckKind::OverflowAddInt, check))
            }
            (mir::BinaryOperator::Add, false) => {
                Ok(Check::overflow(CheckKind::OverflowAddUint, check))
            }
            (mir::BinaryOperator::Subtract, true) => {
                Ok(Check::overflow(CheckKind::OverflowSubInt, check))
            }
            (mir::BinaryOperator::Subtract, false) => {
                Ok(Check::overflow(CheckKind::OverflowSubUint, check))
            }
            (mir::BinaryOperator::Multiply, true) => {
                Ok(Check::overflow(CheckKind::OverflowMulInt, check))
            }
            (mir::BinaryOperator::Multiply, false) => {
                Ok(Check::overflow(CheckKind::OverflowMulUint, check))
            }
            (mir::BinaryOperator::SignedDivide | mir::BinaryOperator::SignedRemainder, true) => {
                Ok(Check::overflow(CheckKind::OverflowDivInt, check))
            }
            (
                mir::BinaryOperator::UnsignedDivide | mir::BinaryOperator::UnsignedRemainder,
                false,
            ) => Ok(Check::overflow(CheckKind::OverflowDivUint, check)),
            _ => Err(self.invalid_instruction("overflow check")),
        }
    }
}

/// Return the lowered switch integer layout.
fn switch_layout(layout: Option<Operand>) -> (bool, u16, bool) {
    match layout {
        Some(Operand::Int { width, signed }) if width <= u64::BITS as u16 => (true, width, signed),
        Some(Operand::Int { width, signed }) => (false, width, signed),
        _ => (false, 0, false),
    }
}

/// Pack one switch integer layout into an instruction operand.
fn switch_layout_field(width: u16, is_signed: bool) -> u32 {
    let sign = if is_signed { 1 << 16 } else { 0 };

    u32::from(width) | sign
}

/// Select one concrete bounds check.
fn bounds_check(index_signed: bool, length_signed: bool, check: BoundsCheck) -> Check {
    match (index_signed, length_signed) {
        (true, true) => Check::bounds(CheckKind::BoundsIntInt, check),
        (true, false) => Check::bounds(CheckKind::BoundsIntUint, check),
        (false, true) => Check::bounds(CheckKind::BoundsUintInt, check),
        (false, false) => Check::bounds(CheckKind::BoundsUintUint, check),
    }
}

/// Select one concrete div-zero check.
fn div_zero_check(is_signed: bool, divisor: u32) -> Check {
    if is_signed {
        return Check::div_zero(CheckKind::DivZeroInt, divisor);
    }

    Check::div_zero(CheckKind::DivZeroUint, divisor)
}

/// Select one concrete shift range check.
fn shift_range_check(is_signed: bool, check: ShiftRangeCheck) -> Check {
    if is_signed {
        return Check::shift(CheckKind::ShiftRangeInt, check);
    }

    Check::shift(CheckKind::ShiftRangeUint, check)
}

/// Select one concrete narrow check.
fn narrow_check(is_signed: bool, check: NarrowCheck) -> Check {
    if is_signed {
        return Check::narrow(CheckKind::NarrowInt, check);
    }

    Check::narrow(CheckKind::NarrowUint, check)
}
