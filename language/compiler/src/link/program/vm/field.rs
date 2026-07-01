use destack_mir as mir;

use destack_program::vm::Instruction;

use crate::LinkResult;

use super::lower::BlockLowerer;
use super::op::select_field_addr_op;

impl<'a> BlockLowerer<'a> {
    /// Lower one field address.
    pub(super) fn lower_field_addr(
        &self,
        destination: mir::Value,
        base: mir::Value,
        index: u32,
    ) -> LinkResult<Instruction> {
        // lower fixed projections as base plus byte offset
        let layout = self
            .operand_map()
            .get(base)
            .ok_or_else(|| self.invalid_instruction("field base operand"))?;
        let op = select_field_addr_op(self.operand_map(), base)
            .ok_or_else(|| self.invalid_instruction("field address operation"))?;
        let field = self.field_projection_for_value(base, index)?;
        let base = if layout.is_frame_backed() {
            self.value_offset(base)?
        } else {
            self.cell_offset(base)?
        };

        Ok(Instruction::new(
            op,
            self.cell_offset(destination)?,
            base,
            0,
            self.instruction_byte_offset(field.byte_offset())?,
        ))
    }
}
