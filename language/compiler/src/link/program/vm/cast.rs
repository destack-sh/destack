use destack_mir as mir;

use crate::LinkResult;

use destack_program::vm::{
    AggregateSelect, FloatCast, FloatToIntCast, Instruction, IntToFloatCast, IntegerCast, Op,
    PointerCast, TensorViewCast, WideIntegerCast,
};

use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one cast instruction.
    pub(super) fn lower_cast(
        &self,
        destination: mir::Value,
        operator: mir::CastOperator,
        argument: mir::Value,
        to_type: mir::TypeId,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        // require SSA values and the target type
        let destination_type = self.value_type_for_value(destination)?;
        let argument_type = self.value_type_for_value(argument)?;

        // build tensor view descriptors from dense pointer casts
        if operator == mir::CastOperator::Bitcast
            && matches!(
                self.function.tree.get(to_type),
                mir::Type::TensorView { .. }
            )
        {
            let view_layout = self.tensor_layout(pool, to_type)?;

            return Ok(pool.instruction_with_side(
                Op::CastTensorView,
                TensorViewCast {
                    dest_offset: self.value_offset(destination)?,
                    pointer_offset: self.cell_offset(argument)?,
                    view_layout,
                },
            ));
        }

        let destination_is_cell = self.layout_for_type(destination_type)?.is_cell();
        let argument_is_cell = self.layout_for_type(argument_type)?.is_cell();
        let operand_lowerer = self.function.operand_lowerer();
        let type_linker = self.function.type_linker();

        // use the direct cell path when both sides fit in one cell
        if destination_is_cell && argument_is_cell {
            return Ok(Instruction::new(
                self.cell_cast_op(operator)?,
                self.cell_offset(destination)?,
                self.cell_offset(argument)?,
                self.encode_cell_cast(
                    &type_linker,
                    &operand_lowerer,
                    operator,
                    argument_type,
                    to_type,
                )?,
                0,
            ));
        }

        // wide integer casts need explicit source and destination widths
        let (source_width, source_signed) = operand_lowerer
            .integer_layout(argument_type)
            .ok_or_else(|| self.invalid_cast("wide integer source"))?;
        let (dest_width, dest_signed) = operand_lowerer
            .integer_layout(to_type)
            .ok_or_else(|| self.invalid_cast("wide integer destination"))?;
        let source_signed = wide_source_signed(operator, source_signed);
        let cast = WideIntegerCast::new(source_width, dest_width, source_signed, false);

        // expand a cell into frame bytes
        if argument_is_cell {
            return Ok(Instruction::new(
                Op::CastCellToWideInt,
                self.value_offset(destination)?,
                self.cell_offset(argument)?,
                cast.flags(),
                cast.widths(),
            ));
        }

        // collapse frame bytes into a cell
        if destination_is_cell {
            let cast = WideIntegerCast::new(source_width, dest_width, source_signed, dest_signed);

            return Ok(Instruction::new(
                Op::CastWideIntToCell,
                self.cell_offset(destination)?,
                self.value_offset(argument)?,
                cast.flags(),
                cast.widths(),
            ));
        }

        // transform wide integer frame bytes
        Ok(Instruction::new(
            Op::CastWideInt,
            self.value_offset(destination)?,
            self.value_offset(argument)?,
            cast.flags(),
            cast.widths(),
        ))
    }

    /// Lower one select instruction.
    pub(super) fn lower_select(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        condition: mir::Value,
        then_value: mir::Value,
        else_value: mir::Value,
    ) -> LinkResult<Instruction> {
        // require SSA values

        // select cell values without touching frame bytes
        let destination_type = self.value_type_for_value(destination)?;
        let destination_layout = self.layout_for_type(destination_type)?;
        if destination_layout.is_cell() {
            return Ok(Instruction::new(
                Op::SelectCell,
                self.cell_offset(destination)?,
                self.cell_offset(condition)?,
                self.cell_offset(then_value)?,
                self.cell_offset(else_value)?,
            ));
        }

        // select frame-backed values by copying their frame slot
        Ok(pool.instruction_with_side(
            Op::SelectAggregate,
            AggregateSelect {
                destination_offset: self.value_offset(destination)?,
                condition_offset: self.cell_offset(condition)?,
                then_offset: self.value_offset(then_value)?,
                else_offset: self.value_offset(else_value)?,
                byte_len: destination_layout.byte_len,
            },
        ))
    }

    /// Pack one cell integer cast target.
    fn cell_cast_op(&self, operator: mir::CastOperator) -> LinkResult<Op> {
        match operator {
            mir::CastOperator::Bitcast => Ok(Op::CastBitcast),
            mir::CastOperator::Truncate => Ok(Op::CastTruncate),
            mir::CastOperator::ZeroExtend => Ok(Op::CastZeroExtend),
            mir::CastOperator::SignExtend => Ok(Op::CastSignExtend),
            mir::CastOperator::FloatToSignedInt => Ok(Op::CastFloatToSignedInt),
            mir::CastOperator::FloatToUnsignedInt => Ok(Op::CastFloatToUnsignedInt),
            mir::CastOperator::FloatToSignedIntSaturating => Ok(Op::CastFloatToSignedIntSaturating),
            mir::CastOperator::FloatToUnsignedIntSaturating => {
                Ok(Op::CastFloatToUnsignedIntSaturating)
            }
            mir::CastOperator::SignedIntToFloat => Ok(Op::CastSignedIntToFloat),
            mir::CastOperator::UnsignedIntToFloat => Ok(Op::CastUnsignedIntToFloat),
            mir::CastOperator::FloatTruncate
            | mir::CastOperator::FloatExtend
            | mir::CastOperator::FloatConvert => Ok(Op::CastFloatConvert),
            mir::CastOperator::PointerToInt => Ok(Op::CastPointerToInt),
            mir::CastOperator::IntToPointer => Ok(Op::CastIntToPointer),
        }
    }

    /// Encode one cell cast instruction field.
    fn encode_cell_cast(
        &self,
        type_linker: &super::super::TypeLinker<'_>,
        operand_lowerer: &super::value::OperandLowerer<'_>,
        operator: mir::CastOperator,
        from_type: mir::LocalNodeId<mir::Type>,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<u32> {
        match operator {
            mir::CastOperator::Bitcast => Ok(0),
            mir::CastOperator::Truncate
            | mir::CastOperator::ZeroExtend
            | mir::CastOperator::SignExtend
            | mir::CastOperator::PointerToInt => {
                let (width, signed) = operand_lowerer
                    .integer_layout(to_type)
                    .ok_or_else(|| self.invalid_cast("integer target"))?;

                IntegerCast::new(width, signed)
                    .map(|cast| cast.field())
                    .map_err(|_| self.invalid_cast("integer cast"))
            }
            mir::CastOperator::FloatToSignedInt
            | mir::CastOperator::FloatToUnsignedInt
            | mir::CastOperator::FloatToSignedIntSaturating
            | mir::CastOperator::FloatToUnsignedIntSaturating => {
                let source = type_linker
                    .cell_layout(from_type)
                    .ok_or_else(|| self.invalid_cast("float cast source"))?;
                let (width, _) = operand_lowerer
                    .integer_layout(to_type)
                    .ok_or_else(|| self.invalid_cast("float cast target"))?;

                FloatToIntCast::new(source, width)
                    .map(|cast| cast.field())
                    .map_err(|_| self.invalid_cast("float to integer cast"))
            }
            mir::CastOperator::SignedIntToFloat | mir::CastOperator::UnsignedIntToFloat => {
                let destination = type_linker
                    .cell_layout(to_type)
                    .ok_or_else(|| self.invalid_cast("integer cast target"))?;

                IntToFloatCast::new(destination)
                    .map(|cast| cast.field())
                    .map_err(|_| self.invalid_cast("integer to float cast"))
            }
            mir::CastOperator::FloatTruncate
            | mir::CastOperator::FloatExtend
            | mir::CastOperator::FloatConvert => {
                let source = type_linker
                    .cell_layout(from_type)
                    .ok_or_else(|| self.invalid_cast("float cast source"))?;
                let destination = type_linker
                    .cell_layout(to_type)
                    .ok_or_else(|| self.invalid_cast("float cast target"))?;

                FloatCast::new(source, destination)
                    .map(|cast| cast.field())
                    .map_err(|_| self.invalid_cast("float cast"))
            }
            mir::CastOperator::IntToPointer => {
                let layout = type_linker
                    .cell_layout(to_type)
                    .ok_or_else(|| self.invalid_cast("pointer cast target"))?;

                PointerCast::new(layout)
                    .map(|cast| cast.field())
                    .map_err(|_| self.invalid_cast("integer to pointer cast"))
            }
        }
    }
}

/// Return one wide integer cast flag field.
fn wide_source_signed(operator: mir::CastOperator, source_signed: bool) -> bool {
    match operator {
        mir::CastOperator::SignExtend => true,
        mir::CastOperator::ZeroExtend => false,
        _ => source_signed,
    }
}
