use destack_bytecode::{Instruction, Opcode};
use destack_program::{DynamicTableId, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one dynamic value operation.
    pub(crate) fn execute_dynamic(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();

        match instruction.opcode() {
            Opcode::DYNAMIC_BIND => {
                let target = operands.range().map_err(|_| self.invalid_instruction())?;
                let payload = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let table = DynamicTableId(operands.u32().map_err(|_| self.invalid_instruction())?);
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.start.0, self.read(payload.0));
                self.write(target.start.0 + 1, Word::from(table));
            }
            Opcode::DYNAMIC_PAYLOAD => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let dynamic = operands.range().map_err(|_| self.invalid_instruction())?;
                if dynamic.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.0, self.read(dynamic.start.0));
            }
            Opcode::DYNAMIC_TYPE => {
                let target = operands
                    .register()
                    .map_err(|_| self.invalid_instruction())?;
                let dynamic = operands.range().map_err(|_| self.invalid_instruction())?;
                if dynamic.word_count != 2 {
                    return Err(self.invalid_instruction());
                }
                let table = DynamicTableId::from(self.read(dynamic.start.0 + 1));
                let table = self
                    .machine
                    .program
                    .dynamic_table(table)
                    .ok_or_else(|| self.invalid_instruction())?;

                self.write(target.0, Word::from_bits(table.concrete.0 as u64));
            }
            _ => unreachable!("dynamic dispatch selects one dynamic opcode"),
        }

        Ok(())
    }
}
