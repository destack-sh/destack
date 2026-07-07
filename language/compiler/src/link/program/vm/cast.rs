use destack_mir as mir;

use crate::LinkResult;

use destack_program::vm::{
    AggregateSelect, FloatCast, FloatToIntCast, Instruction, IntToFloatCast, IntegerCast, Op,
    PointerCast, TensorViewCast, WideIntegerCast,
};

use super::lower::BlockLowerer;
use super::pool::Pool;

/// One VM cell cast encoding.
enum CellCast {
    /// Cast with one packed instruction field.
    Field {
        /// The VM operation.
        op: Op,
        /// The packed instruction field.
        field: u32,
    },
    /// Cast with source and destination integer metadata.
    WideInteger {
        /// The VM operation.
        op: Op,
        /// The packed integer cast metadata.
        cast: WideIntegerCast,
    },
}

impl CellCast {
    /// Build one field encoded VM cast.
    fn field(op: Op, field: u32) -> Self {
        Self::Field { op, field }
    }

    /// Build one wide integer encoded VM cast.
    fn wide_integer(op: Op, cast: WideIntegerCast) -> Self {
        Self::WideInteger { op, cast }
    }

    /// Build one VM instruction.
    fn instruction(self, destination: u32, argument: u32) -> Instruction {
        match self {
            CellCast::Field { op, field } => Instruction::new(op, destination, argument, field, 0),
            CellCast::WideInteger { op, cast } => {
                Instruction::new(op, destination, argument, cast.flags(), cast.widths())
            }
        }
    }
}

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
            let cast = self.cell_cast(
                &type_linker,
                &operand_lowerer,
                operator,
                argument_type,
                to_type,
            )?;
            let destination_offset = self.cell_offset(destination)?;
            let argument_offset = self.cell_offset(argument)?;

            return Ok(cast.instruction(destination_offset, argument_offset));
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
                byte_len: destination_layout.byte_len(),
            },
        ))
    }

    /// Build one cell cast encoding.
    fn cell_cast(
        &self,
        type_linker: &super::super::TypeLinker<'_>,
        operand_lowerer: &super::value::OperandLowerer<'_>,
        operator: mir::CastOperator,
        from_type: mir::LocalNodeId<mir::Type>,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<CellCast> {
        match operator {
            mir::CastOperator::Bitcast => Ok(CellCast::field(Op::CastBitcast, 0)),
            mir::CastOperator::Saturate => {
                self.saturating_integer_cell_cast(operand_lowerer, from_type, to_type)
            }
            mir::CastOperator::Truncate => {
                self.integer_cell_cast(operand_lowerer, to_type, Op::CastTruncate)
            }
            mir::CastOperator::ZeroExtend => {
                self.integer_cell_cast(operand_lowerer, to_type, Op::CastZeroExtend)
            }
            mir::CastOperator::SignExtend => {
                self.integer_cell_cast(operand_lowerer, to_type, Op::CastSignExtend)
            }
            mir::CastOperator::PointerToInt => {
                self.integer_cell_cast(operand_lowerer, to_type, Op::CastPointerToInt)
            }
            mir::CastOperator::FloatToSignedInt => self.float_to_int_cell_cast(
                type_linker,
                operand_lowerer,
                from_type,
                to_type,
                Op::CastFloatToSignedInt,
            ),
            mir::CastOperator::FloatToUnsignedInt => self.float_to_int_cell_cast(
                type_linker,
                operand_lowerer,
                from_type,
                to_type,
                Op::CastFloatToUnsignedInt,
            ),
            mir::CastOperator::FloatToSignedIntSaturating => self.float_to_int_cell_cast(
                type_linker,
                operand_lowerer,
                from_type,
                to_type,
                Op::CastFloatToSignedIntSaturating,
            ),
            mir::CastOperator::FloatToUnsignedIntSaturating => self.float_to_int_cell_cast(
                type_linker,
                operand_lowerer,
                from_type,
                to_type,
                Op::CastFloatToUnsignedIntSaturating,
            ),
            mir::CastOperator::SignedIntToFloat => {
                self.int_to_float_cell_cast(type_linker, to_type, Op::CastSignedIntToFloat)
            }
            mir::CastOperator::UnsignedIntToFloat => {
                self.int_to_float_cell_cast(type_linker, to_type, Op::CastUnsignedIntToFloat)
            }
            mir::CastOperator::FloatTruncate
            | mir::CastOperator::FloatExtend
            | mir::CastOperator::FloatConvert => {
                self.float_cell_cast(type_linker, from_type, to_type)
            }
            mir::CastOperator::IntToPointer => self.int_to_pointer_cell_cast(type_linker, to_type),
        }
    }

    /// Build one destination-shaped integer cell cast.
    fn integer_cell_cast(
        &self,
        operand_lowerer: &super::value::OperandLowerer<'_>,
        to_type: mir::LocalNodeId<mir::Type>,
        op: Op,
    ) -> LinkResult<CellCast> {
        let (width, signed) = operand_lowerer
            .integer_layout(to_type)
            .ok_or_else(|| self.invalid_cast("integer target"))?;

        IntegerCast::new(width, signed)
            .map(|cast| CellCast::field(op, cast.field()))
            .map_err(|_| self.invalid_cast("integer cast"))
    }

    /// Build one saturating integer cell cast.
    fn saturating_integer_cell_cast(
        &self,
        operand_lowerer: &super::value::OperandLowerer<'_>,
        from_type: mir::LocalNodeId<mir::Type>,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<CellCast> {
        let (source_width, source_signed) = operand_lowerer
            .integer_layout(from_type)
            .ok_or_else(|| self.invalid_cast("integer saturation source"))?;
        let (dest_width, dest_signed) = operand_lowerer
            .integer_layout(to_type)
            .ok_or_else(|| self.invalid_cast("integer saturation destination"))?;
        let cast = WideIntegerCast::new(source_width, dest_width, source_signed, dest_signed);

        Ok(CellCast::wide_integer(Op::CastSaturateInt, cast))
    }

    /// Build one float to integer cell cast.
    fn float_to_int_cell_cast(
        &self,
        type_linker: &super::super::TypeLinker<'_>,
        operand_lowerer: &super::value::OperandLowerer<'_>,
        from_type: mir::LocalNodeId<mir::Type>,
        to_type: mir::LocalNodeId<mir::Type>,
        op: Op,
    ) -> LinkResult<CellCast> {
        let source = type_linker
            .cell_layout(from_type)
            .ok_or_else(|| self.invalid_cast("float cast source"))?;
        let (width, _) = operand_lowerer
            .integer_layout(to_type)
            .ok_or_else(|| self.invalid_cast("float cast target"))?;

        FloatToIntCast::new(source, width)
            .map(|cast| CellCast::field(op, cast.field()))
            .map_err(|_| self.invalid_cast("float to integer cast"))
    }

    /// Build one integer to float cell cast.
    fn int_to_float_cell_cast(
        &self,
        type_linker: &super::super::TypeLinker<'_>,
        to_type: mir::LocalNodeId<mir::Type>,
        op: Op,
    ) -> LinkResult<CellCast> {
        let destination = type_linker
            .cell_layout(to_type)
            .ok_or_else(|| self.invalid_cast("integer cast target"))?;

        IntToFloatCast::new(destination)
            .map(|cast| CellCast::field(op, cast.field()))
            .map_err(|_| self.invalid_cast("integer to float cast"))
    }

    /// Build one float cell cast.
    fn float_cell_cast(
        &self,
        type_linker: &super::super::TypeLinker<'_>,
        from_type: mir::LocalNodeId<mir::Type>,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<CellCast> {
        let source = type_linker
            .cell_layout(from_type)
            .ok_or_else(|| self.invalid_cast("float cast source"))?;
        let destination = type_linker
            .cell_layout(to_type)
            .ok_or_else(|| self.invalid_cast("float cast target"))?;

        FloatCast::new(source, destination)
            .map(|cast| CellCast::field(Op::CastFloatConvert, cast.field()))
            .map_err(|_| self.invalid_cast("float cast"))
    }

    /// Build one integer to pointer cell cast.
    fn int_to_pointer_cell_cast(
        &self,
        type_linker: &super::super::TypeLinker<'_>,
        to_type: mir::LocalNodeId<mir::Type>,
    ) -> LinkResult<CellCast> {
        let layout = type_linker
            .cell_layout(to_type)
            .ok_or_else(|| self.invalid_cast("pointer cast target"))?;

        PointerCast::new(layout)
            .map(|cast| CellCast::field(Op::CastIntToPointer, cast.field()))
            .map_err(|_| self.invalid_cast("integer to pointer cast"))
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
