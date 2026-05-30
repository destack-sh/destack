use destack_mir as mir;

use crate::program::{
    FloatCast, FloatToIntCast, FrameSelect, Instruction, IntToFloatCast, IntegerCast, Op,
    PointerCast, TensorViewCast, ValueLayout, WideIntegerCast, value_layout_from_type,
    word_layout_from_type,
};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one cast instruction.
    pub(super) fn lower_cast(
        &self,
        destination: mir::ValueReference,
        operator: mir::CastOperator,
        argument: mir::ValueReference,
        to_type: mir::TypeReference,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // require SSA values and the target type
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("cast destination"))?;
        let argument = argument
            .value()
            .ok_or_else(|| Error::invalid_program("cast argument"))?;
        let to_type = to_type
            .ty()
            .ok_or_else(|| Error::invalid_program("cast destination type"))?;
        let destination_type = self.value_type_for_value(destination)?;
        let argument_type = self.value_type_for_value(argument)?;

        // build tensor view descriptors from dense pointer casts
        if operator == mir::CastOperator::Bitcast
            && matches!(self.tree.get(to_type), mir::Type::TensorView { .. })
        {
            let view_layout = self.tensor_layout(pool, to_type)?;

            return Ok(pool.instruction_with_side(
                Op::CastTensorView,
                TensorViewCast {
                    dest_offset: value_offset(self, destination)?,
                    pointer_offset: word_offset(self, argument)?,
                    view_layout,
                },
            ));
        }

        let destination_is_word = self.layout_for_type(destination_type)?.is_word();
        let argument_is_word = self.layout_for_type(argument_type)?.is_word();

        // use the direct word path when both sides fit in one word
        if destination_is_word && argument_is_word {
            return Ok(Instruction::new(
                word_cast_op(operator)?,
                word_offset(self, destination)?,
                word_offset(self, argument)?,
                word_cast_field(self.tree, operator, argument_type, to_type)?,
                0,
            ));
        }

        // wide integer casts need explicit source and destination widths
        let (source_width, source_signed) = integer_layout(self.tree, argument_type)?;
        let (dest_width, dest_signed) = integer_layout(self.tree, to_type)?;
        let source_signed = wide_source_signed(operator, source_signed);
        let cast = WideIntegerCast::new(source_width, dest_width, source_signed, false);

        // expand a word into frame bytes
        if argument_is_word {
            return Ok(Instruction::new(
                Op::CastWordToWideInt,
                value_offset(self, destination)?,
                word_offset(self, argument)?,
                cast.flags(),
                cast.widths(),
            ));
        }

        // collapse frame bytes into a word
        if destination_is_word {
            let cast = WideIntegerCast::new(source_width, dest_width, source_signed, dest_signed);

            return Ok(Instruction::new(
                Op::CastWideIntToWord,
                word_offset(self, destination)?,
                value_offset(self, argument)?,
                cast.flags(),
                cast.widths(),
            ));
        }

        // transform wide integer frame bytes
        Ok(Instruction::new(
            Op::CastWideInt,
            value_offset(self, destination)?,
            value_offset(self, argument)?,
            cast.flags(),
            cast.widths(),
        ))
    }

    /// Lower one select instruction.
    pub(super) fn lower_select(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        condition: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("select destination"))?;
        let condition = condition
            .value()
            .ok_or_else(|| Error::invalid_program("select condition"))?;
        let then_value = then_value
            .value()
            .ok_or_else(|| Error::invalid_program("select then value"))?;
        let else_value = else_value
            .value()
            .ok_or_else(|| Error::invalid_program("select else value"))?;

        // select word values without touching frame bytes
        let destination_type = self.value_type_for_value(destination)?;
        let destination_layout = self.layout_for_type(destination_type)?;
        if destination_layout.is_word() {
            return Ok(Instruction::new(
                Op::SelectWord,
                word_offset(self, destination)?,
                word_offset(self, condition)?,
                word_offset(self, then_value)?,
                word_offset(self, else_value)?,
            ));
        }

        // select frame-backed values by copying their frame slot
        Ok(pool.instruction_with_side(
            Op::SelectFrame,
            FrameSelect {
                destination_offset: value_offset(self, destination)?,
                condition_offset: word_offset(self, condition)?,
                then_offset: value_offset(self, then_value)?,
                else_offset: value_offset(self, else_value)?,
                byte_len: destination_layout.byte_len,
            },
        ))
    }
}

/// Pack one word integer cast target.
fn word_cast_op(operator: mir::CastOperator) -> Result<Op> {
    match operator {
        mir::CastOperator::Bitcast => Ok(Op::CastBitcast),
        mir::CastOperator::Truncate => Ok(Op::CastTruncate),
        mir::CastOperator::ZeroExtend => Ok(Op::CastZeroExtend),
        mir::CastOperator::SignExtend => Ok(Op::CastSignExtend),
        mir::CastOperator::FloatToSignedInt => Ok(Op::CastFloatToSignedInt),
        mir::CastOperator::FloatToUnsignedInt => Ok(Op::CastFloatToUnsignedInt),
        mir::CastOperator::FloatToSignedIntSaturating => Ok(Op::CastFloatToSignedIntSaturating),
        mir::CastOperator::FloatToUnsignedIntSaturating => Ok(Op::CastFloatToUnsignedIntSaturating),
        mir::CastOperator::SignedIntToFloat => Ok(Op::CastSignedIntToFloat),
        mir::CastOperator::UnsignedIntToFloat => Ok(Op::CastUnsignedIntToFloat),
        mir::CastOperator::FloatTruncate
        | mir::CastOperator::FloatExtend
        | mir::CastOperator::FloatConvert => Ok(Op::CastFloatConvert),
        mir::CastOperator::PointerToInt => Ok(Op::CastPointerToInt),
        mir::CastOperator::IntToPointer => Ok(Op::CastIntToPointer),
    }
}

/// Return one integer value layout.
fn word_cast_field(
    tree: &mir::Tree,
    operator: mir::CastOperator,
    from_type: mir::LocalNodeId<mir::Type>,
    to_type: mir::LocalNodeId<mir::Type>,
) -> Result<u32> {
    match operator {
        mir::CastOperator::Bitcast => Ok(0),
        mir::CastOperator::Truncate
        | mir::CastOperator::ZeroExtend
        | mir::CastOperator::SignExtend
        | mir::CastOperator::PointerToInt => {
            let (width, signed) = integer_layout(tree, to_type)?;

            Ok(IntegerCast::new(width, signed)?.field())
        }
        mir::CastOperator::FloatToSignedInt
        | mir::CastOperator::FloatToUnsignedInt
        | mir::CastOperator::FloatToSignedIntSaturating
        | mir::CastOperator::FloatToUnsignedIntSaturating => {
            let source = word_layout_from_type(tree, from_type).ok_or(Error::invalid_cast())?;
            let (width, _) = integer_layout(tree, to_type)?;

            Ok(FloatToIntCast::new(source, width)?.field())
        }
        mir::CastOperator::SignedIntToFloat | mir::CastOperator::UnsignedIntToFloat => {
            let destination = word_layout_from_type(tree, to_type).ok_or(Error::invalid_cast())?;

            Ok(IntToFloatCast::new(destination)?.field())
        }
        mir::CastOperator::FloatTruncate
        | mir::CastOperator::FloatExtend
        | mir::CastOperator::FloatConvert => {
            let source = word_layout_from_type(tree, from_type).ok_or(Error::invalid_cast())?;
            let destination = word_layout_from_type(tree, to_type).ok_or(Error::invalid_cast())?;

            Ok(FloatCast::new(source, destination)?.field())
        }
        mir::CastOperator::IntToPointer => {
            let layout = word_layout_from_type(tree, to_type).ok_or(Error::invalid_cast())?;

            Ok(PointerCast::new(layout)?.field())
        }
    }
}

/// Return one word cast operation.
fn integer_layout(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<(u16, bool)> {
    match value_layout_from_type(tree, ty) {
        ValueLayout::Int { width, signed } => Ok((width, signed)),
        actual => Err(Error::type_mismatch(
            "integer cast value",
            format!("{actual:?}"),
        )),
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
