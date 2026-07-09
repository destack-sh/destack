use destack_mir as mir;

use crate::LinkResult;

use destack_program::vm::{Instruction, Op};

use super::lower::BlockLowerer;
use super::value::Operand;

impl<'a> BlockLowerer<'a> {
    /// Lower one write barrier.
    pub(super) fn lower_barrier_write(
        &self,
        object: mir::Value,
        offset: mir::Value,
        byte_len: mir::Value,
    ) -> LinkResult<Instruction> {
        // require SSA values

        // encode the collector that owns this reference
        let object_type = self.value_type_for_value(object)?;
        let object_layout = self.function.operand_for_type(object_type);
        let Some(Operand::Reference { space, .. }) = object_layout else {
            return Err(
                self.type_mismatch("managed barrier reference", format!("{object_layout:?}"))
            );
        };

        let op = match space {
            mir::Space::Local => Op::BarrierWriteHeap,
            mir::Space::Shared => Op::BarrierWriteSharedHeap,
            _ => {
                return Err(self.type_mismatch("managed barrier reference", format!("{space:?}")));
            }
        };

        Ok(Instruction::new(
            op,
            self.cell_offset(object)?,
            self.cell_offset(offset)?,
            self.cell_offset(byte_len)?,
            0,
        ))
    }
}
