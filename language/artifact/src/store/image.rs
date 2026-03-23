use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use postcard::Error as PostcardError;
use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use destack_source::ProfileVersion;

use super::{ArtifactFamily, ArtifactImageKey};
use crate::{
    CacheStore, CacheStoreError, DEFAULT_LANGUAGE_CACHE_DIR_NAME, DEFAULT_LANGUAGE_CACHE_NAMESPACE,
};

/// Magic prefix for on disk artifact images.
pub const ARTIFACT_IMAGE_MAGIC: [u8; 4] = *b"DSCH";
/// Artifact image format version.
pub const ARTIFACT_IMAGE_FORMAT_VERSION: u32 = 6;
/// Limit artifact image size to avoid excessive memory usage.
pub const ARTIFACT_IMAGE_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
/// Number of bytes used to encode one header length prefix.
pub const ARTIFACT_IMAGE_HEADER_LENGTH_BYTES: usize = 4;

/// Persisted requirement proof for one artifact image dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactImageRequirement {
    /// The required artifact image key.
    pub key: ArtifactImageKey,
    /// The validated dependency image hash for that artifact.
    pub validation_hash: u64,
}

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
    /// Family owned header context bytes for extra freshness validation.
    pub context_bytes: Vec<u8>,
    /// Hash of the payload and persisted dependency proofs.
    pub validation_hash: u64,
    /// Persisted dependency proofs required to reuse this image safely.
    pub requirements: Vec<ArtifactImageRequirement>,
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
            context_bytes: Vec::new(),
            validation_hash: 0,
            requirements: Vec::new(),
            payload_hash,
        }
    }

    /// Attach one persisted requirement proof list to this header.
    pub fn with_requirements(self, requirements: Vec<ArtifactImageRequirement>) -> Self {
        Self {
            requirements,
            ..self
        }
    }

    /// Attach one family owned header context payload to this header.
    pub fn with_context_bytes(self, context_bytes: Vec<u8>) -> Self {
        Self {
            context_bytes,
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
            && self.context_bytes == actual.context_bytes
    }

    /// Compare this header with another header ignoring family context bytes.
    pub fn matches_without_context_bytes(&self, actual: &Self) -> bool {
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
        header.requirements = canonicalize_artifact_image_requirements(header.requirements)?;
        header.payload_hash = artifact_payload_hash_from_payload(&payload)?;
        header.validation_hash =
            artifact_validation_hash(header.payload_hash, &header.requirements)?;
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
            .and_then(|_| validate_artifact_image_validation_hash(&self.header))
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
        let (header, payload_bytes) = split_artifact_image_bytes(bytes)?;
        validate_artifact_image_payload_hash_bytes(&header, payload_bytes)?;
        validate_artifact_image_validation_hash(&header)?;

        let payload =
            deserialize_payload_with_limit::<T>(payload_bytes, ARTIFACT_IMAGE_LIMIT_BYTES)?;

        Ok(Self { header, payload })
    }
}

impl ArtifactImageHeader {
    /// Deserialize one prefixed artifact image header from bytes.
    pub fn deserialize_prefixed(bytes: &[u8]) -> Result<Self, ArtifactImageError> {
        let (header, _) = split_artifact_image_header_bytes(bytes)?;
        validate_artifact_image_validation_hash(&header)?;

        Ok(header)
    }
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
    /// The image framing was malformed.
    InvalidLayout(&'static str),
    /// The image exceeded the configured size limit.
    SizeLimitExceeded { limit: u64, actual: u64 },
    /// The payload hash did not match.
    InvalidPayloadHash { expected: u64, found: u64 },
    /// The recursive validation hash did not match.
    InvalidValidationHash { expected: u64, found: u64 },
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
            ArtifactImageError::InvalidLayout(message) => {
                write!(f, "invalid artifact image layout: {message}")
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
            ArtifactImageError::InvalidValidationHash { expected, found } => {
                write!(
                    f,
                    "invalid validation hash, expected {expected}, found {found}"
                )
            }
            ArtifactImageError::Io(error) => write!(f, "artifact image io error: {error}"),
        }
    }
}

impl std::error::Error for ArtifactImageError {}

/// Cache backed artifact image reader and writer.
#[derive(Debug)]
pub struct ArtifactImageStore<'a> {
    /// The cache store backing the images.
    store: &'a dyn CacheStore,
    /// Base directory for persisted artifact images.
    image_root: PathBuf,
}

impl<'a> ArtifactImageStore<'a> {
    /// Create an artifact image store for a cache root.
    pub fn new(store: &'a dyn CacheStore, cache_root: &Path) -> Self {
        let image_root = cache_root
            .join(DEFAULT_LANGUAGE_CACHE_NAMESPACE)
            .join(DEFAULT_LANGUAGE_CACHE_DIR_NAME);

        Self { store, image_root }
    }

    /// Load one persisted artifact image by stable image key.
    pub fn load<T>(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned,
    {
        let Some(bytes) = self.load_bytes(artifact_image_key)? else {
            return Ok(None);
        };
        let image = ArtifactImage::<T>::deserialize(&bytes)?;

        Ok(Some(image))
    }

    /// Load one persisted artifact image header by stable image key.
    pub fn load_header(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        let image_path = self.image_path(artifact_image_key);
        let lock_path = self.lock_path(artifact_image_key);
        self.store.with_shared_lock(
            &lock_path,
            || -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
                if let Some(metadata) = self.store.metadata(&image_path)?
                    && metadata.size_bytes > ARTIFACT_IMAGE_LIMIT_BYTES
                {
                    return Err(ArtifactImageError::SizeLimitExceeded {
                        limit: ARTIFACT_IMAGE_LIMIT_BYTES,
                        actual: metadata.size_bytes,
                    }
                    .into());
                }

                let Some(prefix) = self
                    .store
                    .read_prefix(&image_path, ARTIFACT_IMAGE_HEADER_LENGTH_BYTES)?
                else {
                    return Ok(None);
                };
                if prefix.len() < ARTIFACT_IMAGE_HEADER_LENGTH_BYTES {
                    return Err(ArtifactImageError::InvalidLayout(
                        "missing artifact image header length prefix",
                    ));
                }

                let header_length = u32::from_le_bytes(
                    prefix[0..ARTIFACT_IMAGE_HEADER_LENGTH_BYTES]
                        .try_into()
                        .map_err(|_| {
                            ArtifactImageError::InvalidLayout(
                                "invalid artifact image header length",
                            )
                        })?,
                ) as usize;
                let total_prefix = ARTIFACT_IMAGE_HEADER_LENGTH_BYTES + header_length;
                let Some(header_bytes) = self.store.read_prefix(&image_path, total_prefix)? else {
                    return Ok(None);
                };
                let header = ArtifactImageHeader::deserialize_prefixed(&header_bytes)?;

                self.store.touch(&image_path)?;

                Ok(Some(header))
            },
        )
    }

    /// Save one persisted artifact image.
    pub fn save<T>(&self, image: &ArtifactImage<T>) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        let image_path = self.image_path(&image.header.artifact_image_key);
        let lock_path = self.lock_path(&image.header.artifact_image_key);
        self.store
            .with_exclusive_lock(&lock_path, || -> Result<(), ArtifactImageError> {
                let bytes = image.serialize()?;
                self.store.write_atomic(&image_path, &bytes)?;

                Ok(())
            })
    }

    /// Resolve the image path for one stable image key.
    fn image_path(&self, artifact_image_key: &ArtifactImageKey) -> PathBuf {
        let prefix = artifact_image_file_prefix(artifact_image_key);
        let hash = self.image_key_hash(artifact_image_key);

        self.image_root.join(format!("{prefix}-{hash:016x}.bin"))
    }

    /// Resolve the lock path for one stable image key.
    fn lock_path(&self, artifact_image_key: &ArtifactImageKey) -> PathBuf {
        let prefix = artifact_image_file_prefix(artifact_image_key);
        let hash = self.image_key_hash(artifact_image_key);

        self.image_root.join(format!("{prefix}-{hash:016x}.lock"))
    }

    /// Load raw image bytes for one stable image key.
    fn load_bytes(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<Vec<u8>>, ArtifactImageError> {
        let image_path = self.image_path(artifact_image_key);
        let lock_path = self.lock_path(artifact_image_key);
        self.store.with_shared_lock(
            &lock_path,
            || -> Result<Option<Vec<u8>>, ArtifactImageError> {
                if let Some(metadata) = self.store.metadata(&image_path)?
                    && metadata.size_bytes > ARTIFACT_IMAGE_LIMIT_BYTES
                {
                    return Err(ArtifactImageError::SizeLimitExceeded {
                        limit: ARTIFACT_IMAGE_LIMIT_BYTES,
                        actual: metadata.size_bytes,
                    }
                    .into());
                }

                let Some(bytes) = self.store.read(&image_path)? else {
                    return Ok(None);
                };

                self.store.touch(&image_path)?;

                Ok(Some(bytes))
            },
        )
    }

    /// Hash one stable image key for filesystem storage.
    fn image_key_hash(&self, artifact_image_key: &ArtifactImageKey) -> u64 {
        let mut hasher = FxHasher::default();
        artifact_image_key.hash(&mut hasher);
        hasher.finish()
    }
}

impl From<std::io::Error> for ArtifactImageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<CacheStoreError> for ArtifactImageError {
    fn from(error: CacheStoreError) -> Self {
        match error {
            CacheStoreError::Io(error) => Self::Io(error),
        }
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

/// Compute one recursive validation hash from the payload and requirement proofs.
pub fn artifact_validation_hash(
    payload_hash: u64,
    requirements: &[ArtifactImageRequirement],
) -> Result<u64, ArtifactImageError> {
    let mut hasher = FxHasher::default();
    payload_hash.hash(&mut hasher);

    for requirement in requirements {
        let key_bytes =
            postcard::to_allocvec(&requirement.key).map_err(ArtifactImageError::Serialize)?;
        key_bytes.hash(&mut hasher);
        requirement.validation_hash.hash(&mut hasher);
    }

    Ok(hasher.finish())
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

/// Split one serialized artifact image into header and payload bytes.
fn split_artifact_image_bytes(
    bytes: &[u8],
) -> Result<(ArtifactImageHeader, &[u8]), ArtifactImageError> {
    let (header, payload_offset) = split_artifact_image_header_bytes(bytes)?;
    let payload_bytes = &bytes[payload_offset..];

    Ok((header, payload_bytes))
}

/// Split one serialized artifact image header from the leading bytes.
fn split_artifact_image_header_bytes(
    bytes: &[u8],
) -> Result<(ArtifactImageHeader, usize), ArtifactImageError> {
    if bytes.len() < ARTIFACT_IMAGE_HEADER_LENGTH_BYTES {
        return Err(ArtifactImageError::InvalidLayout(
            "missing artifact image header length prefix",
        ));
    }

    let length_bytes: [u8; ARTIFACT_IMAGE_HEADER_LENGTH_BYTES] = bytes[0..4]
        .try_into()
        .map_err(|_| ArtifactImageError::InvalidLayout("invalid artifact image header length"))?;
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

/// Validate one payload hash against an artifact image header.
fn validate_artifact_image_payload_hash<T: Serialize>(
    header: &ArtifactImageHeader,
    payload: &T,
) -> Result<(), ArtifactImageError> {
    let payload_bytes = serialize_payload_with_limit(payload, ARTIFACT_IMAGE_LIMIT_BYTES)?;
    validate_artifact_image_payload_hash_bytes(header, &payload_bytes)
}

/// Validate one payload byte hash against an artifact image header.
fn validate_artifact_image_payload_hash_bytes(
    header: &ArtifactImageHeader,
    payload_bytes: &[u8],
) -> Result<(), ArtifactImageError> {
    let payload_hash = payload_hash_from_bytes(payload_bytes);
    if payload_hash != header.payload_hash {
        return Err(ArtifactImageError::InvalidPayloadHash {
            expected: header.payload_hash,
            found: payload_hash,
        });
    }

    Ok(())
}

/// Validate one recursive validation hash against an artifact image header.
fn validate_artifact_image_validation_hash(
    header: &ArtifactImageHeader,
) -> Result<(), ArtifactImageError> {
    let validation_hash = artifact_validation_hash(header.payload_hash, &header.requirements)?;
    if validation_hash != header.validation_hash {
        return Err(ArtifactImageError::InvalidValidationHash {
            expected: header.validation_hash,
            found: validation_hash,
        });
    }

    Ok(())
}

/// Canonicalize one persisted requirement proof list.
fn canonicalize_artifact_image_requirements(
    mut requirements: Vec<ArtifactImageRequirement>,
) -> Result<Vec<ArtifactImageRequirement>, ArtifactImageError> {
    let mut keyed_requirements = Vec::with_capacity(requirements.len());

    for requirement in requirements.drain(..) {
        let key_bytes =
            postcard::to_allocvec(&requirement.key).map_err(ArtifactImageError::Serialize)?;
        keyed_requirements.push((key_bytes, requirement));
    }

    keyed_requirements.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    keyed_requirements.dedup_by(|left, right| left.1.key == right.1.key);

    Ok(keyed_requirements
        .into_iter()
        .map(|(_, requirement)| requirement)
        .collect())
}

/// Resolve the stable file prefix for one artifact image family.
fn artifact_image_file_prefix(artifact_image_key: &ArtifactImageKey) -> &'static str {
    match artifact_image_key {
        ArtifactImageKey::ModuleGraph { .. } => "module-graph",
        ArtifactImageKey::Ast { .. } => "ast",
        ArtifactImageKey::DirBase { .. } => "dir-base",
        ArtifactImageKey::DirPrepared { .. } => "dir-prepared",
        ArtifactImageKey::DirResolved { .. } => "dir-resolved",
        ArtifactImageKey::DirDeclared { .. } => "dir-declared",
        ArtifactImageKey::DirInterface { .. } => "dir-interface",
        ArtifactImageKey::DirAnalyzed { .. } => "dir-analyzed",
        ArtifactImageKey::DirElaborated { .. } => "dir-elaborated",
        ArtifactImageKey::DirPatched { .. } => "dir-patched",
        ArtifactImageKey::MirBase { .. } => "mir-base",
        ArtifactImageKey::MirOptimized { .. } => "mir-optimized",
        ArtifactImageKey::ModuleArtifact { .. } => "module-artifact",
        ArtifactImageKey::PackageOutput { .. } => "package-output",
        ArtifactImageKey::LanguageEnvironment { .. } => "language-environment",
        ArtifactImageKey::IntrinsicEnvironment { .. } => "intrinsic-environment",
        ArtifactImageKey::LibraryEnvironment { .. } => "library-environment",
    }
}
