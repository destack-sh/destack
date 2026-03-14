use std::borrow::Cow;
use std::mem::size_of_val;
use std::sync::Arc;

use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use super::ReferenceMapId;
use crate::TreeVector;

/// One immutable managed extent image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedExtentImage {
    /// Whether this extent slot is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this extent.
    pub len: usize,
    /// The chunk width used by this extent.
    pub chunk_bytes: usize,
    /// The immutable extent chunks.
    pub(crate) chunks: TreeVector<Arc<[u8]>>,
    /// The interned reference map for this extent.
    pub trace_id: ReferenceMapId,
    /// The durable layout id for this extent, if any.
    pub layout_id: u32,
}

impl ManagedExtentImage {
    /// Report whether this extent image shares durable backing with another extent image.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        self.is_allocated == other.is_allocated
            && self.len == other.len
            && self.chunk_bytes == other.chunk_bytes
            && self.trace_id == other.trace_id
            && self.layout_id == other.layout_id
            && self.chunks.len() == other.chunks.len()
            && self
                .chunks
                .iter()
                .zip(other.chunks.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
    }
}

/// One stable managed extent identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedExtentId(u64);

impl ManagedExtentId {
    /// Create one managed extent identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the managed extent identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }
}

/// One managed extent chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ManagedExtentChunk {
    /// The chunk byte storage.
    storage: ManagedExtentChunkStorage,
}

/// One managed extent chunk storage state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ManagedExtentChunkStorage {
    /// Owned mutable bytes local to one live heap.
    Owned(Box<[u8]>),
    /// Immutable bytes shared through forked images.
    Shared(Arc<[u8]>),
}

impl ManagedExtentChunk {
    /// Create one chunk from the given bytes.
    fn new(bytes: &[u8]) -> Self {
        Self {
            storage: ManagedExtentChunkStorage::Owned(bytes.to_vec().into_boxed_slice()),
        }
    }

    /// Restore one chunk from one immutable image.
    fn from_image(bytes: Arc<[u8]>) -> Self {
        Self {
            storage: ManagedExtentChunkStorage::Shared(bytes),
        }
    }

    /// Return one borrowed chunk slice.
    fn as_slice(&self) -> &[u8] {
        match &self.storage {
            ManagedExtentChunkStorage::Owned(bytes) => bytes,
            ManagedExtentChunkStorage::Shared(bytes) => bytes,
        }
    }

    /// Return one immutable chunk image.
    fn image(&mut self) -> Arc<[u8]> {
        if let ManagedExtentChunkStorage::Owned(bytes) = &mut self.storage {
            let bytes = std::mem::take(bytes);
            self.storage = ManagedExtentChunkStorage::Shared(Arc::from(bytes));
        }

        match &self.storage {
            ManagedExtentChunkStorage::Owned(_) => unreachable!(),
            ManagedExtentChunkStorage::Shared(bytes) => bytes.clone(),
        }
    }

    /// Return mutable chunk bytes, detaching on first write.
    fn bytes_mut(&mut self) -> &mut [u8] {
        if let ManagedExtentChunkStorage::Shared(bytes) = &self.storage {
            self.storage =
                ManagedExtentChunkStorage::Owned(bytes.as_ref().to_vec().into_boxed_slice());
        }

        match &mut self.storage {
            ManagedExtentChunkStorage::Owned(bytes) => bytes.as_mut(),
            ManagedExtentChunkStorage::Shared(_) => unreachable!(),
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
            ManagedExtentChunkStorage::Owned(bytes) => size_of_val(bytes.as_ref()),
            ManagedExtentChunkStorage::Shared(bytes) => size_of_val(bytes.as_ref()),
        }
    }
}

/// One managed extent for large or dynamic allocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedExtent {
    /// Whether this extent slot is currently allocated.
    is_allocated: bool,
    /// The logical byte length of this extent.
    len: usize,
    /// The chunk width used by this extent.
    chunk_bytes: usize,
    /// The extent chunk storage.
    chunks: Vec<ManagedExtentChunk>,
    /// The interned reference map for this extent.
    trace_id: ReferenceMapId,
    /// The durable layout id for this extent, if any.
    layout_id: u32,
    /// The live mark state for this extent.
    marked: bool,
    /// The active pin count for this extent.
    pin_count: u16,
}

impl ManagedExtent {
    /// Create one managed extent from the given bytes and metadata.
    pub(crate) fn new(
        bytes: &[u8],
        chunk_bytes: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) -> Self {
        Self {
            is_allocated: true,
            len: bytes.len(),
            chunk_bytes,
            chunks: Self::chunk_storage(bytes, chunk_bytes),
            trace_id,
            layout_id: layout_id.map(LayoutId::raw).unwrap_or_default(),
            marked: false,
            pin_count: 0,
        }
    }

    /// Restore one managed extent from one immutable extent image.
    pub(crate) fn from_extent_image(image: &ManagedExtentImage) -> Self {
        Self {
            is_allocated: image.is_allocated,
            len: image.len,
            chunk_bytes: image.chunk_bytes,
            chunks: image
                .chunks
                .iter()
                .cloned()
                .map(ManagedExtentChunk::from_image)
                .collect(),
            trace_id: image.trace_id,
            layout_id: image.layout_id,
            marked: false,
            pin_count: 0,
        }
    }

    /// Return whether this extent slot is currently allocated.
    pub(crate) fn is_allocated(&self) -> bool {
        self.is_allocated
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

    /// Return the trace id for this extent.
    pub(crate) fn trace_id(&self) -> ReferenceMapId {
        self.trace_id
    }

    /// Return the layout id for this extent, if any.
    pub(crate) fn layout_id(&self) -> Option<LayoutId> {
        if self.layout_id == 0 {
            None
        } else {
            Some(LayoutId::new(self.layout_id))
        }
    }

    /// Set the layout id for this extent.
    pub(crate) fn set_layout_id(&mut self, layout_id: LayoutId) {
        self.layout_id = layout_id.raw();
    }

    /// Report whether this extent is marked.
    pub(crate) fn is_marked(&self) -> bool {
        self.marked
    }

    /// Mark this extent.
    pub(crate) fn mark(&mut self) {
        self.marked = true;
    }

    /// Clear the mark state for this extent.
    pub(crate) fn clear_mark(&mut self) {
        self.marked = false;
    }

    /// Return the active pin count for this extent.
    pub(crate) fn active_pins(&self) -> usize {
        self.pin_count as usize
    }

    /// Increment the pin count for this extent.
    pub(crate) fn pin(&mut self) -> bool {
        if !self.is_allocated {
            return false;
        }

        self.pin_count = self.pin_count.saturating_add(1);
        true
    }

    /// Decrement the pin count for this extent.
    pub(crate) fn unpin(&mut self) -> bool {
        if !self.is_allocated || self.pin_count == 0 {
            return false;
        }

        self.pin_count -= 1;
        true
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

    /// Replace the entire extent payload and metadata.
    pub(crate) fn replace(
        &mut self,
        bytes: &[u8],
        chunk_bytes: usize,
        trace_id: ReferenceMapId,
        layout_id: Option<LayoutId>,
    ) {
        self.is_allocated = true;
        self.len = bytes.len();
        self.chunk_bytes = chunk_bytes;
        self.chunks = Self::chunk_storage(bytes, chunk_bytes);
        self.trace_id = trace_id;
        self.layout_id = layout_id.map(LayoutId::raw).unwrap_or_default();
        self.marked = false;
        self.pin_count = 0;
    }

    /// Free this extent slot while keeping its stable id alive.
    pub(crate) fn free(&mut self) {
        self.is_allocated = false;
        self.len = 0;
        self.chunk_bytes = 0;
        self.chunks.clear();
        self.trace_id = ReferenceMapId::new(0);
        self.layout_id = 0;
        self.marked = false;
        self.pin_count = 0;
    }

    /// Capture one immutable extent image.
    pub(crate) fn extent_image(&mut self) -> ManagedExtentImage {
        let chunk_images = self
            .chunks
            .iter_mut()
            .map(ManagedExtentChunk::image)
            .collect::<Vec<_>>();

        ManagedExtentImage {
            is_allocated: self.is_allocated,
            len: self.len,
            chunk_bytes: self.chunk_bytes,
            chunks: TreeVector::from_shared(&chunk_images, None),
            trace_id: self.trace_id,
            layout_id: self.layout_id,
        }
    }

    /// Return the retained heap bytes for this extent.
    pub(crate) fn retained_bytes(&self) -> usize {
        let mut retained_bytes = self.chunks.capacity() * std::mem::size_of::<ManagedExtentChunk>();

        for chunk in &self.chunks {
            retained_bytes += chunk.retained_bytes();
        }

        retained_bytes
    }

    // split one extent payload into chunk-backed storage
    fn chunk_storage(bytes: &[u8], chunk_bytes: usize) -> Vec<ManagedExtentChunk> {
        bytes
            .chunks(chunk_bytes.max(1))
            .map(ManagedExtentChunk::new)
            .collect()
    }
}
