use crate::{CodeLength, CodeOffset, CodePointer};

/// Loaded code address range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeMapping {
    /// The base address.
    pub base: CodePointer,
    /// The byte length.
    pub byte_length: CodeLength,
}

impl CodeMapping {
    /// Return one address inside this loaded code range.
    pub fn pointer_at(self, offset: CodeOffset) -> Option<CodePointer> {
        let offset = offset.0 as usize;
        let byte_length = self.byte_length.0 as usize;
        if offset >= byte_length {
            return None;
        }

        self.base
            .address()
            .checked_add(offset)
            .map(CodePointer::from_address)
    }
}
