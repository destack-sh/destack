use postcard::Error as PostcardError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{ArtifactVersion, CacheStoreError};

use super::{ARTIFACT_IMAGE_HEADER_LENGTH_BYTES, ARTIFACT_IMAGE_LIMIT_BYTES, ARTIFACT_IMAGE_MAGIC};

/// Common header for one serialized artifact image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ImageHeader {
    /// The magic prefix used to identify artifact images.
    magic: [u8; 4],
    /// The exact artifact image version.
    version: ArtifactVersion,
}

impl ImageHeader {
    /// Create a new artifact image header.
    fn new(version: ArtifactVersion) -> Self {
        Self {
            magic: ARTIFACT_IMAGE_MAGIC,
            version,
        }
    }

    /// Split one serialized image into the decoded header and payload bytes.
    fn split_from_bytes(bytes: &[u8]) -> Result<(ImageHeader, &[u8]), ArtifactImageError> {
        let (header, payload_offset) = Self::decode_prefixed(bytes)?;
        let payload_bytes = &bytes[payload_offset..];

        Ok((header, payload_bytes))
    }

    /// Decode one serialized image header from the leading bytes.
    fn decode_prefixed(bytes: &[u8]) -> Result<(ImageHeader, usize), ArtifactImageError> {
        if bytes.len() < ARTIFACT_IMAGE_HEADER_LENGTH_BYTES {
            return Err(ArtifactImageError::Corrupt(
                "missing artifact image header length prefix",
            ));
        }

        let length_bytes: [u8; ARTIFACT_IMAGE_HEADER_LENGTH_BYTES] = bytes[0..4]
            .try_into()
            .map_err(|_| ArtifactImageError::Corrupt("invalid artifact image header length"))?;
        let header_length = u32::from_le_bytes(length_bytes) as usize;
        let payload_offset = ARTIFACT_IMAGE_HEADER_LENGTH_BYTES + header_length;
        if bytes.len() < payload_offset {
            return Err(ArtifactImageError::Corrupt(
                "truncated artifact image header bytes",
            ));
        }

        let header_bytes = &bytes[ARTIFACT_IMAGE_HEADER_LENGTH_BYTES..payload_offset];
        let header = deserialize_payload_with_limit::<ImageHeader>(
            header_bytes,
            ARTIFACT_IMAGE_LIMIT_BYTES,
        )?;

        Ok((header, payload_offset))
    }
}

/// One serialized artifact image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactImage<T> {
    /// The exact artifact version.
    version: ArtifactVersion,
    /// The serialized payload.
    pub payload: T,
}

impl<T> ArtifactImage<T> {
    /// Create one artifact image.
    pub fn new(version: ArtifactVersion, payload: T) -> Self {
        Self { version, payload }
    }

    /// Return the exact artifact version.
    pub fn version(&self) -> ArtifactVersion {
        self.version
    }
}

impl<T> ArtifactImage<T>
where
    T: Serialize,
{
    /// Serialize this artifact image to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, ArtifactImageError> {
        self.serialize_with_limit(ARTIFACT_IMAGE_LIMIT_BYTES)
    }

    /// Serialize this artifact image to bytes with one explicit size limit.
    pub(crate) fn serialize_with_limit(&self, limit: u64) -> Result<Vec<u8>, ArtifactImageError> {
        let header = ImageHeader::new(self.version);
        let header_bytes = serialize_payload_with_limit(&header, limit)?;
        let header_length =
            u32::try_from(header_bytes.len()).map_err(|_| ArtifactImageError::Size {
                limit,
                actual: header_bytes.len() as u64,
            })?;
        let payload_bytes = serialize_payload_with_limit(&self.payload, limit)?;
        let total_bytes = ARTIFACT_IMAGE_HEADER_LENGTH_BYTES as u64
            + header_bytes.len() as u64
            + payload_bytes.len() as u64;
        if total_bytes > limit {
            return Err(ArtifactImageError::Size {
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

impl<T> ArtifactImage<T>
where
    T: DeserializeOwned,
{
    /// Deserialize one artifact image from bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ArtifactImageError> {
        let (header, payload_bytes) = ImageHeader::split_from_bytes(bytes)?;
        if header.magic != ARTIFACT_IMAGE_MAGIC {
            return Err(ArtifactImageError::Corrupt("invalid artifact image magic"));
        }

        let payload =
            deserialize_payload_with_limit::<T>(payload_bytes, ARTIFACT_IMAGE_LIMIT_BYTES)?;

        Ok(Self {
            version: header.version,
            payload,
        })
    }
}

/// Errors that can occur while reading or writing artifact images.
#[derive(Debug)]
pub enum ArtifactImageError {
    /// The image bytes are malformed or internally inconsistent.
    Corrupt(&'static str),
    /// The image header version did not match the expected exact version.
    Version {
        expected: ArtifactVersion,
        found: ArtifactVersion,
    },
    /// The image failed to encode or decode.
    Codec(PostcardError),
    /// The image exceeded the configured size limit.
    Size { limit: u64, actual: u64 },
    /// The cache already has different bytes for the same exact version.
    Conflict { version: ArtifactVersion },
    /// The artifact image cache failed to read or write.
    Cache(CacheStoreError),
}

impl fmt::Display for ArtifactImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactImageError::Corrupt(message) => {
                write!(f, "corrupt artifact image: {message}")
            }
            ArtifactImageError::Version { expected, found } => {
                write!(
                    f,
                    "unexpected artifact image version, expected {expected:?}, found {found:?}"
                )
            }
            ArtifactImageError::Codec(error) => {
                write!(f, "artifact image codec error: {error}")
            }
            ArtifactImageError::Size { limit, actual } => {
                write!(
                    f,
                    "artifact image exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            ArtifactImageError::Conflict { version } => {
                write!(f, "conflicting artifact image bytes for {version:?}")
            }
            ArtifactImageError::Cache(error) => {
                write!(f, "artifact image cache error: {error}")
            }
        }
    }
}

impl std::error::Error for ArtifactImageError {}

impl From<std::io::Error> for ArtifactImageError {
    fn from(error: std::io::Error) -> Self {
        Self::Cache(CacheStoreError::Io(error))
    }
}

/// Serialize one payload with a size limit.
fn serialize_payload_with_limit<T: Serialize>(
    payload: &T,
    limit: u64,
) -> Result<Vec<u8>, ArtifactImageError> {
    let bytes = postcard::to_allocvec(payload).map_err(ArtifactImageError::Codec)?;
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(ArtifactImageError::Size { limit, actual });
    }

    Ok(bytes)
}

/// Deserialize one payload with a size limit.
fn deserialize_payload_with_limit<T: DeserializeOwned>(
    bytes: &[u8],
    limit: u64,
) -> Result<T, ArtifactImageError> {
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(ArtifactImageError::Size { limit, actual });
    }

    postcard::from_bytes(bytes).map_err(ArtifactImageError::Codec)
}
