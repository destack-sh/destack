use std::ptr;

use tspp_bytecode::{Address, Instruction, Opcode};
use tspp_program::{DynamicTableId, MemoryAccess, Runtime, Word};

use crate::diagnostic::Result;
use crate::machine::Activation;

impl<R: Runtime + ?Sized> Activation<'_, '_, R> {
    /// Execute one dynamic value operation.
    pub(crate) fn execute_dynamic(
        &mut self,
        instruction: Instruction<'_>,
    ) -> Result<Option<(MemoryAccess, (usize, usize))>> {
        let mut operands = self.operands(instruction);

        let memory = match instruction.opcode() {
            Opcode::DYNAMIC_BIND => {
                let target = operands.span()?;
                let payload = operands.register()?;
                let table = DynamicTableId(operands.u32()?);
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                self.write(target.start.0, self.read(payload.0));
                self.write(target.start.0 + 1, Word::from(table));

                None
            }
            Opcode::DYNAMIC_READ => {
                let target = operands.span()?;
                let dynamic = operands.span()?;
                let slot = operands.u16()?;
                let byte_len = operands.u32()? as usize;
                if dynamic.word_count != 2 {
                    return Err(self.invalid_instruction());
                }
                let target = self.register_byte_range(target)?;
                if byte_len > target.len() {
                    return Err(self.invalid_instruction());
                }

                // resolve the selected field through the linked dynamic row
                let table = DynamicTableId::from(self.read(dynamic.start.0 + 1));
                let offset = self
                    .machine
                    .program
                    .dynamic_entry(table, u32::from(slot))
                    .and_then(|entry| entry.field_offset_value())
                    .ok_or_else(|| self.invalid_instruction())?;
                let source = self.resolve_address(dynamic.start.0, Address::Reference)?;
                let source = source + offset as usize;

                // copy the exact field bytes before clearing trailing register padding
                let target_address = self.fiber.stack.address(target.start) as *mut u8;
                // SAFETY: linked field rows and the result width describe one live field
                unsafe { ptr::copy(source as *const u8, target_address, byte_len) };
                self.fiber
                    .stack
                    .zero(target.start + byte_len, target.len() - byte_len)?;

                Some((MemoryAccess::Read, (source, byte_len)))
            }
            Opcode::DYNAMIC_TYPE => {
                let target = operands.register()?;
                let dynamic = operands.span()?;
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

                None
            }
            _ => unreachable!("dynamic dispatch selects one dynamic opcode"),
        };

        Ok(memory)
    }
}
