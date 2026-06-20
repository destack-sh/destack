use destack_mir as mir;

use crate::{Error, Result};
use destack_program::vm::{Instruction, Intrinsic, IntrinsicDest, Op};

use super::frame::cell_offset;
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
    ) -> Result<Instruction> {
        // collect argument values and static layouts
        let argument_values = self.tree.get_values(arguments);
        let mut layouts = Vec::with_capacity(argument_values.len());
        for argument in argument_values {
            let layout = self
                .value_shape_map()
                .get(*argument)
                .ok_or(Error::invalid_instruction())?;

            layouts.push(layout);
        }

        // intern the argument range
        let arguments = pool.argument_range(argument_values);

        // resolve the optional destination
        let dest = match destination {
            Some(destination) => {
                let slot = self
                    .frame_layout
                    .value(destination.0)
                    .ok_or(Error::invalid_instruction())?;

                if self.slot_is_cell(slot) {
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
