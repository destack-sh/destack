use destack_mir as mir;

use crate::program::{Instruction, Intrinsic, IntrinsicDest, Op};
use crate::{Error, Result};

use super::frame::word_offset;
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one intrinsic call.
    pub(super) fn lower_intrinsic(
        &self,
        destination: Option<mir::ValueReference>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ArgumentSlice,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        // collect argument values and static layouts
        let argument_values = self.tree.get_arguments(arguments);
        let mut layouts = Vec::with_capacity(argument_values.len());
        for argument in argument_values {
            let argument = argument
                .value()
                .ok_or_else(|| Error::MissingRepresentation {
                    context: "intrinsic argument".to_string(),
                })?;
            let layout = self
                .value_layout_map()
                .get(argument)
                .ok_or(Error::InvalidInstruction)?;

            layouts.push(layout);
        }

        // intern the argument range
        let arguments = pool.argument_reference_range(argument_values, "intrinsic argument")?;

        // resolve the optional destination
        let dest = match destination {
            Some(destination) => {
                let destination =
                    destination
                        .value()
                        .ok_or_else(|| Error::MissingRepresentation {
                            context: "intrinsic destination".to_string(),
                        })?;
                let slot = self
                    .frame_layout
                    .value(destination.0)
                    .ok_or(Error::InvalidInstruction)?;

                if slot.is_word {
                    IntrinsicDest::Word(word_offset(self, destination)?)
                } else {
                    IntrinsicDest::Frame(destination)
                }
            }
            None => IntrinsicDest::None,
        };

        // encode the intrinsic choice in the opcode
        let op = intrinsic_op(intrinsic)?;
        let intrinsic = Intrinsic::new(dest, arguments, &layouts).ok_or_else(|| {
            Error::UnsupportedInstruction {
                name: "intrinsic with more than 16 arguments".to_string(),
            }
        })?;

        Ok(pool.instruction_with_side_record(op, intrinsic))
    }
}

/// Return the VM opcode for one runtime intrinsic.
fn intrinsic_op(intrinsic: mir::Intrinsic) -> Result<Op> {
    match intrinsic {
        mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => {
            Err(Error::UnsupportedInstruction {
                name: format!("intrinsic.{} (compile-time only)", intrinsic.to_str()),
            })
        }
        mir::Intrinsic::LeadingZeroCount => Ok(Op::IntrinsicLeadingZeroCount),
        mir::Intrinsic::TrailingZeroCount => Ok(Op::IntrinsicTrailingZeroCount),
        mir::Intrinsic::PopulationCount => Ok(Op::IntrinsicPopulationCount),
        mir::Intrinsic::ByteSwap => Ok(Op::IntrinsicByteSwap),
        mir::Intrinsic::BitReverse => Ok(Op::IntrinsicBitReverse),
        mir::Intrinsic::RotateLeft => Ok(Op::IntrinsicRotateLeft),
        mir::Intrinsic::RotateRight => Ok(Op::IntrinsicRotateRight),
        mir::Intrinsic::AddOverflow => Ok(Op::IntrinsicAddOverflow),
        mir::Intrinsic::SubOverflow => Ok(Op::IntrinsicSubOverflow),
        mir::Intrinsic::MulOverflow => Ok(Op::IntrinsicMulOverflow),
        mir::Intrinsic::AddUnchecked => Ok(Op::IntrinsicAddUnchecked),
        mir::Intrinsic::SubUnchecked => Ok(Op::IntrinsicSubUnchecked),
        mir::Intrinsic::MulUnchecked => Ok(Op::IntrinsicMulUnchecked),
        mir::Intrinsic::DivUnchecked => Ok(Op::IntrinsicDivUnchecked),
        mir::Intrinsic::RemUnchecked => Ok(Op::IntrinsicRemUnchecked),
        mir::Intrinsic::ShlUnchecked => Ok(Op::IntrinsicShlUnchecked),
        mir::Intrinsic::ShrUnchecked => Ok(Op::IntrinsicShrUnchecked),
        mir::Intrinsic::SatAdd => Ok(Op::IntrinsicSatAdd),
        mir::Intrinsic::SatSub => Ok(Op::IntrinsicSatSub),
        mir::Intrinsic::Memcpy => Ok(Op::IntrinsicMemcpy),
        mir::Intrinsic::Memmove => Ok(Op::IntrinsicMemmove),
        mir::Intrinsic::Memset => Ok(Op::IntrinsicMemset),
        mir::Intrinsic::Memcmp => Ok(Op::IntrinsicMemcmp),
        mir::Intrinsic::PrefetchRead => Ok(Op::IntrinsicPrefetchRead),
        mir::Intrinsic::PrefetchWrite => Ok(Op::IntrinsicPrefetchWrite),
        mir::Intrinsic::Transmute => Ok(Op::IntrinsicTransmute),
        mir::Intrinsic::AddressSpaceCast => Ok(Op::IntrinsicAddressSpaceCast),
        mir::Intrinsic::PointerOffsetFrom => Ok(Op::IntrinsicPointerOffsetFrom),
        mir::Intrinsic::RawEq => Ok(Op::IntrinsicRawEq),
        mir::Intrinsic::Sqrt => Ok(Op::IntrinsicSqrt),
        mir::Intrinsic::Abs => Ok(Op::IntrinsicAbs),
        mir::Intrinsic::Fma => Ok(Op::IntrinsicFma),
        mir::Intrinsic::CopySign => Ok(Op::IntrinsicCopySign),
        mir::Intrinsic::Min => Ok(Op::IntrinsicMin),
        mir::Intrinsic::Max => Ok(Op::IntrinsicMax),
        mir::Intrinsic::Sin => Ok(Op::IntrinsicSin),
        mir::Intrinsic::Cos => Ok(Op::IntrinsicCos),
        mir::Intrinsic::Tan => Ok(Op::IntrinsicTan),
        mir::Intrinsic::Asin => Ok(Op::IntrinsicAsin),
        mir::Intrinsic::Acos => Ok(Op::IntrinsicAcos),
        mir::Intrinsic::Atan => Ok(Op::IntrinsicAtan),
        mir::Intrinsic::Atan2 => Ok(Op::IntrinsicAtan2),
        mir::Intrinsic::Exp => Ok(Op::IntrinsicExp),
        mir::Intrinsic::Exp2 => Ok(Op::IntrinsicExp2),
        mir::Intrinsic::Log => Ok(Op::IntrinsicLog),
        mir::Intrinsic::Log2 => Ok(Op::IntrinsicLog2),
        mir::Intrinsic::Log10 => Ok(Op::IntrinsicLog10),
        mir::Intrinsic::Pow => Ok(Op::IntrinsicPow),
        mir::Intrinsic::Floor => Ok(Op::IntrinsicFloor),
        mir::Intrinsic::Ceil => Ok(Op::IntrinsicCeil),
        mir::Intrinsic::Trunc => Ok(Op::IntrinsicTrunc),
        mir::Intrinsic::Round => Ok(Op::IntrinsicRound),
        mir::Intrinsic::Breakpoint => Ok(Op::IntrinsicBreakpoint),
        mir::Intrinsic::ReturnAddress => Ok(Op::IntrinsicReturnAddress),
        mir::Intrinsic::FrameAddress => Ok(Op::IntrinsicFrameAddress),
        mir::Intrinsic::Expect => Ok(Op::IntrinsicExpect),
        mir::Intrinsic::BlackBox => Ok(Op::IntrinsicBlackBox),
    }
}
