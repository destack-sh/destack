use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one direct allocation operation.
    pub(super) fn emit_new(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        destination: mir::Value,
        kind: bytecode::NewKind,
        initialization: bytecode::Initialization,
        length: Option<mir::Value>,
    ) -> Result<(), EmitError> {
        let point = self.object.instruction_point(instruction_id);
        let allocation = self
            .object
            .allocation_index(point)
            .ok_or_else(|| self.internal("missing allocation site"))?;
        let operation = bytecode::New {
            kind,
            initialization,
            is_fallible: false,
        };
        let opcode = bytecode::Opcode::new(operation);
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
        let destinations =
            self.successor_destinations(terminator, mir::Successor::NewSuccess, success)?;
        let [destination] = destinations.as_slice() else {
            return Err(self.internal("fallible allocation requires one result"));
        };
        let operation = bytecode::New {
            kind,
            initialization,
            is_fallible: true,
        };
        let opcode = bytecode::Opcode::new(operation);
        let mut instruction = bytecode::InstructionBuilder::new(opcode);
        instruction.relocation(bytecode::RelocationTag::ALLOCATION, allocation);
        if let Some(length) = length {
            instruction.register(self.word(length)?);
        }
        instruction.branch(self.edge_label(terminator, mir::Successor::NewSuccess, success)?);
        instruction.branch(self.edge_label(terminator, mir::Successor::NewFailure, failure)?);

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
