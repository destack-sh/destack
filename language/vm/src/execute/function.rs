use destack_bytecode::{Instruction, Opcode};
use destack_program::{FunctionId, Runtime, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one function value operation.
    pub(crate) fn execute_function(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = self.operands(instruction);

        match instruction.opcode() {
            Opcode::FUNCTION_ADDRESS => {
                let target = operands.register()?;
                let function = operands.u32()?;

                self.write(target.0, Word::from_bits(function as u64));
            }
            Opcode::FUNCTION_BIND => {
                let target = operands.span()?;
                let function = operands.u32()?;
                let environment = operands.register()?;
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.start.0, Word::from_bits(function as u64));
                self.write(target.start.0 + 1, self.read(environment.0));
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
