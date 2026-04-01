use serde::{Deserialize, Serialize};

use destack_heap::Value;
use destack_mir as mir;

/// One frame-local stack allocation owned by the interpreter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StackAllocation {
    /// The raw byte storage for this allocation.
    bytes: Vec<u8>,
    /// The stored raw storage type.
    storage_type: mir::LocalNodeId<mir::Type>,
}

impl StackAllocation {
    /// Create one zeroed stack allocation with the given byte length.
    pub(crate) fn new(byte_len: usize, storage_type: mir::LocalNodeId<mir::Type>) -> Self {
        Self {
            bytes: vec![0u8; byte_len],
            storage_type,
        }
    }

    /// Create one stack allocation from captured bytes.
    pub(crate) fn from_bytes(bytes: Vec<u8>, storage_type: mir::LocalNodeId<mir::Type>) -> Self {
        Self {
            bytes,
            storage_type,
        }
    }

    /// Borrow the stack allocation bytes.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Borrow the stack allocation bytes mutably.
    pub(crate) fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.bytes
    }

    /// Return the allocation byte length.
    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Return the compiled storage type for this allocation.
    pub(crate) fn storage_type(&self) -> mir::LocalNodeId<mir::Type> {
        self.storage_type
    }

    /// Clone the allocation bytes into one owned buffer.
    pub(crate) fn clone_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

/// Resize a stack and clear the active range.
#[allow(clippy::uninit_vec)]
pub(crate) fn resize_and_clear_stack(stack: &mut Vec<Value>, base: usize, end: usize) {
    // validate bounds
    debug_assert!(base <= end, "stack range out of bounds: {base}..{end}");

    // resize without redundant initialization
    if end > stack.len() {
        let additional = end - stack.len();
        stack.reserve(additional);

        // safety: fill the new range immediately
        unsafe {
            stack.set_len(end);
        }
    } else {
        stack.truncate(end);
    }

    // clear active stack slots
    stack[base..end].fill(Value::VOID);
}
