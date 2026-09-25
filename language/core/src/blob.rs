use std::collections::hash_map;
use std::io::{self, Read, Write};
use std::mem::size_of;
use std::sync::Arc;
use std::{error, fmt, ptr, slice, str};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{SectionEntry, SectionImageError, SectionLoader};

/// Bytes in one complete BlobId.
const BLOB_ID_BYTE_LEN: usize = 32;
/// Hexadecimal characters in one displayed BlobId.
const BLOB_ID_TEXT_LEN: usize = BLOB_ID_BYTE_LEN * 2;

/// The identity of one exact immutable byte sequence.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct BlobId([u8; BLOB_ID_BYTE_LEN]);

impl BlobId {
    /// Build one BlobId from its complete BLAKE3 digest.
    pub const fn new(bytes: [u8; BLOB_ID_BYTE_LEN]) -> Self {
        Self(bytes)
    }

    /// Derive one BlobId from exact bytes.
    pub fn for_bytes(bytes: &[u8]) -> Self {
        Self::new(*blake3::hash(bytes).as_bytes())
    }

    /// Return the complete BLAKE3 digest.
    pub const fn bytes(self) -> [u8; BLOB_ID_BYTE_LEN] {
        self.0
    }
}

impl fmt::Debug for BlobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("b")?;
        format_blob_id(self, formatter)
    }
}

impl fmt::Display for BlobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_blob_id(self, formatter)
    }
}

impl str::FromStr for BlobId {
    type Err = ParseBlobIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.as_bytes();
        if text.len() != BLOB_ID_TEXT_LEN {
            return Err(ParseBlobIdError);
        }

        let mut bytes = [0_u8; BLOB_ID_BYTE_LEN];
        for (index, byte) in bytes.iter_mut().enumerate() {
            let offset = index * 2;
            let high = hex_digit(text[offset])?;
            let low = hex_digit(text[offset + 1])?;
            *byte = high << 4 | low;
        }

        Ok(Self::new(bytes))
    }
}

/// Invalid BlobId text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseBlobIdError;

impl fmt::Display for ParseBlobIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid BlobId")
    }
}

impl error::Error for ParseBlobIdError {}

/// One exact immutable byte sequence.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct Blob {
    /// The exact byte identity.
    pub id: BlobId,
    /// The byte length.
    pub byte_len: u64,
}

impl Blob {
    /// Build one Blob from its exact identity and byte length.
    pub const fn new(id: BlobId, byte_len: u64) -> Self {
        Self { id, byte_len }
    }

    /// Derive one Blob from exact bytes.
    pub fn for_bytes(bytes: &[u8]) -> Self {
        Self::new(BlobId::for_bytes(bytes), bytes.len() as u64)
    }
}

impl fmt::Display for Blob {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.id, self.byte_len)
    }
}

// safety: BlobId contains only a fixed byte array
unsafe impl SectionEntry for BlobId {
    const NEEDS_VALIDATION: bool = false;

    fn validate(_bytes: &[u8], _loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        Ok(())
    }
}

// safety: Blob contains only SectionEntry fields under a stable C representation
unsafe impl SectionEntry for Blob {
    const NEEDS_VALIDATION: bool = false;

    fn validate(_bytes: &[u8], _loader: SectionLoader<'_>) -> Result<(), SectionImageError> {
        Ok(())
    }
}

/// Retained immutable memory for one verified Blob.
#[derive(Debug, Clone)]
pub struct BlobMemory {
    /// The exact retained Blob.
    blob: Blob,
    /// The retained byte memory.
    memory: BlobBytes,
}

/// Immutable Blob bytes by ownership.
#[derive(Debug, Clone)]
enum BlobBytes {
    /// Build-owned bytes.
    Static(&'static [u8]),
    /// Runtime-owned bytes.
    Shared(Arc<dyn Memory>),
}

/// Shared immutable byte memory.
trait Memory: AsRef<[u8]> + fmt::Debug + Send + Sync {}

impl<T> Memory for T where T: AsRef<[u8]> + fmt::Debug + Send + Sync {}

impl BlobMemory {
    /// Retain static bytes with their build-verified Blob.
    ///
    /// # Safety
    ///
    /// `memory` must have the exact identity and length described by `blob`.
    pub unsafe fn from_static(blob: Blob, memory: &'static [u8]) -> Self {
        Self {
            blob,
            memory: BlobBytes::Static(memory),
        }
    }

    /// Retain and identify shared immutable byte memory.
    pub fn from_shared<T>(memory: Arc<T>) -> Self
    where
        T: AsRef<[u8]> + fmt::Debug + Send + Sync + 'static,
    {
        let blob = Blob::for_bytes(memory.as_ref().as_ref());

        Self {
            blob,
            memory: BlobBytes::Shared(memory),
        }
    }

    /// Retain and identify owned immutable bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::from_shared(Arc::new(bytes))
    }

    /// Return the exact retained Blob.
    pub const fn blob(&self) -> Blob {
        self.blob
    }

    /// Return the retained immutable bytes.
    pub fn bytes(&self) -> &[u8] {
        match &self.memory {
            BlobBytes::Static(memory) => memory,
            BlobBytes::Shared(memory) => memory.as_ref().as_ref(),
        }
    }
}

impl AsRef<[u8]> for BlobMemory {
    fn as_ref(&self) -> &[u8] {
        self.bytes()
    }
}

/// Failure while storing or opening immutable Blobs.
#[derive(Debug)]
pub enum BlobStoreError {
    /// One Blob was not present.
    Missing {
        /// The missing Blob.
        blob: Blob,
    },
    /// Stored bytes did not match their expected description.
    Corrupt {
        /// The expected Blob.
        expected: Blob,
        /// The observed Blob.
        actual: Blob,
    },
    /// Stored bytes have the wrong length.
    Length {
        /// The expected Blob.
        blob: Blob,
        /// The observed byte length.
        actual: u64,
    },
    /// The backing I/O operation failed.
    Io(Box<io::Error>),
}

impl fmt::Display for BlobStoreError {
    /// Format this Blob store failure.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { blob } => write!(formatter, "missing Blob {}", blob.id),
            Self::Corrupt { expected, actual } => write!(
                formatter,
                "corrupt Blob {}: expected {} bytes, observed Blob {} with {} bytes",
                expected.id, expected.byte_len, actual.id, actual.byte_len
            ),
            Self::Length { blob, actual } => write!(
                formatter,
                "corrupt Blob {}: expected {} bytes, observed {actual} bytes",
                blob.id, blob.byte_len
            ),
            Self::Io(error) => write!(formatter, "Blob store I/O failed: {error}"),
        }
    }
}

impl error::Error for BlobStoreError {}

impl From<io::Error> for BlobStoreError {
    /// Convert one backing I/O failure.
    fn from(error: io::Error) -> Self {
        Self::Io(Box::new(error))
    }
}

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

    /// Retain one exact byte slice in aligned immutable storage.
    pub fn retain_bytes(&self, bytes: &[u8]) -> Result<Blob, BlobStoreError> {
        let memory = Arc::new(AlignedBytes::new(bytes));
        let memory = Arc::new(BlobMemory::from_shared(memory));

        self.retain_memory(memory)
    }

    /// Retain one existing immutable byte allocation.
    pub fn retain_memory(&self, memory: Arc<BlobMemory>) -> Result<Blob, BlobStoreError> {
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

        Ok(blob)
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
        let blobs = self.blobs.read();
        let Some(memory) = blobs.get(&blob.id) else {
            return Ok(false);
        };
        if memory.blob() != blob {
            return Err(BlobStoreError::Corrupt {
                expected: blob,
                actual: memory.blob(),
            });
        }

        Ok(true)
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
        self.store.retain_bytes(&self.bytes)
    }
}

impl Write for BlobWriter<'_> {
    /// Append bytes to this unpublished Blob.
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(buffer);

        Ok(buffer.len())
    }

    /// Complete pending backing writes.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Aligned immutable byte allocation.
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
    /// Borrow the initialized byte slice.
    fn as_ref(&self) -> &[u8] {
        let bytes = self.chunks.as_ptr().cast::<u8>();

        // safety: byte_len never exceeds the initialized backing chunks
        unsafe { slice::from_raw_parts(bytes, self.byte_len) }
    }
}

/// Format one BlobId as lowercase hexadecimal text.
fn format_blob_id(id: &BlobId, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in id.0 {
        write!(formatter, "{byte:02x}")?;
    }

    Ok(())
}

/// Decode one lowercase hexadecimal digit.
fn hex_digit(byte: u8) -> Result<u8, ParseBlobIdError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(ParseBlobIdError),
    }
}
