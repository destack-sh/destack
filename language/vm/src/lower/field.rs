use destack_mir as mir;

use crate::program::{Instruction, Op};
use crate::{Error, Result};

use super::lower::BlockLowerer;
use super::op::select_field_addr_op;
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

        // lower frame projections through the frame address op
        let op = select_field_addr_op(self.value_layout_map(), base)?;
        let field = self.field_access_for_value(base, index)?;
        if op == Op::AddressFrame {
            let reference = reference_meta_for_value(self.value_layout_map(), destination);
            let access = pool.frame_access(field.into());

            return Ok(Instruction::new(
                op,
                destination.id(),
                base.id(),
                reference.bits() as u32,
                access.0,
            ));
        }

        // lower memory projections through the selected address family
        let field = pool.field_access(field);
        let reference = reference_meta_for_value(self.value_layout_map(), destination);

        Ok(Instruction::new(
            op,
            destination.id(),
            base.id(),
            reference.bits() as u32,
            field.0,
        ))
    }
}
