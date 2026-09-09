use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one generated destructor call.
    pub(super) fn emit_drop(&mut self, value: mir::Value) -> Result<(), EmitError> {
        // select the destructor for the value type
        let ty = self.value_type(value)?;
        let destructor = self.optimized.drops.destructor(ty, mir::Storage::Frame);
        let instruction = match destructor {
            // call one statically selected frame destructor
            Some(destructor) => {
                let destructor = self.types.function_id(destructor)?;
                let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::DROP);
                instruction.span(self.register(value)?);
                instruction.relocation(bytecode::RelocationTag::FUNCTION, destructor.0);

                instruction
            }
            // select one erased payload or environment destructor from its owner allocation
            None if matches!(
                self.optimized
                    .tree
                    .get(self.optimized.tree.storage_type(ty)),
                mir::Type::Dynamic {
                    kind: mir::ReferenceKind::Unique,
                    ..
                } | mir::Type::Function {
                    kind: mir::ReferenceKind::Unique,
                    ..
                }
            ) =>
            {
                let mut instruction =
                    bytecode::InstructionBuilder::new(bytecode::Opcode::DROP_INDIRECT);
                instruction.register(self.representation_register(self.register(value)?, value)?);

                instruction
            }
            None => return Err(self.internal("drop has no destructor")),
        };

        self.encode(instruction, &[])
    }
}
