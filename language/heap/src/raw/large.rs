use std::borrow::Cow;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::alloc::{ChunkPayload, PageArena};

/// One immutable raw large-allocation image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawLargeAllocationImage {
    /// Whether this large-allocation slot is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this large allocation.
    pub len: usize,
    /// The page width used by this large allocation.
    pub page_bytes: usize,
    /// The immutable large-allocation chunks.
    pub(crate) chunks: Vec<Rc<[u8]>>,
}

impl RawLargeAllocationImage {
    /// Report whether this large-allocation image shares durable backing with another large-allocation image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.is_allocated == other.is_allocated
            && self.len == other.len
            && self.page_bytes == other.page_bytes
            && self.chunks.len() == other.chunks.len()
            && self
                .chunks
                .iter()
                .zip(other.chunks.iter())
                .all(|(left, right)| Rc::ptr_eq(left, right))
    }
}

/// One stable raw large-allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawLargeAllocationId(u64);

impl RawLargeAllocationId {
    /// Create one raw large-allocation identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw large-allocation identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }
}

/// One raw large allocation for large or dynamic payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawLargeAllocation {
    /// Whether this large-allocation slot is currently allocated.
    is_allocated: bool,
    /// The raw payload storage.
    payload: ChunkPayload,
}

impl RawLargeAllocation {
    /// Create one raw large allocation from the given bytes.
    pub(crate) fn new(bytes: &[u8], page_bytes: usize, page_arena: &mut PageArena) -> Self {
        Self {
            is_allocated: true,
            payload: ChunkPayload::new(bytes, page_bytes, page_arena),
        }
    }

    /// Create one zeroed raw large allocation with the given byte length.
    pub(crate) fn new_zeroed(
        byte_len: usize,
        page_bytes: usize,
        page_arena: &mut PageArena,
    ) -> Self {
        Self {
            is_allocated: true,
            payload: ChunkPayload::new_zeroed(byte_len, page_bytes, page_arena),
        }
    }

    /// Restore one raw large allocation from one immutable large-allocation image.
    pub(crate) fn from_large_allocation_image(image: &RawLargeAllocationImage) -> Self {
        Self {
            is_allocated: image.is_allocated,
            payload: ChunkPayload::from_image(
                image.len,
                image.page_bytes,
                image.chunks.iter().cloned(),
            ),
        }
    }

    /// Return the large-allocation bytes as borrowed or materialized storage.
    pub(crate) fn bytes<'a>(&'a self, page_arena: &'a PageArena) -> Cow<'a, [u8]> {
        self.payload.bytes(page_arena)
    }

    /// Write one byte by index.
    pub(crate) fn set(&mut self, page_arena: &mut PageArena, index: usize, byte: u8) -> bool {
        self.payload.set(page_arena, index, byte)
    }

    /// Replace the entire large-allocation payload.
    pub(crate) fn replace(&mut self, page_arena: &mut PageArena, bytes: &[u8], page_bytes: usize) {
        self.is_allocated = true;
        self.payload.replace(page_arena, bytes, page_bytes);
    }

    /// Replace the entire large-allocation payload with zeroed bytes.
    pub(crate) fn replace_zeroed(
        &mut self,
        page_arena: &mut PageArena,
        byte_len: usize,
        page_bytes: usize,
    ) {
        self.is_allocated = true;
        self.payload
            .replace_zeroed(page_arena, byte_len, page_bytes);
    }

    /// Free this large-allocation slot while keeping its stable id alive.
    pub(crate) fn free(&mut self, page_arena: &mut PageArena) {
        self.is_allocated = false;
        self.payload.clear(page_arena);
    }

    /// Write one byte slice by range.
    pub(crate) fn set_bytes(
        &mut self,
        page_arena: &mut PageArena,
        start: usize,
        bytes: &[u8],
    ) -> bool {
        self.payload.write_window(page_arena, start, bytes)
    }

    /// Capture one immutable chunk image sequence.
    pub(crate) fn large_allocation_image(
        &mut self,
        page_arena: &mut PageArena,
    ) -> RawLargeAllocationImage {
        RawLargeAllocationImage {
            is_allocated: self.is_allocated,
            len: self.payload.len(),
            page_bytes: self.payload.page_bytes(),
            chunks: self.payload.image(page_arena),
        }
    }

    /// Return the retained heap bytes for this large allocation.
    pub(crate) fn retained_bytes(&self) -> usize {
        self.payload.retained_bytes()
    }
}
