use destack_mir as mir;

use crate::program::{AddressFrameElement, ElementAddr, Instruction, Opcode, SliceElementAddr};
use crate::{Error, Result};

use super::access::slice_element_access;
use super::lower::BlockLowerer;
use super::opcode::select_element_addr_opcode;
use super::pool::Pool;
use super::value::{pointer_class_for_value, reference_meta_for_value};

impl<'a> BlockLowerer<'a> {
    /// Lower one element address.
    pub(super) fn lower_element_addr(
        &self,
        pool: &mut Pool<'_>,
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
        let pointer_class = pointer_class_for_value(self.value_layout_map(), array);
        let pointee_type = self.projection_type_for_value(array)?;
        if let Some(access) = pointee_type.and_then(|pointee_type| {
            slice_element_access(self.tree, self.layouts(), pointee_type, pointer_class)
        }) {
            return Ok(Instruction::new(
                Opcode::AddressSliceElement,
                SliceElementAddr {
                    dest: destination,
                    slice: array,
                    index,
                    reference: reference_meta_for_value(self.value_layout_map(), destination),
                    access: pool.slice_element_access(access),
                },
            ));
        }

        // frame projections keep the dynamic index in frame metadata
        let opcode = select_element_addr_opcode(self.value_layout_map(), array)?;
        let element = self.element_access_for_value(array)?;
        let array_length = self.array_length_for_value(array)?;
        if opcode == Opcode::AddressFrame {
            return Ok(Instruction::new(
                Opcode::AddressFrameElement,
                AddressFrameElement {
                    dest: destination,
                    base: array,
                    index,
                    reference: reference_meta_for_value(self.value_layout_map(), destination),
                    access: pool.frame_access(element.into_frame_access(0, array_length)),
                },
            ));
        }

        // memory projections use the selected address family
        Ok(Instruction::new(
            opcode,
            ElementAddr {
                dest: destination,
                array,
                index,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
                array_length,
                element: pool.element_access(element),
            },
        ))
    }
}
