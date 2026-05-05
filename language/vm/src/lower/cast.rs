use destack_mir as mir;

use crate::program::{
    FrameSelect, Instruction, Op, ValueLayout, WordLayout, value_layout_from_type,
    word_layout_from_type,
};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::pool::Pool;

const CAST_SIGN_BIT: u32 = 1 << 8;
const WORD_LAYOUT_HEAP_REFERENCE: u32 = 1;
const WORD_LAYOUT_SHARED_HEAP_REFERENCE: u32 = 2;
const WORD_LAYOUT_RAW_POINTER: u32 = 3;
const WORD_LAYOUT_SHARED_RAW_POINTER: u32 = 4;
const WORD_LAYOUT_STACK_POINTER: u32 = 5;
const WORD_LAYOUT_FRAME_POINTER: u32 = 6;
const WORD_LAYOUT_STATIC_POINTER: u32 = 7;
const WORD_LAYOUT_FUNCTION_POINTER: u32 = 8;

impl<'a> BlockLowerer<'a> {
    /// Lower one cast instruction.
    pub(super) fn lower_cast(
        &self,
        destination: mir::ValueReference,
        operator: mir::CastOperator,
        argument: mir::ValueReference,
        to_type: mir::TypeReference,
    ) -> Result<Instruction> {
        // require SSA values and the target type
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "cast destination".to_string(),
            })?;
        let argument = argument
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "cast argument".to_string(),
            })?;
        let to_type = to_type.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "cast destination type".to_string(),
        })?;
        let destination_type = self.value_type_for_value(destination)?;
        let argument_type = self.value_type_for_value(argument)?;
        let destination_is_word = self.layout_for_type(destination_type)?.is_word();
        let argument_is_word = self.layout_for_type(argument_type)?.is_word();

        // use the direct word path when both sides fit in one word
        if destination_is_word && argument_is_word {
            return Ok(Instruction::new(
                word_cast_op(self.tree, operator, to_type)?,
                word_offset(self, destination)?,
                word_offset(self, argument)?,
                word_cast_field(self.tree, operator, to_type)?,
                0,
            ));
        }

        // wide integer casts need explicit source and destination widths
        let (source_width, source_signed) = integer_layout(self.tree, argument_type)?;
        let (dest_width, dest_signed) = integer_layout(self.tree, to_type)?;
        let source_signed = wide_source_signed(operator, source_signed);

        // expand a word into frame bytes
        if argument_is_word {
            return Ok(Instruction::new(
                Op::CastWordToWideInt,
                value_offset(self, destination)?,
                word_offset(self, argument)?,
                wide_cast_flag_field(source_signed, false),
                wide_cast_layout_field(source_width, dest_width),
            ));
        }

        // collapse frame bytes into a word
        if destination_is_word {
            return Ok(Instruction::new(
                Op::CastWideIntToWord,
                word_offset(self, destination)?,
                value_offset(self, argument)?,
                wide_cast_flag_field(source_signed, dest_signed),
                wide_cast_layout_field(source_width, dest_width),
            ));
        }

        // transform wide integer frame bytes
        Ok(Instruction::new(
            Op::CastWideInt,
            value_offset(self, destination)?,
            value_offset(self, argument)?,
            wide_cast_flag_field(source_signed, false),
            wide_cast_layout_field(source_width, dest_width),
        ))
    }

    /// Lower one select instruction.
    pub(super) fn lower_select(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        condition: mir::ValueReference,
        then_value: mir::ValueReference,
        else_value: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select destination".to_string(),
            })?;
        let condition = condition
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select condition".to_string(),
            })?;
        let then_value = then_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select then value".to_string(),
            })?;
        let else_value = else_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "select else value".to_string(),
            })?;

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
        Ok(pool.instruction_with_side_record(
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
fn word_cast_op(
    tree: &mir::Tree,
    operator: mir::CastOperator,
    to_type: mir::LocalNodeId<mir::Type>,
) -> Result<Op> {
    match operator {
        mir::CastOperator::Bitcast => Ok(Op::CastBitcast),
        mir::CastOperator::Truncate => Ok(Op::CastTruncate),
        mir::CastOperator::ZeroExtend => Ok(Op::CastZeroExtend),
        mir::CastOperator::SignExtend => Ok(Op::CastSignExtend),
        mir::CastOperator::FloatToSignedInt => Ok(Op::CastFloatToSignedInt),
        mir::CastOperator::FloatToUnsignedInt => Ok(Op::CastFloatToUnsignedInt),
        mir::CastOperator::FloatToSignedIntSaturating => Ok(Op::CastFloatToSignedIntSaturating),
        mir::CastOperator::FloatToUnsignedIntSaturating => Ok(Op::CastFloatToUnsignedIntSaturating),
        mir::CastOperator::SignedIntToFloat => match value_layout_from_type(tree, to_type) {
            ValueLayout::Float { width: 32 } => Ok(Op::CastSignedIntToF32),
            ValueLayout::Float { width: 64 } => Ok(Op::CastSignedIntToF64),
            _ => Err(Error::InvalidCast),
        },
        mir::CastOperator::UnsignedIntToFloat => match value_layout_from_type(tree, to_type) {
            ValueLayout::Float { width: 32 } => Ok(Op::CastUnsignedIntToF32),
            ValueLayout::Float { width: 64 } => Ok(Op::CastUnsignedIntToF64),
            _ => Err(Error::InvalidCast),
        },
        mir::CastOperator::FloatTruncate => Ok(Op::CastFloatTruncate),
        mir::CastOperator::FloatExtend => Ok(Op::CastFloatExtend),
        mir::CastOperator::PointerToInt => Ok(Op::CastPointerToInt),
        mir::CastOperator::IntToPointer => Ok(Op::CastIntToPointer),
    }
}

/// Pack one pointer word cast target.
fn wide_cast_flag_field(source_signed: bool, dest_signed: bool) -> u32 {
    let source_signed = u32::from(source_signed) << 8;
    let dest_signed = u32::from(dest_signed) << 9;

    source_signed | dest_signed
}

/// Return one word cast shape field.
fn integer_cast_field(width: u16, signed: bool) -> Result<u32> {
    let width = u8::try_from(width).map_err(|_| Error::InvalidCast)?;
    let sign = if signed { CAST_SIGN_BIT } else { 0 };

    Ok(u32::from(width) | sign)
}

/// Return one wide integer cast layout field.
fn pointer_cast_field(layout: WordLayout) -> Result<u32> {
    match layout {
        WordLayout::HeapReference => Ok(WORD_LAYOUT_HEAP_REFERENCE),
        WordLayout::SharedHeapReference => Ok(WORD_LAYOUT_SHARED_HEAP_REFERENCE),
        WordLayout::RawPointer => Ok(WORD_LAYOUT_RAW_POINTER),
        WordLayout::SharedRawPointer => Ok(WORD_LAYOUT_SHARED_RAW_POINTER),
        WordLayout::StackPointer => Ok(WORD_LAYOUT_STACK_POINTER),
        WordLayout::FramePointer => Ok(WORD_LAYOUT_FRAME_POINTER),
        WordLayout::StaticPointer => Ok(WORD_LAYOUT_STATIC_POINTER),
        WordLayout::FunctionPointer => Ok(WORD_LAYOUT_FUNCTION_POINTER),
        _ => Err(Error::InvalidCast),
    }
}

/// Return one integer value layout.
fn word_cast_field(
    tree: &mir::Tree,
    operator: mir::CastOperator,
    to_type: mir::LocalNodeId<mir::Type>,
) -> Result<u32> {
    match operator {
        mir::CastOperator::Bitcast
        | mir::CastOperator::FloatTruncate
        | mir::CastOperator::FloatExtend => Ok(0),
        mir::CastOperator::Truncate
        | mir::CastOperator::ZeroExtend
        | mir::CastOperator::SignExtend
        | mir::CastOperator::FloatToSignedInt
        | mir::CastOperator::FloatToUnsignedInt
        | mir::CastOperator::FloatToSignedIntSaturating
        | mir::CastOperator::FloatToUnsignedIntSaturating
        | mir::CastOperator::PointerToInt => {
            let (width, signed) = integer_layout(tree, to_type)?;

            integer_cast_field(width, signed)
        }
        mir::CastOperator::SignedIntToFloat | mir::CastOperator::UnsignedIntToFloat => Ok(0),
        mir::CastOperator::IntToPointer => {
            let layout = word_layout_from_type(tree, to_type).ok_or(Error::InvalidCast)?;

            pointer_cast_field(layout)
        }
    }
}

/// Return whether one wide source should be sign extended.
fn wide_cast_layout_field(source_width: u16, dest_width: u16) -> u32 {
    u32::from(source_width) | (u32::from(dest_width) << 16)
}

/// Return one word cast operation.
fn integer_layout(tree: &mir::Tree, ty: mir::LocalNodeId<mir::Type>) -> Result<(u16, bool)> {
    match value_layout_from_type(tree, ty) {
        ValueLayout::Int { width, signed } => Ok((width, signed)),
        actual => Err(Error::TypeMismatch {
            expected: "integer cast value".to_string(),
            actual: format!("{actual:?}"),
        }),
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
