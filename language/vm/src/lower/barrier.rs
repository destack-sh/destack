use destack_mir as mir;

use crate::{Error, Result};
use destack_program::vm::{AddressSpace, Instruction, Op, ValueShape, value_shape_from_type};

use super::frame::cell_offset;
use super::lower::BlockLowerer;

impl<'a> BlockLowerer<'a> {
    /// Lower one write barrier.
    pub(super) fn lower_barrier_write(
        &self,
        object: mir::Value,
        offset: mir::Value,
        byte_len: mir::Value,
    ) -> Result<Instruction> {
        // require SSA values

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
