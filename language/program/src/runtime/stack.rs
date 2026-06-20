use serde::{Deserialize, Serialize};

/// Durable stack image captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackImage {
    /// The captured stack bytes.
    pub bytes: Vec<u8>,
}

impl StackImage {
    /// Create one empty stack image.
    pub fn empty() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Return the captured byte length.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Return whether this stack image has no captured bytes.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Append one frame byte range and return its stack offset.
    pub fn push_frame(&mut self, bytes: &[u8]) -> usize {
        let stack_offset = self.bytes.len();
        self.bytes.extend_from_slice(bytes);

        stack_offset
    }

    /// Return one captured frame byte range.
    pub fn frame_bytes(&self, stack_offset: usize, byte_len: usize) -> Option<&[u8]> {
        let end = stack_offset.checked_add(byte_len)?;

        self.bytes.get(stack_offset..end)
    }

    /// Return one captured frame byte range mutably.
    pub fn frame_bytes_mut(&mut self, stack_offset: usize, byte_len: usize) -> Option<&mut [u8]> {
        let end = stack_offset.checked_add(byte_len)?;

        self.bytes.get_mut(stack_offset..end)
    }
}
