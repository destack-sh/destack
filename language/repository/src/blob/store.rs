use std::collections::hash_map;
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::{ptr, slice};

use destack_core::{Blob, BlobId, BlobMemory};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::BlobStoreError;

/// Immutable bytes shared by repositories on one host.
#[derive(Debug, Default)]
pub struct BlobStore {
    /// Retained bytes by exact identity.
    blobs: RwLock<FxHashMap<BlobId, Arc<BlobMemory>>>,
}

impl BlobStore {
    /// Create an empty Blob store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Retain one exact byte stream.
    pub fn retain(&self, input: &mut dyn Read) -> Result<Blob, BlobStoreError> {
        let mut writer = self.writer();
        io::copy(input, &mut writer)?;

        writer.commit()
    }

    /// Create one incremental Blob writer.
    pub fn writer(&self) -> BlobWriter<'_> {
        BlobWriter::new(self)
    }

    /// Open one retained Blob.
    pub fn open(&self, blob: Blob) -> Result<Arc<BlobMemory>, BlobStoreError> {
        let memory = self
            .blobs
            .read()
            .get(&blob.id)
            .cloned()
            .ok_or(BlobStoreError::Missing { blob })?;

        Self::verify(blob, memory)
    }

    /// Return whether one exact Blob is retained.
    pub fn contains(&self, blob: Blob) -> Result<bool, BlobStoreError> {
        let Some(memory) = self.blobs.read().get(&blob.id).cloned() else {
            return Ok(false);
        };

        Self::verify(blob, memory).map(|_| true)
    }

    /// Retain one immutable byte allocation by exact identity.
    fn insert(&self, memory: Arc<BlobMemory>) -> Result<(), BlobStoreError> {
        let blob = memory.blob();
        let mut blobs = self.blobs.write();
        match blobs.entry(blob.id) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(memory);
            }
            hash_map::Entry::Occupied(entry) if entry.get().blob() == blob => {}
            hash_map::Entry::Occupied(entry) => {
                return Err(BlobStoreError::Corrupt {
                    expected: blob,
                    actual: entry.get().blob(),
                });
            }
        }

        Ok(())
    }

    /// Validate one retained Blob allocation.
    fn verify(blob: Blob, memory: Arc<BlobMemory>) -> Result<Arc<BlobMemory>, BlobStoreError> {
        let actual = memory.blob();
        if actual.byte_len != blob.byte_len {
            return Err(BlobStoreError::Length {
                blob,
                actual: actual.byte_len,
            });
        }
        if actual.id != blob.id {
            return Err(BlobStoreError::Corrupt {
                expected: blob,
                actual,
            });
        }

        Ok(memory)
    }
}

/// Incremental writer for one unpublished Blob.
#[derive(Debug)]
pub struct BlobWriter<'a> {
    /// Destination Blob store.
    store: &'a BlobStore,
    /// Bytes written so far.
    bytes: Vec<u8>,
}

impl<'a> BlobWriter<'a> {
    /// Create one unpublished Blob writer.
    fn new(store: &'a BlobStore) -> Self {
        Self {
            store,
            bytes: Vec::new(),
        }
    }

    /// Publish the complete Blob.
    pub fn commit(self) -> Result<Blob, BlobStoreError> {
        let memory = Arc::new(AlignedBytes::new(&self.bytes));
        let memory = Arc::new(BlobMemory::from_shared(memory));
        let blob = memory.blob();
        self.store.insert(memory)?;

        Ok(blob)
    }
}

impl Write for BlobWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buffer);

        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Aligned immutable Blob bytes.
#[derive(Debug)]
struct AlignedBytes {
    /// Aligned backing chunks.
    chunks: Box<[u128]>,
    /// Initialized byte length.
    byte_len: usize,
}

impl AlignedBytes {
    /// Copy exact bytes into aligned immutable memory.
    fn new(bytes: &[u8]) -> Self {
        let chunk_bytes = size_of::<u128>();
        let chunk_count = bytes.len().div_ceil(chunk_bytes);
        let mut chunks = vec![0_u128; chunk_count].into_boxed_slice();
        let destination = chunks.as_mut_ptr().cast::<u8>();

        // safety: chunks contain at least bytes.len() writable bytes
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len());
        }

        Self {
            chunks,
            byte_len: bytes.len(),
        }
    }
}

impl AsRef<[u8]> for AlignedBytes {
    fn as_ref(&self) -> &[u8] {
        let bytes = self.chunks.as_ptr().cast::<u8>();

        // safety: byte_len never exceeds the initialized backing chunks
        unsafe { slice::from_raw_parts(bytes, self.byte_len) }
    }
}
