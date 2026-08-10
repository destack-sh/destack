use std::sync::Arc;
use std::{error, fmt, str};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

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
    memory: Arc<dyn Memory>,
}

/// Shared immutable byte memory.
trait Memory: AsRef<[u8]> + fmt::Debug + Send + Sync {}

impl<T> Memory for T where T: AsRef<[u8]> + fmt::Debug + Send + Sync {}

impl BlobMemory {
    /// Retain and identify shared immutable byte memory.
    pub fn from_shared<T>(memory: Arc<T>) -> Self
    where
        T: AsRef<[u8]> + fmt::Debug + Send + Sync + 'static,
    {
        let blob = Blob::for_bytes(memory.as_ref().as_ref());

        Self { blob, memory }
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
        self.memory.as_ref().as_ref()
    }
}

impl AsRef<[u8]> for BlobMemory {
    fn as_ref(&self) -> &[u8] {
        self.bytes()
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
