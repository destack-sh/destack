use tspp_bytecode::{Instruction, Opcode};
use tspp_program::{FunctionId, Runtime, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one function value operation.
    pub(crate) fn execute_function(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);

        match instruction.opcode() {
            Opcode::FUNCTION_ADDRESS => {
                let target = operands.register()?;
                let function = FunctionId(operands.u32()?);

                self.write(target.0, function.into());
            }
            Opcode::FUNCTION_BIND => {
                let target = operands.span()?;
                let function = FunctionId(operands.u32()?);
                let environment = operands.register()?;
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.start.0, function.into());
                self.write(target.start.0 + 1, self.read(environment.0));
            }
            _ => unreachable!("function dispatch selects one function opcode"),
        }

        Ok(())
    }

    /// Decode one function id stored in a callable word.
    pub(crate) fn function_id(&self, word: Word) -> Result<FunctionId> {
        FunctionId::from_word(word).ok_or_else(|| self.invalid_instruction())
    }
}
