use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_program::ContextNode;

use crate::EmitError;

use super::FunctionEmitter;

impl FunctionEmitter<'_> {
    /// Emit one current execution context load.
    pub(super) fn emit_context_current(
        &mut self,
        destination: mir::Value,
    ) -> Result<(), EmitError> {
        let instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::CONTEXT_CURRENT);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one current execution context replacement.
    pub(super) fn emit_context_replace(
        &mut self,
        destination: mir::Value,
        context: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::CONTEXT_REPLACE);
        instruction.register(self.word(context)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one immutable context extension.
    pub(super) fn emit_context_bind(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        context: mir::Value,
        variable: mir::Value,
        value: mir::Value,
        node_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        let allocation = self.context_allocation(instruction_id)?;
        let value_offset = self.context_value_offset(node_type)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::CONTEXT_BIND);
        instruction.register(self.word(context)?);
        instruction.register(self.word(variable)?);
        instruction.span(self.register(value)?);
        instruction.relocation(bytecode::RelocationTag::ALLOCATION, allocation);
        instruction.u32(value_offset);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one context value lookup.
    pub(super) fn emit_context_get(
        &mut self,
        destination: mir::Value,
        context: mir::Value,
        variable: mir::Value,
        default: mir::Value,
        node_type: mir::TypeId,
    ) -> Result<(), EmitError> {
        let value_offset = self.context_value_offset(node_type)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::CONTEXT_GET);
        instruction.register(self.word(context)?);
        instruction.register(self.word(variable)?);
        instruction.span(self.register(default)?);
        instruction.u32(value_offset);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Resolve one context node allocation site.
    fn context_allocation(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
    ) -> Result<u32, EmitError> {
        let point = self.object.instruction_point(instruction);

        self.object
            .allocation_index(point)
            .ok_or_else(|| self.internal("missing context allocation site"))
    }

    /// Resolve and verify one context node's inline value offset.
    fn context_value_offset(&self, node_type: mir::TypeId) -> Result<u32, EmitError> {
        let parent = self.types.field(node_type, 0)?;
        let variable = self.types.field(node_type, 1)?;
        let value = self.types.field(node_type, 2)?;

        // require every generated context node to share the Program header ABI
        if parent.offset as usize != ContextNode::PARENT_OFFSET
            || variable.offset as usize != ContextNode::VARIABLE_OFFSET
            || value.offset < ContextNode::BYTE_LEN as u32
        {
            return Err(self.internal("context node does not match the Program ABI"));
        }

        Ok(value.offset)
    }
}
