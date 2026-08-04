use std::mem::size_of;

use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::FunctionEmitter;

impl<'a> FunctionEmitter<'a> {
    /// Emit one contiguous slice subview.
    pub(super) fn emit_slice_view(
        &mut self,
        destination: mir::Value,
        source: mir::Value,
        start: mir::Value,
        length: mir::Value,
    ) -> Result<(), EmitError> {
        let source_type = self.value_type(source)?;
        let mir::Type::Slice { element, .. } = self.optimized.tree.get(source_type) else {
            return Err(self.internal("slice view source is not a slice"));
        };
        let stride = self.types.byte_len(*element)?;
        let mut instruction = bytecode::InstructionBuilder::new(bytecode::Opcode::SLICE_VIEW);
        instruction.span(self.register(source)?);
        instruction.u32(stride);
        instruction.register(self.word(start)?);
        instruction.register(self.word(length)?);
        let destination = self.register(destination)?;

        self.encode(instruction, &[destination])
    }

    /// Emit one slice length projection.
    pub(super) fn emit_slice_length(
        &mut self,
        destination: mir::Value,
        slice: mir::Value,
    ) -> Result<(), EmitError> {
        let length_byte_offset = size_of::<u64>() as u32;
        let length_byte_len = size_of::<u64>() as u32;

        self.emit_extract(destination, slice, length_byte_offset, length_byte_len)
    }
}
