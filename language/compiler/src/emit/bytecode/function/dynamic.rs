use std::mem::size_of;

use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one dynamic value binding.
    pub(super) fn emit_dynamic_bind(
        &mut self,
        destination: mir::Value,
        payload: mir::Value,
        concrete: mir::TypeId,
    ) -> Result<(), EmitError> {
        let dynamic_type = self.value_type(destination)?;
        let mir::Type::Dynamic { constraint, .. } = self.optimized.tree.get(dynamic_type) else {
            return Err(self.internal("dynamic binding result is not dynamic"));
        };
        let table = self.types.dynamic_id(concrete, *constraint)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::DYNAMIC_BIND);
        instruction.register(self.word(payload)?);
        instruction.dynamic_table(table);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one erased dynamic payload projection.
    pub(super) fn emit_dynamic_payload(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
    ) -> Result<(), EmitError> {
        let payload_byte_len = size_of::<u64>() as u32;

        self.emit_extract(destination, dynamic, 0, payload_byte_len)
    }

    /// Emit one dynamic concrete-type projection.
    pub(super) fn emit_dynamic_type(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::DYNAMIC_TYPE);
        instruction.span(self.register(dynamic)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }
}
