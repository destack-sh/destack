use destack_mir as mir;

use crate::program::Instruction;
use crate::{Error, Result};

use super::frame::{cell_offset, value_offset};
use super::lower::BlockLowerer;
use super::op::{select_element_addr_op, select_slice_element_addr_op};
use super::pool::Pool;
use super::projection::{slice_element_address_space, slice_projection};

impl<'a> BlockLowerer<'a> {
    /// Lower one element address.
    pub(super) fn lower_element_addr(
        &self,
        pool: &mut Pool<'_, '_>,
        destination: mir::ValueReference,
        array: mir::ValueReference,
        index: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let destination = destination
            .value()
            .ok_or_else(|| Error::invalid_program("element address destination"))?;
        let array = array
            .value()
            .ok_or_else(|| Error::invalid_program("element address array"))?;
        let index = index
            .value()
            .ok_or_else(|| Error::invalid_program("element address index"))?;

        // slice descriptors have a distinct address path
        let pointee_type = self.projection_type_for_value(array)?;
        if let Some(access) = pointee_type
            .and_then(|pointee_type| slice_projection(self.tree, self.layouts(), pointee_type))
        {
            let address_space = pointee_type
                .and_then(|pointee_type| slice_element_address_space(self.tree, pointee_type))
                .ok_or(Error::invalid_instruction())?;
            let op = select_slice_element_addr_op(address_space)?;
            let access = pool.slice_projection(access);

            return Ok(Instruction::new(
                op,
                cell_offset(self, destination)?,
                value_offset(self, array)?,
                cell_offset(self, index)?,
                access.0,
            ));
        }

        // frame projections keep the dynamic index in frame metadata
        let layout = self
            .value_shape_map()
            .get(array)
            .ok_or(Error::invalid_instruction())?;
        let op = select_element_addr_op(self.value_shape_map(), array)?;
        let element = self.element_projection_for_value(array)?;
        let array_length = self.array_length_for_value(array)?;
        if layout.is_frame_storage() {
            let access = pool.projection(element.with_length(array_length));

            return Ok(Instruction::new(
                op,
                cell_offset(self, destination)?,
                value_offset(self, array)?,
                cell_offset(self, index)?,
                access.0,
            ));
        }

        // memory projections use the selected address family
        let element = pool.projection(element);

        Ok(Instruction::new(
            op,
            cell_offset(self, destination)?,
            cell_offset(self, array)?,
            cell_offset(self, index)?,
            element.0,
        ))
    }
}
