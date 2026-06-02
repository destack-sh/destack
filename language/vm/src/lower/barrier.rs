use destack_mir as mir;

use crate::program::{AddressSpace, Instruction, Op, ValueShape, value_shape_from_type};
use crate::{Error, Result};

use super::frame::cell_offset;
use super::lower::BlockLowerer;

impl<'a> BlockLowerer<'a> {
    /// Lower one write barrier.
    pub(super) fn lower_barrier_write(
        &self,
        object: mir::ValueReference,
        offset: mir::ValueReference,
        byte_len: mir::ValueReference,
    ) -> Result<Instruction> {
        // require SSA values
        let object = object
            .value()
            .ok_or_else(|| Error::invalid_program("barrier.write object"))?;
        let offset = offset
            .value()
            .ok_or_else(|| Error::invalid_program("barrier.write offset"))?;
        let byte_len = byte_len
            .value()
            .ok_or_else(|| Error::invalid_program("barrier.write byte length"))?;

        // encode the collector that owns this reference
        let object_type = self.value_type_for_value(object)?;
        let object_layout = value_shape_from_type(self.tree, object_type);
        let Some(ValueShape::Pointer { address_space, .. }) = object_layout else {
            return Err(Error::type_mismatch(
                "managed barrier reference",
                format!("{object_layout:?}"),
            ));
        };

        let op = match address_space {
            AddressSpace::Local => Op::BarrierWriteHeap,
            AddressSpace::Shared => Op::BarrierWriteSharedHeap,
            _ => {
                return Err(Error::type_mismatch(
                    "managed barrier reference",
                    format!("{address_space:?}"),
                ));
            }
        };

        Ok(Instruction::new(
            op,
            cell_offset(self, object)?,
            cell_offset(self, offset)?,
            cell_offset(self, byte_len)?,
            0,
        ))
    }
}
