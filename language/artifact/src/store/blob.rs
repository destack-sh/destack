use postcard::Error as PostcardError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{ArtifactVersion, CacheStoreError};

use super::{ARTIFACT_BLOB_HEADER_LENGTH_BYTES, ARTIFACT_BLOB_MAGIC, CACHE_BLOB_LIMIT_BYTES};

/// Common header for one serialized artifact payload blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PayloadHeader {
    /// The magic prefix used to identify artifact payload blobs.
    magic: [u8; 4],
    /// The exact artifact version.
    version: ArtifactVersion,
}

impl PayloadHeader {
    /// Create a new artifact payload header.
    fn new(version: ArtifactVersion) -> Self {
        Self {
            magic: ARTIFACT_BLOB_MAGIC,
            version,
        }
    }

    /// Split one serialized blob into the decoded header and payload bytes.
    fn split_from_bytes(bytes: &[u8]) -> Result<(PayloadHeader, &[u8]), ArtifactBlobError> {
        let (header, payload_offset) = Self::decode_prefixed(bytes)?;
        let payload_bytes = &bytes[payload_offset..];

        Ok((header, payload_bytes))
    }

    /// Decode one serialized payload header from the leading bytes.
    fn decode_prefixed(bytes: &[u8]) -> Result<(PayloadHeader, usize), ArtifactBlobError> {
        if bytes.len() < ARTIFACT_BLOB_HEADER_LENGTH_BYTES {
            return Err(ArtifactBlobError::Corrupt(
                "missing artifact payload header length prefix",
            ));
        }

        let length_bytes: [u8; ARTIFACT_BLOB_HEADER_LENGTH_BYTES] = bytes[0..4]
            .try_into()
            .map_err(|_| ArtifactBlobError::Corrupt("invalid artifact payload header length"))?;
        let header_length = u32::from_le_bytes(length_bytes) as usize;
        let payload_offset = ARTIFACT_BLOB_HEADER_LENGTH_BYTES + header_length;
        if bytes.len() < payload_offset {
            return Err(ArtifactBlobError::Corrupt(
                "truncated artifact payload header bytes",
            ));
        }

        let header_bytes = &bytes[ARTIFACT_BLOB_HEADER_LENGTH_BYTES..payload_offset];
        let header =
            deserialize_payload_with_limit::<PayloadHeader>(header_bytes, CACHE_BLOB_LIMIT_BYTES)?;

        Ok((header, payload_offset))
    }
}

/// One versioned artifact payload blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPayloadBlob<T> {
    /// The exact artifact version.
    version: ArtifactVersion,
    /// The serialized payload.
    pub payload: T,
}

impl<T> ArtifactPayloadBlob<T> {
    /// Create one artifact payload blob.
    pub fn new(version: ArtifactVersion, payload: T) -> Self {
        Self { version, payload }
    }

    /// Return the exact artifact version.
    pub fn version(&self) -> ArtifactVersion {
        self.version
    }
}

impl<T> ArtifactPayloadBlob<T>
where
    T: Serialize,
{
    /// Serialize this artifact payload blob to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, ArtifactBlobError> {
        self.serialize_with_limit(CACHE_BLOB_LIMIT_BYTES)
    }

    /// Serialize this artifact payload blob to bytes with one explicit size limit.
    pub(crate) fn serialize_with_limit(&self, limit: u64) -> Result<Vec<u8>, ArtifactBlobError> {
        let header = PayloadHeader::new(self.version);
        let header_bytes = serialize_payload_with_limit(&header, limit)?;
        let header_length =
            u32::try_from(header_bytes.len()).map_err(|_| ArtifactBlobError::Size {
                limit,
                actual: header_bytes.len() as u64,
            })?;
        let payload_bytes = serialize_payload_with_limit(&self.payload, limit)?;
        let total_bytes = ARTIFACT_BLOB_HEADER_LENGTH_BYTES as u64
            + header_bytes.len() as u64
            + payload_bytes.len() as u64;
        if total_bytes > limit {
            return Err(ArtifactBlobError::Size {
                limit,
                actual: total_bytes,
            });
        }

        let mut bytes = Vec::with_capacity(total_bytes as usize);
        bytes.extend_from_slice(&header_length.to_le_bytes());
        bytes.extend_from_slice(&header_bytes);
        bytes.extend_from_slice(&payload_bytes);

        Ok(bytes)
    }
}

impl<T> ArtifactPayloadBlob<T>
where
    T: DeserializeOwned,
{
    /// Deserialize one artifact payload blob from bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ArtifactBlobError> {
        let (header, payload_bytes) = PayloadHeader::split_from_bytes(bytes)?;
        if header.magic != ARTIFACT_BLOB_MAGIC {
            return Err(ArtifactBlobError::Corrupt("invalid artifact payload magic"));
        }

        let payload = deserialize_payload_with_limit::<T>(payload_bytes, CACHE_BLOB_LIMIT_BYTES)?;

        Ok(Self {
            version: header.version,
            payload,
        })
    }
}

/// Errors that can occur while reading or writing artifact blobs.
#[derive(Debug)]
pub enum ArtifactBlobError {
    /// The blob bytes are malformed or internally inconsistent.
    Corrupt(&'static str),
    /// The blob header version did not match the expected exact version.
    Version {
        /// The requested artifact version.
        expected: Box<ArtifactVersion>,
        /// The artifact version carried by the blob.
        found: Box<ArtifactVersion>,
    },
    /// The blob failed to encode or decode.
    Codec(Box<PostcardError>),
    /// The blob exceeded the configured size limit.
    Size { limit: u64, actual: u64 },
    /// The cache already has different bytes for the same exact version.
    Conflict { version: Box<ArtifactVersion> },
    /// The artifact store failed to read or write.
    Store(Box<CacheStoreError>),
}

impl fmt::Display for ArtifactBlobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactBlobError::Corrupt(message) => {
                write!(f, "corrupt artifact blob: {message}")
            }
            ArtifactBlobError::Version { expected, found } => {
                write!(
                    f,
                    "unexpected artifact blob version, expected {expected:?}, found {found:?}"
                )
            }
            ArtifactBlobError::Codec(error) => {
                write!(f, "artifact blob codec error: {error}")
            }
            ArtifactBlobError::Size { limit, actual } => {
                write!(
                    f,
                    "artifact blob exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            ArtifactBlobError::Conflict { version } => {
                write!(f, "conflicting artifact blob bytes for {version:?}")
            }
            ArtifactBlobError::Store(error) => {
                write!(f, "artifact store error: {error}")
            }
        }
    }
}

impl std::error::Error for ArtifactBlobError {}

impl From<std::io::Error> for ArtifactBlobError {
    fn from(error: std::io::Error) -> Self {
        Self::Store(Box::new(CacheStoreError::from(error)))
    }
}

/// Serialize one payload with a size limit.
fn serialize_payload_with_limit<T: Serialize>(
    payload: &T,
    limit: u64,
) -> Result<Vec<u8>, ArtifactBlobError> {
    let bytes = postcard::to_allocvec(payload)
        .map_err(|error| ArtifactBlobError::Codec(Box::new(error)))?;
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(ArtifactBlobError::Size { limit, actual });
    }

    Ok(bytes)
}

/// Deserialize one payload with a size limit.
fn deserialize_payload_with_limit<T: DeserializeOwned>(
    bytes: &[u8],
    limit: u64,
) -> Result<T, ArtifactBlobError> {
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(ArtifactBlobError::Size { limit, actual });
    }

    postcard::from_bytes(bytes).map_err(|error| ArtifactBlobError::Codec(Box::new(error)))
}
