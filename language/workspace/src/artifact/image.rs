use std::fmt;
use std::hash::{Hash, Hasher};

use postcard::Error as PostcardError;
use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use destack_source::ProfileVersion;

use super::{ArtifactFamily, ArtifactImageKey};

/// Magic prefix for on disk artifact images.
pub const ARTIFACT_IMAGE_MAGIC: [u8; 4] = *b"DSCH";
/// Artifact image format version.
pub const ARTIFACT_IMAGE_FORMAT_VERSION: u32 = 4;
/// Limit artifact image size to avoid excessive memory usage.
pub const ARTIFACT_IMAGE_LIMIT_BYTES: u64 = 512 * 1024 * 1024;

/// Common header for one serialized artifact image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactImageHeader {
    /// The magic prefix used to identify artifact images.
    pub magic: [u8; 4],
    /// The artifact image format version.
    pub format_version: u32,
    /// The compiler version that produced the image.
    pub compiler_version: String,
    /// The stable artifact image key for the serialized payload.
    pub artifact_image_key: ArtifactImageKey,
    /// The profile version used when producing the image when applicable.
    pub profile_version: Option<ProfileVersion>,
    /// Hash of the effective compiler configuration.
    pub config_hash: u64,
    /// Hash of the workspace string universe when the image depends on it.
    pub workspace_strings_hash: Option<u64>,
    /// Hash of the serialized payload bytes.
    pub payload_hash: u64,
}

#[allow(clippy::too_many_arguments)]
impl ArtifactImageHeader {
    /// Create a new artifact image header.
    pub fn new(
        artifact_image_key: ArtifactImageKey,
        compiler_version: String,
        profile_version: Option<ProfileVersion>,
        config_hash: u64,
        workspace_strings_hash: Option<u64>,
        payload_hash: u64,
    ) -> Self {
        Self {
            magic: ARTIFACT_IMAGE_MAGIC,
            format_version: ARTIFACT_IMAGE_FORMAT_VERSION,
            compiler_version,
            artifact_image_key,
            profile_version,
            config_hash,
            workspace_strings_hash,
            payload_hash,
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

        if self.format_version != ARTIFACT_IMAGE_FORMAT_VERSION {
            return Err(ArtifactImageError::InvalidFormatVersion {
                expected: ARTIFACT_IMAGE_FORMAT_VERSION,
                found: self.format_version,
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

    /// Compare this header with another header ignoring payload hash.
    pub fn matches(&self, actual: &Self) -> bool {
        self.magic == actual.magic
            && self.format_version == actual.format_version
            && self.compiler_version == actual.compiler_version
            && self.artifact_image_key == actual.artifact_image_key
            && self.profile_version == actual.profile_version
            && self.config_hash == actual.config_hash
            && self.workspace_strings_hash == actual.workspace_strings_hash
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
        header.payload_hash = artifact_payload_hash_from_payload(&payload)?;
        Ok(Self { header, payload })
    }

    /// Validate the image header and payload hash for one expected family.
    pub fn validate_for_family(
        &self,
        expected_family: ArtifactFamily,
    ) -> Result<(), ArtifactImageError> {
        self.header
            .validate_for_family(expected_family)
            .and_then(|_| validate_artifact_image_payload_hash(&self.header, &self.payload))
    }

    /// Serialize this artifact image to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, ArtifactImageError> {
        self.serialize_with_limit(ARTIFACT_IMAGE_LIMIT_BYTES)
    }

    /// Serialize this artifact image to bytes with one explicit size limit.
    pub(crate) fn serialize_with_limit(&self, limit: u64) -> Result<Vec<u8>, ArtifactImageError> {
        let payload = serialize_payload_with_limit(&self.payload, limit)?;
        let envelope = ArtifactImageEnvelope {
            header: self.header.clone(),
            payload,
        };

        serialize_payload_with_limit(&envelope, limit)
    }
}

impl<T> ArtifactImage<T>
where
    T: DeserializeOwned,
{
    /// Deserialize one artifact image from bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, ArtifactImageError> {
        let envelope = deserialize_payload_with_limit::<ArtifactImageEnvelope>(
            bytes,
            ARTIFACT_IMAGE_LIMIT_BYTES,
        )?;

        let payload_hash = payload_hash_from_bytes(&envelope.payload);
        if payload_hash != envelope.header.payload_hash {
            return Err(ArtifactImageError::InvalidPayloadHash {
                expected: envelope.header.payload_hash,
                found: payload_hash,
            });
        }

        let payload =
            deserialize_payload_with_limit::<T>(&envelope.payload, ARTIFACT_IMAGE_LIMIT_BYTES)?;

        Ok(Self {
            header: envelope.header,
            payload,
        })
    }
}

/// Serialized artifact image envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArtifactImageEnvelope {
    /// The image header.
    header: ArtifactImageHeader,
    /// The serialized payload bytes.
    payload: Vec<u8>,
}

/// Errors that can occur while reading or writing artifact images.
#[derive(Debug)]
pub enum ArtifactImageError {
    /// The image header magic did not match.
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },
    /// The image header format version did not match.
    InvalidFormatVersion { expected: u32, found: u32 },
    /// The image header family did not match the expected payload family.
    InvalidArtifactFamily {
        expected: ArtifactFamily,
        found: ArtifactFamily,
    },
    /// The image failed to serialize.
    Serialize(PostcardError),
    /// The image failed to deserialize.
    Deserialize(PostcardError),
    /// The image exceeded the configured size limit.
    SizeLimitExceeded { limit: u64, actual: u64 },
    /// The payload hash did not match.
    InvalidPayloadHash { expected: u64, found: u64 },
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
            ArtifactImageError::InvalidFormatVersion { expected, found } => {
                write!(
                    f,
                    "invalid artifact image format version, expected {expected}, found {found}"
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
            ArtifactImageError::SizeLimitExceeded { limit, actual } => {
                write!(
                    f,
                    "artifact image exceeded size limit, limit {limit}, actual {actual}"
                )
            }
            ArtifactImageError::InvalidPayloadHash { expected, found } => {
                write!(
                    f,
                    "invalid payload hash, expected {expected}, found {found}"
                )
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

/// Compute one payload hash from serialized bytes.
fn payload_hash_from_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Compute one payload hash for a serializable artifact payload.
pub fn artifact_payload_hash_from_payload<T: Serialize>(
    payload: &T,
) -> Result<u64, ArtifactImageError> {
    let bytes = serialize_payload_with_limit(payload, ARTIFACT_IMAGE_LIMIT_BYTES)?;
    Ok(payload_hash_from_bytes(&bytes))
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

/// Validate one payload hash against an artifact image header.
fn validate_artifact_image_payload_hash<T: Serialize>(
    header: &ArtifactImageHeader,
    payload: &T,
) -> Result<(), ArtifactImageError> {
    let payload_hash = artifact_payload_hash_from_payload(payload)?;
    if payload_hash != header.payload_hash {
        return Err(ArtifactImageError::InvalidPayloadHash {
            expected: header.payload_hash,
            found: payload_hash,
        });
    }

    Ok(())
}
