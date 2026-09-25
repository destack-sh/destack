use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one exact counter increment.
    pub(super) fn emit_profile_increment(
        &mut self,
        counter: mir::CounterId,
    ) -> Result<(), EmitError> {
        let mut instruction =
            bytecode::InstructionBuilder::new(bytecode::Opcode::PROFILE_INCREMENT);
        instruction.counter(bytecode::CounterId(counter.0));

        self.encode(instruction, &[])
    }

    /// Emit one exact sample observation.
    pub(super) fn emit_profile_sample(
        &mut self,
        sampler: mir::SamplerId,
        value: mir::Value,
    ) -> Result<(), EmitError> {
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::PROFILE_SAMPLE);
        instruction.sampler(bytecode::SamplerId(sampler.0));
        instruction.register(self.word(value)?);

        self.encode(instruction, &[])
    }
}
