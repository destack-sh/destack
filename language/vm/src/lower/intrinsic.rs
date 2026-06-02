use destack_mir as mir;

use crate::program::{Instruction, Intrinsic, IntrinsicDest, Op};
use crate::{Error, Result};

use super::frame::cell_offset;
use super::lower::BlockLowerer;
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one intrinsic call.
    pub(super) fn lower_intrinsic(
        &self,
        destination: Option<mir::ValueReference>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ArgumentSlice,
        pool: &mut Pool<'_, '_>,
    ) -> Result<Instruction> {
        // collect argument values and static layouts
        let argument_values = self.tree.get_arguments(arguments);
        let mut layouts = Vec::with_capacity(argument_values.len());
        for argument in argument_values {
            let argument = argument
                .value()
                .ok_or_else(|| Error::invalid_program("intrinsic argument"))?;
            let layout = self
                .value_shape_map()
                .get(argument)
                .ok_or(Error::invalid_instruction())?;

            layouts.push(layout);
        }

        // intern the argument range
        let arguments = pool.argument_reference_range(argument_values, "intrinsic argument")?;

        // resolve the optional destination
        let dest = match destination {
            Some(destination) => {
                let destination = destination
                    .value()
                    .ok_or_else(|| Error::invalid_program("intrinsic destination"))?;
                let slot = self
                    .frame_layout
                    .value(destination.0)
                    .ok_or(Error::invalid_instruction())?;

                if slot.is_cell {
                    IntrinsicDest::Cell(cell_offset(self, destination)?)
                } else {
                    IntrinsicDest::Frame(destination)
                }
            }
            None => IntrinsicDest::None,
        };

        // reject compile-time intrinsics before building the side record
        validate_runtime_intrinsic(intrinsic)?;
        let intrinsic = Intrinsic::new(intrinsic, dest, arguments, &layouts).ok_or_else(|| {
            Error::unsupported_instruction("intrinsic with more than 16 arguments")
        })?;

        Ok(pool.instruction_with_side(Op::Intrinsic, intrinsic))
    }
}

/// Validate one runtime intrinsic.
fn validate_runtime_intrinsic(intrinsic: mir::Intrinsic) -> Result<()> {
    match intrinsic {
        mir::Intrinsic::TypeOf | mir::Intrinsic::SizeOf | mir::Intrinsic::AlignOf => {
            Err(Error::unsupported_instruction(format!(
                "intrinsic.{} (compile-time only)",
                intrinsic.to_str()
            )))
        }
        _ => Ok(()),
    }
}
