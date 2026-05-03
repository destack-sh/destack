use destack_mir as mir;

use crate::program::{AddressFrame, FieldAddr, Instruction, Opcode};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::opcode::select_field_addr_opcode;
use super::pool::Pool;
use super::value::reference_meta_for_value;

impl<'a> BlockLowerer<'a> {
    /// Lower one field address.
    pub(super) fn lower_field_addr(
        &self,
        pool: &mut Pool<'_>,
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

        // lower frame projections through the frame address opcode
        let opcode = select_field_addr_opcode(self.value_layout_map(), base)?;
        let field = self.field_access_for_value(base, index)?;
        if opcode == Opcode::AddressFrame {
            return Ok(Instruction::new(
                opcode,
                AddressFrame {
                    dest: destination,
                    base,
                    reference: reference_meta_for_value(self.value_layout_map(), destination),
                    access: pool.frame_access(field.into()),
                },
            ));
        }

        // lower memory projections through the selected address family
        Ok(Instruction::new(
            opcode,
            FieldAddr {
                dest: destination,
                base,
                index,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
                field_count: self.field_count_for_value(base)?,
                field: pool.field_access(field),
            },
        ))
    }
}
