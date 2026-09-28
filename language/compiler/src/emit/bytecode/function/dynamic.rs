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

    /// Emit one runtime type read of a dynamic value or a class object.
    pub(super) fn emit_type_of(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        // read a class object's type through the virtual table its dispatch field names
        let ty = self.value_type(value)?;
        let ty = self.optimized.tree.type_definition(ty).clone();
        if let mir::Type::Reference { pointee, .. } = ty {
            let reference = self
                .register_type(value)?
                .reference_type()
                .ok_or_else(|| self.internal("object type read of a non-reference value"))?;
            let layout = self
                .optimized
                .layouts
                .type_layout(pointee)
                .ok_or_else(|| self.internal("object type read of a type without a layout"))?;
            let mir::LayoutShape::Object(layout) = &layout.shape else {
                return Err(self.internal("object type read of a non-object type"));
            };
            let dispatch_offset = layout.dispatch_offset.ok_or_else(|| {
                self.internal("object type read of a class without a dispatch field")
            })?;
            let mut instruction =
                bytecode::InstructionBuilder::new(bytecode::Opcode::TYPE_OF_OBJECT);
            instruction.register(self.word(value)?);
            instruction.reference(reference.kind(), reference.storage());
            instruction.u32(dispatch_offset);
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
