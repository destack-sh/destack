use destack_mir as mir;

use crate::program::Instruction;
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::op::select_field_addr_op;

/// Encode one fixed byte offset into an instruction operand.
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
        let layout = self
            .value_layout_map()
            .get(base)
            .ok_or(Error::InvalidInstruction)?;
        let op = select_field_addr_op(self.value_layout_map(), base)?;
        let field = self.field_projection_for_value(base, index)?;
        let base = if layout.is_frame_storage() {
            value_offset(self, base)?
        } else {
            word_offset(self, base)?
        };

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            base,
            0,
            instruction_byte_offset(field.byte_offset)?,
        ))
    }
}
