use std::ptr;

use tspp_bytecode::{Address, Instruction, Opcode};
use tspp_mir::VIRTUAL_TABLE_ID_OFFSET;
use tspp_program::{DynamicTableId, MemoryAccess, Runtime, VirtualTableId, Word};

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
            Opcode::DYNAMIC_BIND_VIRTUAL => {
                let target = operands.span()?;
                let payload = operands.register()?;
                let slot = operands.u16()?;
                if target.word_count != 2 {
                    return Err(self.invalid_instruction());
                }

                // bind the table the runtime class's conformance slot holds
                let class = self.virtual_table(payload.0)?;
                let table = self
                    .machine
                    .program
                    .virtual_dynamic_table(class, u32::from(slot))
                    .ok_or_else(|| self.invalid_instruction())?;
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
            Opcode::TYPE_OF_DYNAMIC => {
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
            Opcode::TYPE_OF_OBJECT => {
                let target = operands.register()?;
                let object = operands.register()?;

                // read the object's runtime class
                let table = self.virtual_table(object.0)?;
                let table = self
                    .machine
                    .program
                    .virtual_table(table)
                    .ok_or_else(|| self.invalid_instruction())?;

                self.write(target.0, Word::from_bits(table.concrete.0 as u64));

                None
            }
            _ => unreachable!("dynamic dispatch selects one dynamic opcode"),
        };

        Ok(memory)
    }

    /// Read the virtual table id of the class object one register references.
    pub(crate) fn virtual_table(&self, register: u16) -> Result<VirtualTableId> {
        let object = self.resolve_address(register, Address::Reference)?;
        let address = object + VIRTUAL_TABLE_ID_OFFSET as usize;

        // SAFETY: linked class objects lead with an aligned virtual table id
        let table = unsafe { ptr::read(address as *const u32) };

        Ok(VirtualTableId(table))
    }
}
