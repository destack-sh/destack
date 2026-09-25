use tspp_bytecode::{BooleanOperation, Instruction};
use tspp_program::{Runtime, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one boolean operation.
    #[inline(always)]
    pub(crate) fn execute_boolean(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let operation = instruction
            .opcode()
            .boolean_operation()
            .ok_or_else(|| self.invalid_instruction())?;
        let mut operands = self.operands(instruction);
        let target = operands.register()?;
        let left = operands.register()?;
        let left = self.read(left.0).as_boolean();

        // execute the unary or binary form
        let value = if operation == BooleanOperation::Not {
            !left
        } else {
            let right = operands.register()?;
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
