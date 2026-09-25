use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one generated destructor call.
    pub(super) fn emit_drop(&mut self, value: mir::Value) -> Result<(), EmitError> {
        // select the destructor for the value type
        let ty = self.value_type(value)?;
        let Some(destructor) = self.optimized.drops.destructor(ty, mir::Storage::Frame) else {
            return Err(self.internal("drop has no destructor"));
        };

        // call one statically selected frame destructor
        let destructor = self.types.function_id(destructor)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::DROP);
        instruction.span(self.register(value)?);
        instruction.relocation(bytecode::RelocationTag::FUNCTION, destructor.0);

        self.encode(instruction, &[])
    }
}
