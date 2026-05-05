use destack_mir as mir;

use crate::program::{Instruction, Op};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::op::select_field_addr_op;
use super::value::reference_meta_for_value;

/// Encode one fixed byte offset into an instruction lane.
fn instruction_byte_offset(byte_offset: usize) -> Result<u32> {
    u32::try_from(byte_offset).map_err(|_| Error::InvalidInstruction)
}

impl<'a> BlockLowerer<'a> {
    /// Lower one field address.
    pub(super) fn lower_field_addr(
        &self,
        destination: mir::ValueReference,
        base: mir::ValueReference,
        index: u32,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "field address destination".to_string(),
            })?;
        let base = base.value().ok_or_else(|| Error::MissingRepresentation {
            context: "field address base".to_string(),
        })?;

        // lower fixed projections as base plus byte offset
        let op = select_field_addr_op(self.value_layout_map(), base)?;
        let field = self.field_access_for_value(base, index)?;
        let reference = reference_meta_for_value(self.value_layout_map(), destination);
        let base = if op == Op::AddressFrameOffset {
            value_offset(self, base)?
        } else {
            word_offset(self, base)?
        };

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            base,
            reference.bits() as u32,
            instruction_byte_offset(field.byte_offset)?,
        ))
    }
}
