use destack_bytecode::{Instruction, Opcode};
use destack_program::{FunctionId, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one function value operation.
    pub(crate) fn execute_function(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();

        match instruction.opcode() {
            Opcode::FUNCTION_ADDRESS => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let function = operands.u32().map_err(|_| self.invalid_instruction())?;

                self.write(target.0, Word::from_bits(function as u64));
            }
            Opcode::FUNCTION_BIND => {
                let target = operands.range().map_err(|_| self.invalid_instruction())?;
                let function = operands.u32().map_err(|_| self.invalid_instruction())?;
                let environment = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.start.0, Word::from_bits(function as u64));
                self.write(target.start.0 + 1, self.read(environment.0));
            }
            Opcode::FUNCTION_ENVIRONMENT => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let _environment_type = operands
                    .value_type()
                    .map_err(|_| self.invalid_instruction())?;
                let function = operands.range().map_err(|_| self.invalid_instruction())?;
                if function.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.0, self.read(function.start.0 + 1));
            }
            Opcode::FUNCTION_ENVIRONMENT_CURRENT => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let frame = self.frame();
                let body = self.body(frame.function)?;
                if body.environment.get().is_none() {
                    return Err(self.invalid_instruction());
                }

                self.write(target.0, self.read(0));
            }
            _ => unreachable!("function dispatch selects one function opcode"),
        }

        Ok(())
    }

    /// Decode one function id stored in a callable word.
    pub(crate) fn function_id(&self, word: Word) -> Result<FunctionId> {
        let function = u32::try_from(word.bits()).map_err(|_| self.invalid_instruction())?;

        Ok(FunctionId(function))
    }
}
