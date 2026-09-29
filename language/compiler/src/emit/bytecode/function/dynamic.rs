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
        table: mir::BindTable,
    ) -> Result<(), EmitError> {
        // bind a class payload through its runtime class's conformance slot
        if let mir::BindTable::Virtual { slot } = table {
            let slot = u16::try_from(slot.0)
                .map_err(|_| self.internal("conformance slot exceeds bytecode"))?;
            let mut instruction =
                bytecode::InstructionBuilder::new(bytecode::Opcode::DYNAMIC_BIND_VIRTUAL);
            instruction.register(self.word(payload)?);
            instruction.u16(slot);
            let destination = self.register(destination)?;

            return self.encode(instruction, &[destination]);
        }

        // read the dynamic representation and its dispatch table
        let dynamic_type = self.value_type(destination)?.storage(&self.optimized.tree);
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

    /// Emit one runtime type read of a dynamic value or a class object.
    pub(super) fn emit_type_of(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        // read a class object's type through its virtual table
        let ty = self.value_type(value)?;
        if let mir::Type::Reference { .. } = self.optimized.tree.type_definition(ty) {
            let mut instruction =
                bytecode::InstructionBuilder::new(bytecode::Opcode::TYPE_OF_OBJECT);
            instruction.register(self.word(value)?);
            let destination = self.register(destination)?;

            return self.encode(instruction, &[destination]);
        }

        // read a dynamic value's type through its dispatch table
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::TYPE_OF_DYNAMIC);
        instruction.span(self.register(value)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }
}
