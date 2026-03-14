use std::mem::size_of_val;
use std::sync::Arc;

/// One shared-memory chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SharedChunk {
    /// The byte storage for this chunk.
    storage: SharedChunkStorage,
}

/// The backing storage for one shared-memory chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SharedChunkStorage {
    /// Owned mutable chunk bytes local to one live world.
    Owned(Box<[u8]>),
    /// Immutable chunk bytes shared through captured images.
    Shared(Arc<[u8]>),
}

impl SharedChunk {
    /// Create one chunk from the given bytes.
    pub(super) fn new(bytes: &[u8]) -> Self {
        Self {
            storage: SharedChunkStorage::Owned(bytes.to_vec().into_boxed_slice()),
        }
    }

    /// Restore one chunk from one immutable image.
    pub(super) fn from_image(bytes: Arc<[u8]>) -> Self {
        Self {
            storage: SharedChunkStorage::Shared(bytes),
        }
    }

    /// Return the chunk bytes as one slice.
    pub(super) fn as_slice(&self) -> &[u8] {
        match &self.storage {
            SharedChunkStorage::Owned(bytes) => bytes,
            SharedChunkStorage::Shared(bytes) => bytes,
        }
    }

    /// Return one byte at the given index.
    pub(super) fn get(&self, index: usize) -> Option<u8> {
        self.as_slice().get(index).copied()
    }

    /// Return one immutable chunk image.
    pub(super) fn image(&mut self) -> Arc<[u8]> {
        if let SharedChunkStorage::Owned(bytes) = &mut self.storage {
            let bytes = std::mem::take(bytes);
            self.storage = SharedChunkStorage::Shared(Arc::from(bytes));
        }

        match &self.storage {
            SharedChunkStorage::Owned(_) => unreachable!(),
            SharedChunkStorage::Shared(bytes) => bytes.clone(),
        }
    }

    /// Return mutable chunk storage, detaching one shared image on first write.
    pub(super) fn bytes_mut(&mut self) -> &mut [u8] {
        if let SharedChunkStorage::Shared(bytes) = &self.storage {
            self.storage = SharedChunkStorage::Owned(bytes.as_ref().to_vec().into_boxed_slice());
        }

        match &mut self.storage {
            SharedChunkStorage::Owned(bytes) => bytes.as_mut(),
            SharedChunkStorage::Shared(_) => unreachable!(),
        }
    }

    /// Write one byte at the given index.
    pub(super) fn set(&mut self, index: usize, byte: u8) -> bool {
        let bytes = self.bytes_mut();
        let Some(slot) = bytes.get_mut(index) else {
            return false;
        };

        *slot = byte;
        true
    }

    /// Return the retained bytes owned by this chunk.
    pub(super) fn retained_bytes(&self) -> usize {
        match &self.storage {
            SharedChunkStorage::Owned(bytes) => size_of_val(bytes.as_ref()),
            SharedChunkStorage::Shared(bytes) => size_of_val(bytes.as_ref()),
        }
    }
}
