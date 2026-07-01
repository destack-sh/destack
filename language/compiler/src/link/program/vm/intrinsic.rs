use destack_mir as mir;

use crate::LinkResult;

use destack_program::vm::{Instruction, IntrinsicCall, IntrinsicDest, Op};

use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one intrinsic call.
    pub(super) fn lower_intrinsic(
        &self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ValueSlice,
        pool: &mut Pool<'_, '_>,
    ) -> LinkResult<Instruction> {
        // collect argument values and operand shapes
        let argument_values = self.function.tree.get_values(arguments);
        let mut operands = Vec::with_capacity(argument_values.len());
        for argument in argument_values {
            let operand = self
                .operand_map()
                .get(*argument)
                .ok_or_else(|| self.invalid_instruction("intrinsic argument operand"))?;
            let operand = operand
                .intrinsic()
                .ok_or_else(|| self.invalid_instruction("intrinsic argument shape"))?;

            operands.push(operand);
        }

        // intern the argument range
        let arguments = pool.argument_range(argument_values)?;

        // resolve the optional destination
        let dest = match destination {
            Some(destination) => {
                let slot = self
                    .function
                    .frame_value_slot(destination.0)
                    .ok_or_else(|| self.invalid_instruction("intrinsic destination slot"))?;

                if self.function.slot_is_cell(slot) {
                    IntrinsicDest::Cell(self.cell_offset(destination)?)
                } else {
                    IntrinsicDest::Frame(self.move_slot(destination)?)
                }
            }
            None => IntrinsicDest::None,
        };

        // reject compile-time intrinsics before building the side record
        self.validate_runtime_intrinsic(intrinsic)?;
        let intrinsic = IntrinsicCall::new(intrinsic, dest, arguments, &operands)
            .ok_or_else(|| self.unsupported_instruction("intrinsic with more than 16 arguments"))?;

        Ok(pool.instruction_with_side(Op::Intrinsic, intrinsic))
    }

    /// Validate one runtime intrinsic.
    fn validate_runtime_intrinsic(&self, intrinsic: mir::Intrinsic) -> LinkResult<()> {
        match intrinsic {
            mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => Err(self
                .unsupported_instruction(format!(
                    "intrinsic.{} (compile-time only)",
                    intrinsic.to_str()
                ))),
            _ => Ok(()),
        }
    }
}
