use std::collections::hash_map;
use std::io::{self, Cursor, Read, Write};
use std::ops::Range;
use std::sync::Arc;
use std::{ptr, slice};

use destack_core::{Blob, BlobId, BlobMemory};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use super::{BlobStore, BlobStoreError, BlobWriter};

/// BlobStore backed by process-local memory.
#[derive(Debug, Default)]
pub struct MemoryBlobStore {
    /// Immutable bytes by exact identity.
    blobs: RwLock<FxHashMap<BlobId, Arc<BlobMemory>>>,
}

/// Incremental writer into one memory BlobStore.
#[derive(Debug)]
struct MemoryBlobWriter<'a> {
    /// Destination BlobStore.
    store: &'a MemoryBlobStore,
    /// Bytes written so far.
    bytes: Vec<u8>,
}

/// Aligned immutable Blob memory.
#[derive(Debug)]
struct Memory {
    /// Aligned backing chunks.
    chunks: Box<[u128]>,
    /// Initialized byte length.
    byte_len: usize,
}

impl MemoryBlobStore {
    /// Create one empty memory BlobStore.
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlobStore for MemoryBlobStore {
    fn put(&self, input: &mut dyn Read) -> Result<Blob, BlobStoreError> {
        let mut writer = MemoryBlobWriter::new(self);
        io::copy(input, &mut writer)?;

        writer.commit()
    }

    fn writer(&self) -> Result<Box<dyn BlobWriter + '_>, BlobStoreError> {
        Ok(Box::new(MemoryBlobWriter::new(self)))
    }

    fn open(&self, blob: Blob) -> Result<Arc<BlobMemory>, BlobStoreError> {
        let memory = self
            .blobs
            .read()
            .get(&blob.id)
            .cloned()
            .ok_or(BlobStoreError::Missing { blob })?;
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

    fn contains(&self, blob: Blob) -> Result<bool, BlobStoreError> {
        match self.open(blob) {
            Ok(_memory) => Ok(true),
            Err(BlobStoreError::Missing { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn read(
        &self,
        blob: Blob,
        range: Range<u64>,
        output: &mut dyn Write,
    ) -> Result<(), BlobStoreError> {
        let memory = self.open(blob)?;
        let range = checked_range(blob, range)?;
        let bytes = memory.bytes();
        let mut input = Cursor::new(&bytes[range]);
        io::copy(&mut input, output)?;

        Ok(())
    }
}

impl<'a> MemoryBlobWriter<'a> {
    /// Create one unpublished Blob writer.
    fn new(store: &'a MemoryBlobStore) -> Self {
        Self {
            store,
            bytes: Vec::new(),
        }
    }

    /// Publish the complete Blob.
    fn commit(self) -> Result<Blob, BlobStoreError> {
        let memory = Arc::new(Memory::from_bytes(&self.bytes));
        let memory = Arc::new(BlobMemory::from_shared(memory));
        let blob = memory.blob();

        // retain one aligned copy per exact identity
        let mut blobs = self.store.blobs.write();
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

        Ok(blob)
    }
}

impl Write for MemoryBlobWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buffer);

        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl BlobWriter for MemoryBlobWriter<'_> {
    fn commit(self: Box<Self>) -> Result<Blob, BlobStoreError> {
        MemoryBlobWriter::commit(*self)
    }
}

impl Memory {
    /// Copy exact bytes into aligned immutable memory.
    fn from_bytes(bytes: &[u8]) -> Self {
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

impl AsRef<[u8]> for Memory {
    fn as_ref(&self) -> &[u8] {
        let bytes = self.chunks.as_ptr().cast::<u8>();

        // safety: byte_len never exceeds the initialized backing chunks
        unsafe { slice::from_raw_parts(bytes, self.byte_len) }
    }
}

/// Validate and convert one Blob byte range.
fn checked_range(blob: Blob, range: Range<u64>) -> Result<Range<usize>, BlobStoreError> {
    if range.start > range.end || range.end > blob.byte_len {
        let error = io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Blob range {}..{} exceeds {} bytes",
                range.start, range.end, blob.byte_len
            ),
        );

        return Err(error.into());
    }

    // convert the validated range to this host's address width
    let start = usize::try_from(range.start).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Blob range start exceeds usize",
        )
    })?;
    let end = usize::try_from(range.end)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Blob range end exceeds usize"))?;

    Ok(start..end)
}
