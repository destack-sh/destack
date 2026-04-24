use serde::{Deserialize, Serialize};

use destack_mir as mir;

/// One frame-local stack allocation owned by the interpreter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StackAllocation {
    /// The raw byte storage for this allocation.
    bytes: Vec<u8>,
    /// The stored raw type.
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

    /// Return the compiled type for this allocation.
    pub(crate) fn storage_type(&self) -> mir::LocalNodeId<mir::Type> {
        self.storage_type
    }

    /// Clone the allocation bytes into one owned buffer.
    pub(crate) fn clone_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}
