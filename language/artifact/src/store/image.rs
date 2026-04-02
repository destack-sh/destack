use postcard::Error as PostcardError;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::{
    ARTIFACT_IMAGE_HEADER_LENGTH_BYTES, ARTIFACT_IMAGE_LIMIT_BYTES, ARTIFACT_IMAGE_MAGIC,
    ArtifactFamily, ArtifactImageKey,
};
use crate::ArtifactContentId;

/// Persisted dependency for one artifact image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactImageDependency {
    /// The required artifact image key.
    pub key: ArtifactImageKey,
    /// The required dependency content id.
    pub content_id: ArtifactContentId,
}

/// Common header for one serialized artifact image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactImageHeader {
    /// The magic prefix used to identify artifact images.
    pub magic: [u8; 4],
    /// The stable artifact image key for the serialized payload.
    pub artifact_image_key: ArtifactImageKey,
    /// Hash of the effective non-artifact inputs.
    pub input_hash: u64,
    /// Persisted artifact image dependencies required to reuse this image safely.
    pub dependencies: Vec<ArtifactImageDependency>,
}

impl ArtifactImageHeader {
    /// Create a new artifact image header.
    pub fn new(artifact_image_key: ArtifactImageKey, input_hash: u64) -> Self {
        Self {
            magic: ARTIFACT_IMAGE_MAGIC,
            artifact_image_key,
            input_hash,
            dependencies: Vec::new(),
        }
    }

    /// Attach one persisted dependency list to this header.
    pub fn with_dependencies(self, dependencies: Vec<ArtifactImageDependency>) -> Self {
        Self {
            dependencies,
            ..self
        }
    }

    /// Validate this header for one expected artifact family.
    pub fn validate_for_family(
        &self,
        expected_family: ArtifactFamily,
    ) -> Result<(), ArtifactImageError> {
        if self.magic != ARTIFACT_IMAGE_MAGIC {
            return Err(ArtifactImageError::InvalidMagic {
                expected: ARTIFACT_IMAGE_MAGIC,
                found: self.magic,
            });
        }

        let found_family = self.artifact_image_key.family();
        if found_family != expected_family {
            return Err(ArtifactImageError::InvalidArtifactFamily {
                expected: expected_family,
                found: found_family,
            });
        }

        Ok(())
    }

    /// Compare this header with another header.
    pub fn matches(&self, actual: &Self) -> bool {
        self.magic == actual.magic
            && self.artifact_image_key == actual.artifact_image_key
            && self.input_hash == actual.input_hash
    }

    /// Canonicalize the persisted image dependencies on this header.
    fn canonicalize_dependencies(&mut self) -> Result<(), ArtifactImageError> {
        let mut keyed_dependencies = Vec::with_capacity(self.dependencies.len());

        for dependency in self.dependencies.drain(..) {
            let key_bytes =
                postcard::to_allocvec(&dependency.key).map_err(ArtifactImageError::Serialize)?;
            keyed_dependencies.push((key_bytes, dependency));
        }

        keyed_dependencies.sort_unstable_by(|left, right| left.0.cmp(&right.0));
        keyed_dependencies.dedup_by(|left, right| left.1.key == right.1.key);
        self.dependencies = keyed_dependencies
            .into_iter()
            .map(|(_, dependency)| dependency)
            .collect();

        Ok(())
    }

    /// Split one serialized image into the decoded header and payload bytes.
    pub fn split_from_bytes(
        bytes: &[u8],
    ) -> Result<(ArtifactImageHeader, &[u8]), ArtifactImageError> {
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
    pub header: ArtifactImageHeader,
    /// The serialized payload.
    pub payload: T,
}

impl<T> ArtifactImage<T>
where
    T: Serialize,
{
    /// Create one artifact image with a computed payload hash.
    pub fn new(mut header: ArtifactImageHeader, payload: T) -> Result<Self, ArtifactImageError> {
        header.canonicalize_dependencies()?;
        Ok(Self { header, payload })
    }

    /// Validate the image header and payload hash for one expected family.
    pub fn validate_for_family(
        &self,
        expected_family: ArtifactFamily,
    ) -> Result<(), ArtifactImageError> {
        self.header.validate_for_family(expected_family)
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
    /// The image header family did not match the expected payload family.
    InvalidArtifactFamily {
        expected: ArtifactFamily,
        found: ArtifactFamily,
    },
    /// The image failed to serialize.
    Serialize(PostcardError),
    /// The image failed to deserialize.
    Deserialize(PostcardError),
    /// The image framing was malformed.
    InvalidLayout(&'static str),
    /// The image exceeded the configured size limit.
    SizeLimitExceeded { limit: u64, actual: u64 },
    /// The stored content id did not match the persisted bytes.
    InvalidContentId {
        expected: ArtifactContentId,
        found: ArtifactContentId,
    },
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
            ArtifactImageError::InvalidArtifactFamily { expected, found } => {
                write!(
                    f,
                    "invalid artifact family, expected {expected:?}, found {found:?}"
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
            ArtifactImageError::InvalidContentId { expected, found } => {
                write!(f, "invalid content id, expected {expected}, found {found}")
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
