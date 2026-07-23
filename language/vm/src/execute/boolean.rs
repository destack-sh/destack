use destack_bytecode::{BooleanOperation, Instruction};
use destack_program::Word;

use crate::diagnostic::Result;
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one boolean operation.
    pub(crate) fn execute_boolean(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let operation = instruction
            .opcode()
            .boolean_operation()
            .ok_or_else(|| self.invalid_instruction())?;
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let left = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let left = self.read(left.0).as_boolean();

        // execute the unary or binary form
        let value = if operation == BooleanOperation::Not {
            !left
        } else {
            let right = operands
                .register()
                .map_err(|_| self.invalid_instruction())?;
            let right = self.read(right.0).as_boolean();

            match operation {
                BooleanOperation::And => left & right,
                BooleanOperation::Or => left | right,
                BooleanOperation::Xor => left ^ right,
                BooleanOperation::Not => unreachable!(),
            }
        };

        self.write(target.0, Word::boolean(value));

        Ok(())
    }
}
