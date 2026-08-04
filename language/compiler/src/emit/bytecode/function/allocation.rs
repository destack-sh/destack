use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one direct allocation operation.
    pub(super) fn emit_new(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        result_type: mir::TypeId,
        kind: bytecode::NewKind,
        initialization: bytecode::Initialization,
        length: Option<mir::Value>,
    ) -> Result<(), EmitError> {
        let point = self.object.instruction_point(instruction_id);
        let allocation = self
            .object
            .allocation_index(point)
            .ok_or_else(|| self.internal("missing allocation site"))?;
        let result = self.types.register_type(result_type)?;
        let reference = match kind {
            bytecode::NewKind::Value => result.reference_type(),
            bytecode::NewKind::Slice => result.slice_reference(),
        }
        .ok_or_else(|| self.internal("allocation result is not a reference"))?;
        let space = reference
            .storage()
            .heap_space()
            .ok_or_else(|| self.internal("allocation requires heap storage"))?;
        let operation = bytecode::New {
            space,
            ownership: reference.kind(),
            kind,
            initialization,
            is_fallible: false,
        };
        let opcode = bytecode::Opcode::new(operation)
            .ok_or_else(|| self.internal("invalid allocation operation"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.relocation(bytecode::RelocationTag::ALLOCATION, allocation);
        if let Some(length) = length {
            instruction.register(self.word(length)?);
        }
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one fallible allocation terminator and its explicit edges.
    pub(super) fn emit_new_try(
        &mut self,
        block: mir::BlockId,
        terminator: &mir::Terminator,
        kind: bytecode::NewKind,
        initialization: bytecode::Initialization,
        length: Option<mir::Value>,
        success: &mir::BlockTarget,
        failure: &mir::BlockTarget,
    ) -> Result<(), EmitError> {
        let point = self.object.terminator_point(block);
        let allocation = self
            .object
            .allocation_index(point)
            .ok_or_else(|| self.internal("missing allocation site"))?;
        let destinations = self.successor_destinations(terminator, success)?;
        let [destination] = destinations.as_slice() else {
            return Err(self.internal("fallible allocation requires one result"));
        };
        let result_type = self
            .optimized
            .tree
            .get(success.block)
            .parameters
            .first()
            .map(|parameter| parameter.ty)
            .ok_or_else(|| self.internal("fallible allocation result type is missing"))?;
        let result = self.types.register_type(result_type)?;
        let reference = match kind {
            bytecode::NewKind::Value => result.reference_type(),
            bytecode::NewKind::Slice => result.slice_reference(),
        }
        .ok_or_else(|| self.internal("allocation result is not a reference"))?;
        let operation = bytecode::New::select(reference, kind, initialization, true)
            .ok_or_else(|| self.internal("invalid fallible allocation operation"))?;
        let opcode = bytecode::Opcode::new(operation)
            .ok_or_else(|| self.internal("invalid fallible allocation opcode"))?;
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.relocation(bytecode::RelocationTag::ALLOCATION, allocation);
        if let Some(length) = length {
            instruction.register(self.word(length)?);
        }
        instruction.branch(self.edge_label(terminator, success)?);
        instruction.branch(self.edge_label(terminator, failure)?);

        self.encode(instruction, &[*destination])
    }

    /// Complete one initialization token without runtime work.
    pub(super) fn emit_new_complete(
        &mut self,
        destination: mir::Value,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let ty = self.register_type(destination)?;
        let source = self.register(value)?;
        let destination = self.register(destination)?;

        self.emit_move(source, destination, ty)
    }
}
