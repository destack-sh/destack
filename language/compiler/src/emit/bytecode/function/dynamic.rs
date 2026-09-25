use tspp_bytecode as bytecode;
use tspp_mir as mir;

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
        // read the dynamic representation and its dispatch table
        let dynamic_type = self
            .optimized
            .tree
            .storage_type(self.value_type(destination)?);
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
        let dynamic = self.register(dynamic)?;
        let payload = bytecode::RegisterSpan::new(dynamic.start, 1);
        let destination_type = self.register_type(destination)?;
        let destination = self.register(destination)?;

        self.emit_move(payload, destination, destination_type)
    }

    /// Emit one dynamic field read.
    pub(super) fn emit_dynamic_read(
        &mut self,
        destination: mir::Value,
        dynamic: mir::Value,
        slot: mir::DispatchSlot,
        result_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        let slot = u16::try_from(slot.0)
            .map_err(|_| self.internal("dynamic dispatch slot exceeds bytecode"))?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::DYNAMIC_READ);
        instruction.span(self.register(dynamic)?);
        instruction.u16(slot);
        instruction.u32(self.types.byte_len(result_type)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
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
