use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Pack physical source ranges into one MIR aggregate destination.
    pub(super) fn emit_aggregate_registers(
        &mut self,
        destination: mir::Value,
        values: &[bytecode::RegisterSpan],
    ) -> Result<(), EmitError> {
        let ty = self.value_type(destination)?;
        let placements = values
            .iter()
            .enumerate()
            .map(|(index, registers)| {
                let (byte_offset, byte_len) = self.types.placement(ty, index as u32)?;

                Ok(bytecode::Placement::new(*registers, byte_offset, byte_len))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::AGGREGATE);
        instruction
            .placements(&placements)
            .map_err(|error| self.bytecode_error(error))?;
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Construct one packed aggregate from logical source values.
    pub(super) fn emit_aggregate(
        &mut self,
        destination: mir::Value,
        values: mir::ValueSlice,
    ) -> Result<(), EmitError> {
        // read the aggregate storage type
        let ty = self
            .function
            .value_type(destination)
            .ok_or_else(|| self.internal("missing aggregate type"))?;
        let values = self.optimized.tree.get_values(values);
        let placements = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let registers = self.register(*value)?;
                let (byte_offset, byte_len) = self.types.placement(ty, index as u32)?;

                Ok(bytecode::Placement::new(registers, byte_offset, byte_len))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::AGGREGATE);
        instruction
            .placements(&placements)
            .map_err(|error| self.bytecode_error(error))?;
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Read one source-ordered field from a packed value.
    pub(super) fn emit_field_get(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        field: u32,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let field = self.types.field(aggregate_type, field)?;

        self.emit_extract(destination, aggregate, field.offset, field.size)
    }

    /// Replace one source-ordered field inside a packed value.
    pub(super) fn emit_field_set(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        field: u32,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let field = self.types.field(aggregate_type, field)?;

        self.emit_insert(destination, aggregate, field.offset, field.size, value)
    }

    /// Read one fixed element from a packed value.
    pub(super) fn emit_element_get(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        index: u32,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let (byte_offset, byte_len) = self.types.element(aggregate_type, index)?;

        self.emit_extract(destination, aggregate, byte_offset, byte_len)
    }

    /// Replace one fixed element inside a packed value.
    pub(super) fn emit_element_set(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        index: u32,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let aggregate_type = self.value_type(aggregate)?;
        let (byte_offset, byte_len) = self.types.element(aggregate_type, index)?;

        self.emit_insert(destination, aggregate, byte_offset, byte_len, value)
    }

    /// Read one exact byte range from a packed value.
    pub(super) fn emit_extract(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        byte_offset: u32,
        byte_len: u32,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::EXTRACT);
        instruction.span(self.register(aggregate)?);
        instruction.u32(byte_offset);
        instruction.u32(byte_len);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Replace one exact byte range inside a packed value.
    pub(super) fn emit_insert(
        &mut self,
        destination: mir::Value,
        aggregate: mir::Value,
        byte_offset: u32,
        byte_len: u32,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::INSERT);
        instruction.span(self.register(aggregate)?);
        instruction.u32(byte_offset);
        instruction.u32(byte_len);
        instruction.span(self.register(value)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }
}
