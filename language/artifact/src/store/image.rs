use postcard::Error as PostcardError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::ArtifactVersion;

use super::{ARTIFACT_IMAGE_HEADER_LENGTH_BYTES, ARTIFACT_IMAGE_LIMIT_BYTES, ARTIFACT_IMAGE_MAGIC};

/// Common header for one serialized artifact image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactImageHeader {
    /// The magic prefix used to identify artifact images.
    pub(crate) magic: [u8; 4],
    /// The exact artifact image version.
    pub(crate) version: ArtifactVersion,
}

impl ArtifactImageHeader {
    /// Create a new artifact image header.
    pub fn new(version: ArtifactVersion) -> Self {
        Self {
            magic: ARTIFACT_IMAGE_MAGIC,
            version,
        }
    }

    /// Split one serialized image into the decoded header and payload bytes.
    fn split_from_bytes(bytes: &[u8]) -> Result<(ArtifactImageHeader, &[u8]), ArtifactImageError> {
        let (header, payload_offset) = Self::decode_prefixed(bytes)?;
        let payload_bytes = &bytes[payload_offset..];

        Ok((header, payload_bytes))
    }

    /// Decode one serialized image header from the leading bytes.
    fn decode_prefixed(bytes: &[u8]) -> Result<(ArtifactImageHeader, usize), ArtifactImageError> {
        if bytes.len() < ARTIFACT_IMAGE_HEADER_LENGTH_BYTES {
            return Err(ArtifactImageError::InvalidLayout(
                "missing artifact image header length prefix",
            ));
        }

        let length_bytes: [u8; ARTIFACT_IMAGE_HEADER_LENGTH_BYTES] =
            bytes[0..4].try_into().map_err(|_| {
                ArtifactImageError::InvalidLayout("invalid artifact image header length")
            })?;
        let header_length = u32::from_le_bytes(length_bytes) as usize;
        let payload_offset = ARTIFACT_IMAGE_HEADER_LENGTH_BYTES + header_length;
        if bytes.len() < payload_offset {
            return Err(ArtifactImageError::InvalidLayout(
                "truncated artifact image header bytes",
            ));
        }

        let header_bytes = &bytes[ARTIFACT_IMAGE_HEADER_LENGTH_BYTES..payload_offset];
        let header = deserialize_payload_with_limit::<ArtifactImageHeader>(
            header_bytes,
            ARTIFACT_IMAGE_LIMIT_BYTES,
        )?;

        Ok((header, payload_offset))
    }
}

/// One serialized artifact image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactImage<T> {
    /// The image header.
    pub(crate) header: ArtifactImageHeader,
    /// The serialized payload.
    pub payload: T,
}

impl<T> ArtifactImage<T>
where
    T: Serialize,
{
    /// Create one artifact image.
    pub fn new(header: ArtifactImageHeader, payload: T) -> Self {
        Self { header, payload }
    }

    /// Serialize this artifact image to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, ArtifactImageError> {
        self.serialize_with_limit(ARTIFACT_IMAGE_LIMIT_BYTES)
    }

    /// Serialize this artifact image to bytes with one explicit size limit.
    pub(crate) fn serialize_with_limit(&self, limit: u64) -> Result<Vec<u8>, ArtifactImageError> {
        let header_bytes = serialize_payload_with_limit(&self.header, limit)?;
        let header_length = u32::try_from(header_bytes.len()).map_err(|_| {
            ArtifactImageError::SizeLimitExceeded {
                limit,
                actual: header_bytes.len() as u64,
            }
        })?;
        let payload_bytes = serialize_payload_with_limit(&self.payload, limit)?;
        let total_bytes = ARTIFACT_IMAGE_HEADER_LENGTH_BYTES as u64
            + header_bytes.len() as u64
            + payload_bytes.len() as u64;
        if total_bytes > limit {
            return Err(ArtifactImageError::SizeLimitExceeded {
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
        let (header, payload_bytes) = ArtifactImageHeader::split_from_bytes(bytes)?;
        let payload =
            deserialize_payload_with_limit::<T>(payload_bytes, ARTIFACT_IMAGE_LIMIT_BYTES)?;

        Ok(Self { header, payload })
    }
}

/// Errors that can occur while reading or writing artifact images.
#[derive(Debug)]
pub enum ArtifactImageError {
    /// The image header magic did not match.
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },
    /// The image header version did not match the expected exact version.
    UnexpectedImageVersion {
        expected: ArtifactVersion,
        found: ArtifactVersion,
    },
    /// The image failed to serialize.
    Serialize(PostcardError),
    /// The image failed to deserialize.
    Deserialize(PostcardError),
    /// The image framing was malformed.
    InvalidLayout(&'static str),
    /// The image exceeded the configured size limit.
    SizeLimitExceeded { limit: u64, actual: u64 },
    /// The cache already has different bytes for the same exact version.
    ConflictingImageVersion { version: ArtifactVersion },
    /// The cache reported an inconsistent write conflict.
    CacheConflict,
    /// The image failed to read or write.
    Io(std::io::Error),
}

impl fmt::Display for ArtifactImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtifactImageError::InvalidMagic { expected, found } => {
                write!(
                    f,
                    "invalid artifact image magic, expected {expected:?}, found {found:?}"
                )
            }
            ArtifactImageError::UnexpectedImageVersion { expected, found } => {
                write!(
                    f,
                    "unexpected artifact image version, expected {expected:?}, found {found:?}"
                )
            }
            ArtifactImageError::Serialize(error) => {
                write!(f, "failed to serialize artifact image: {error}")
            }
            ArtifactImageError::Deserialize(error) => {
                write!(f, "failed to deserialize artifact image: {error}")
            }
            ArtifactImageError::InvalidLayout(message) => {
                write!(f, "invalid artifact image layout: {message}")
            }
            ArtifactImageError::SizeLimitExceeded { limit, actual } => {
                write!(
                    f,
                    "artifact image exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            ArtifactImageError::ConflictingImageVersion { version } => {
                write!(f, "conflicting artifact image bytes for {version:?}")
            }
            ArtifactImageError::CacheConflict => {
                write!(f, "inconsistent artifact image cache conflict")
            }
            ArtifactImageError::Io(error) => write!(f, "artifact image io error: {error}"),
        }
    }
}

impl std::error::Error for ArtifactImageError {}

impl From<std::io::Error> for ArtifactImageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Serialize one payload with a size limit.
fn serialize_payload_with_limit<T: Serialize>(
    payload: &T,
    limit: u64,
) -> Result<Vec<u8>, ArtifactImageError> {
    let bytes = postcard::to_allocvec(payload).map_err(ArtifactImageError::Serialize)?;
    let actual = bytes.len() as u64;
    if actual > limit {
        return Err(ArtifactImageError::SizeLimitExceeded { limit, actual });
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
        return Err(ArtifactImageError::SizeLimitExceeded { limit, actual });
    }

    postcard::from_bytes(bytes).map_err(ArtifactImageError::Deserialize)
}
