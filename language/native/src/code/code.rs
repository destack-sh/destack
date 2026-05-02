use crate::{CodeLen, CodeOffset, CodePointer};

/// Loaded code address range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeMapping {
    /// The base address.
    pub base: CodePointer,
    /// The byte length.
    pub byte_len: CodeLen,
}

impl CodeMapping {
    /// Return one address inside this loaded code range.
    pub fn pointer_at(self, offset: CodeOffset) -> Option<CodePointer> {
        let offset = offset.0 as usize;
        let byte_len = self.byte_len.0 as usize;
        if offset >= byte_len {
            return None;
        }

        self.base
            .address()
            .checked_add(offset)
            .map(CodePointer::from_address)
    }
}
