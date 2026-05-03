use destack_mir as mir;

use crate::program::{BarrierWrite, Instruction, Opcode, ValueLayout, value_layout_from_type};
use crate::{Error, Result};

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
        let object = object.value().ok_or_else(|| Error::MissingRepresentation {
            context: "barrier.write object".to_string(),
        })?;
        let offset = offset.value().ok_or_else(|| Error::MissingRepresentation {
            context: "barrier.write offset".to_string(),
        })?;
        let byte_len = byte_len
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "barrier.write byte length".to_string(),
            })?;

        // encode the collector that owns this reference
        let object_type = self.value_type_for_value(object)?;
        let object_layout = value_layout_from_type(self.tree, object_type);
        let ValueLayout::Pointer { pointer_class, .. } = object_layout else {
            return Err(Error::TypeMismatch {
                expected: "managed barrier reference".to_string(),
                actual: format!("{object_layout:?}"),
            });
        };

        Ok(Instruction::new(
            Opcode::BarrierWrite,
            BarrierWrite {
                object,
                offset,
                byte_len,
                pointer_class,
            },
        ))
    }
}
