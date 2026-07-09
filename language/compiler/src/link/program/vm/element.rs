use destack_mir as mir;

use destack_program::vm::Instruction;

use crate::LinkResult;

use super::lower::BlockLowerer;
use super::op::{select_element_addr_op, select_slice_element_addr_op};
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
    /// Lower one element address.
    pub(super) fn lower_element_addr(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::Value,
        array: mir::Value,
        index: mir::Value,
    ) -> LinkResult<Instruction> {
        // slice descriptors have a distinct address path
        let pointee_type = self.projection_type_for_value(array)?;
        if let Some(access) =
            pointee_type.and_then(|pointee_type| self.slice_projection(pointee_type))
        {
            let pointer = pointee_type
                .and_then(|pointee_type| self.slice_element_cell_layout(pointee_type))
                .ok_or_else(|| self.invalid_instruction("slice element pointer layout"))?;
            let op = select_slice_element_addr_op(pointer)
                .ok_or_else(|| self.invalid_instruction("slice element address operation"))?;
            let access = pool.slice_projection(access);

            return Ok(Instruction::new(
                op,
                self.cell_offset(destination)?,
                self.value_offset(array)?,
                self.cell_offset(index)?,
                access.0,
            ));
        }

        // frame projections keep the dynamic index in frame tables
        let layout = self
            .operand_map()
            .get(array)
            .ok_or_else(|| self.invalid_instruction("array operand"))?;
        let op = select_element_addr_op(self.operand_map(), array)
            .ok_or_else(|| self.invalid_instruction("element address operation"))?;
        let element = self.element_projection_for_value(array)?;
        let array_length = self.array_length_for_value(array)?;
        if layout.is_frame_backed() {
            let access = pool.projection(element.with_length(array_length));

            return Ok(Instruction::new(
                op,
                self.cell_offset(destination)?,
                self.value_offset(array)?,
                self.cell_offset(index)?,
                access.0,
            ));
        }

        // memory projections use the selected address family
        let element = pool.projection(element);

        Ok(Instruction::new(
            op,
            self.cell_offset(destination)?,
            self.cell_offset(array)?,
            self.cell_offset(index)?,
            element.0,
        ))
    }
}
