use destack_bytecode::{Instruction, Opcode};
use destack_program::{TypeId, Word};

use crate::diagnostic::{Error, Result, Trap};
use crate::machine::Activation;

impl Activation<'_, '_> {
    /// Execute one slice descriptor operation.
    pub(crate) fn execute_slice(&mut self, instruction: Instruction<'_>) -> Result<()> {
        match instruction.opcode() {
            Opcode::SLICE_VIEW => self.execute_slice_view(instruction),
            Opcode::SLICE_LENGTH => self.execute_slice_length(instruction),
            _ => unreachable!("slice dispatch selects one slice opcode"),
        }
    }

    /// Form one checked subview over a contiguous slice.
    fn execute_slice_view(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands.range().map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        let element = TypeId(operands.u32().map_err(|_| self.invalid_instruction())?);
        let start = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let length = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        if target.word_count != 2 || source.word_count != 2 {
            return Err(self.invalid_instruction());
        }

        // require the requested element range to fit inside the source slice
        let source_length = self.read(source.start.0 + 1).as_u64();
        let start = self.read(start.0).as_u64();
        let length = self.read(length.0).as_u64();
        if start > source_length || length > source_length - start {
            return Err(Error::trap(Trap::Bounds));
        }

        // advance the stable reference by the linked element layout width
        let element_byte_len = self
            .machine
            .program
            .type_byte_len(element)
            .ok_or_else(|| self.invalid_instruction())?;
        let byte_offset = usize::try_from(start)
            .ok()
            .and_then(|start| start.checked_mul(element_byte_len))
            .ok_or_else(|| Error::trap(Trap::Bounds))?;
        let reference = self.read(source.start.0).bits() as usize;
        let reference = reference
            .checked_add(byte_offset)
            .ok_or_else(|| Error::trap(Trap::Bounds))?;

        self.write(target.start.0, Word::from_bits(reference as u64));
        self.write(target.start.0 + 1, Word::uint64(length));

        Ok(())
    }

    /// Read the element count from one slice descriptor.
    fn execute_slice_length(&mut self, instruction: Instruction<'_>) -> Result<()> {
        let mut operands = instruction.operands();
        let target = operands
            .register()
            .map_err(|_| self.invalid_instruction())?;
        let source = operands.range().map_err(|_| self.invalid_instruction())?;
        if source.word_count != 2 {
            return Err(self.invalid_instruction());
        }

        self.write(target.0, self.read(source.start.0 + 1));

        Ok(())
    }
}
