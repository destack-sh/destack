use destack_mir as mir;

use crate::program::{Instruction, Op};
use crate::{Error, Result};

use super::frame::{value_offset, word_offset};
use super::lower::BlockLowerer;
use super::op::{select_element_addr_op, select_slice_element_addr_op};
use super::pool::Pool;
use super::projection::{slice_element_pointer_class, slice_projection};

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
            .ok_or_else(|| Error::MissingRepresentation {
                context: "element address destination".to_string(),
            })?;
        let array = array.value().ok_or_else(|| Error::MissingRepresentation {
            context: "element address array".to_string(),
        })?;
        let index = index.value().ok_or_else(|| Error::MissingRepresentation {
            context: "element address index".to_string(),
        })?;

        // slice descriptors have a distinct address path
        let pointee_type = self.projection_type_for_value(array)?;
        if let Some(access) = pointee_type
            .and_then(|pointee_type| slice_projection(self.tree, self.layouts(), pointee_type))
        {
            let pointer_class = pointee_type
                .and_then(|pointee_type| slice_element_pointer_class(self.tree, pointee_type))
                .ok_or(Error::InvalidInstruction)?;
            let op = select_slice_element_addr_op(pointer_class)?;
            let access = pool.slice_projection(access);

            return Ok(Instruction::new(
                op,
                word_offset(self, destination)?,
                value_offset(self, array)?,
                word_offset(self, index)?,
                access.0,
            ));
        }

        // frame projections keep the dynamic index in frame metadata
        let op = select_element_addr_op(self.value_layout_map(), array)?;
        let element = self.element_projection_for_value(array)?;
        let array_length = self.array_length_for_value(array)?;
        if op == Op::AddressFrameElement {
            let access = pool.projection(element.with_length(array_length));

            return Ok(Instruction::new(
                op,
                word_offset(self, destination)?,
                value_offset(self, array)?,
                word_offset(self, index)?,
                access.0,
            ));
        }

        // memory projections use the selected address family
        let element = pool.projection(element);

        Ok(Instruction::new(
            op,
            word_offset(self, destination)?,
            word_offset(self, array)?,
            word_offset(self, index)?,
            element.0,
        ))
    }
}
