use std::borrow::Cow;
use std::mem::size_of_val;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::TreeVector;

/// One immutable raw extent image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExtentImage {
    /// Whether this extent slot is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this extent.
    pub len: usize,
    /// The chunk width used by this extent.
    pub chunk_bytes: usize,
    /// The immutable extent chunks.
    pub(crate) chunks: TreeVector<Arc<[u8]>>,
}

impl RawExtentImage {
    /// Report whether this extent image shares durable backing with another extent image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.is_allocated == other.is_allocated
            && self.len == other.len
            && self.chunk_bytes == other.chunk_bytes
            && self.chunks.len() == other.chunks.len()
            && self
                .chunks
                .iter()
                .zip(other.chunks.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
    }
}

/// One stable raw extent identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExtentId(u64);

impl RawExtentId {
    /// Create one raw extent identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw extent identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }
}

/// One raw extent chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawExtentChunk {
    /// The chunk byte storage.
    storage: RawExtentChunkStorage,
}

/// One raw extent chunk storage state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RawExtentChunkStorage {
    /// Owned mutable bytes local to one live heap.
    Owned(Box<[u8]>),
    /// Immutable bytes shared through forked images.
    Shared(Arc<[u8]>),
}

impl RawExtentChunk {
    /// Create one chunk from the given bytes.
    fn new(bytes: &[u8]) -> Self {
        Self {
            storage: RawExtentChunkStorage::Owned(bytes.to_vec().into_boxed_slice()),
        }
    }

    /// Restore one chunk from one immutable image.
    fn from_image(bytes: Arc<[u8]>) -> Self {
        Self {
            storage: RawExtentChunkStorage::Shared(bytes),
        }
    }

    /// Return one borrowed chunk slice.
    fn as_slice(&self) -> &[u8] {
        match &self.storage {
            RawExtentChunkStorage::Owned(bytes) => bytes,
            RawExtentChunkStorage::Shared(bytes) => bytes,
        }
    }

    /// Return one immutable chunk image.
    fn image(&mut self) -> Arc<[u8]> {
        if let RawExtentChunkStorage::Owned(bytes) = &mut self.storage {
            let bytes = std::mem::take(bytes);
            self.storage = RawExtentChunkStorage::Shared(Arc::from(bytes));
        }

        match &self.storage {
            RawExtentChunkStorage::Owned(_) => unreachable!(),
            RawExtentChunkStorage::Shared(bytes) => bytes.clone(),
        }
    }

    /// Return mutable chunk bytes, detaching on first write.
    fn bytes_mut(&mut self) -> &mut [u8] {
        if let RawExtentChunkStorage::Shared(bytes) = &self.storage {
            self.storage = RawExtentChunkStorage::Owned(bytes.as_ref().to_vec().into_boxed_slice());
        }

        match &mut self.storage {
            RawExtentChunkStorage::Owned(bytes) => bytes.as_mut(),
            RawExtentChunkStorage::Shared(_) => unreachable!(),
        }
    }

    /// Write one byte by index.
    fn set(&mut self, index: usize, byte: u8) -> bool {
        let bytes = self.bytes_mut();
        let Some(slot) = bytes.get_mut(index) else {
            return false;
        };

        *slot = byte;
        true
    }

    /// Return the retained bytes owned by this chunk.
    fn retained_bytes(&self) -> usize {
        match &self.storage {
            RawExtentChunkStorage::Owned(bytes) => size_of_val(bytes.as_ref()),
            RawExtentChunkStorage::Shared(bytes) => size_of_val(bytes.as_ref()),
        }
    }
}

/// One raw extent for large or dynamic allocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawExtent {
    /// Whether this extent slot is currently allocated.
    is_allocated: bool,
    /// The logical byte length of this extent.
    len: usize,
    /// The chunk width used by this extent.
    chunk_bytes: usize,
    /// The extent chunk storage.
    chunks: Vec<RawExtentChunk>,
}

impl RawExtent {
    /// Create one raw extent from the given bytes.
    pub(crate) fn new(bytes: &[u8], chunk_bytes: usize) -> Self {
        Self {
            is_allocated: true,
            len: bytes.len(),
            chunk_bytes,
            chunks: Self::chunk_storage(bytes, chunk_bytes),
        }
    }

    /// Restore one raw extent from one immutable extent image.
    pub(crate) fn from_extent_image(image: &RawExtentImage) -> Self {
        Self {
            is_allocated: image.is_allocated,
            len: image.len,
            chunk_bytes: image.chunk_bytes,
            chunks: image
                .chunks
                .iter()
                .cloned()
                .map(RawExtentChunk::from_image)
                .collect(),
        }
    }

    /// Return the extent byte length.
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Return the extent bytes as borrowed or materialized storage.
    pub(crate) fn bytes(&self) -> Cow<'_, [u8]> {
        if self.len == 0 {
            return Cow::Borrowed(&[]);
        }

        if self.chunks.len() == 1 {
            let bytes = self.chunks[0].as_slice();
            return Cow::Borrowed(&bytes[..self.len]);
        }

        let mut bytes = Vec::with_capacity(self.len);

        for chunk in &self.chunks {
            bytes.extend_from_slice(chunk.as_slice());
        }

        bytes.truncate(self.len);
        Cow::Owned(bytes)
    }

    /// Write one byte by index.
    pub(crate) fn set(&mut self, index: usize, byte: u8) -> bool {
        if index >= self.len {
            return false;
        }

        let chunk_index = index / self.chunk_bytes;
        let chunk_offset = index % self.chunk_bytes;
        let Some(chunk) = self.chunks.get_mut(chunk_index) else {
            return false;
        };

        chunk.set(chunk_offset, byte)
    }

    /// Replace the entire extent payload.
    pub(crate) fn replace(&mut self, bytes: &[u8], chunk_bytes: usize) {
        self.is_allocated = true;
        self.len = bytes.len();
        self.chunk_bytes = chunk_bytes;
        self.chunks = Self::chunk_storage(bytes, chunk_bytes);
    }

    /// Free this extent slot while keeping its stable id alive.
    pub(crate) fn free(&mut self) {
        self.is_allocated = false;
        self.len = 0;
        self.chunk_bytes = 0;
        self.chunks.clear();
    }

    /// Capture one immutable chunk image sequence.
    pub(crate) fn extent_image(&mut self) -> RawExtentImage {
        let chunk_images = self
            .chunks
            .iter_mut()
            .map(RawExtentChunk::image)
            .collect::<Vec<_>>();

        RawExtentImage {
            is_allocated: self.is_allocated,
            len: self.len,
            chunk_bytes: self.chunk_bytes,
            chunks: TreeVector::from_shared(&chunk_images, None),
        }
    }

    /// Return the retained heap bytes for this extent.
    pub(crate) fn retained_bytes(&self) -> usize {
        let mut retained_bytes = self.chunks.capacity() * std::mem::size_of::<RawExtentChunk>();

        for chunk in &self.chunks {
            retained_bytes += chunk.retained_bytes();
        }

        retained_bytes
    }

    // split one extent payload into chunk-backed storage
    fn chunk_storage(bytes: &[u8], chunk_bytes: usize) -> Vec<RawExtentChunk> {
        bytes
            .chunks(chunk_bytes.max(1))
            .map(RawExtentChunk::new)
            .collect()
    }
}
