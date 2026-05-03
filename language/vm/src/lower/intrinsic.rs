use destack_mir as mir;

use crate::program::{Instruction, Intrinsic, Opcode};
use crate::{Error, Result};

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
        // intern the argument range
        let arguments = pool
            .argument_reference_range(self.tree.get_arguments(arguments), "intrinsic argument")?;

        // resolve the optional destination
        let destination = destination
            .map(|value| {
                value.value().ok_or_else(|| Error::MissingRepresentation {
                    context: "intrinsic destination".to_string(),
                })
            })
            .transpose()?;

        Ok(Instruction::new(
            Opcode::Intrinsic,
            Intrinsic {
                dest: destination,
                intrinsic,
                arguments,
            },
        ))
    }
}
