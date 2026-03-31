use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use postcard::Error as PostcardError;
use rustc_hash::FxHasher;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use destack_artifact::{CacheStore, CacheStoreError};
use destack_core::StringPool;

/// The magic prefix used to identify repository images.
pub const REPOSITORY_IMAGE_MAGIC: [u8; 4] = *b"RIMG";
/// The current repository image format version.
pub const REPOSITORY_IMAGE_FORMAT_VERSION: u32 = 1;
/// The encoded header length prefix width.
pub const REPOSITORY_IMAGE_HEADER_LENGTH_BYTES: usize = 4;
/// The maximum repository image size.
pub const REPOSITORY_IMAGE_LIMIT_BYTES: u64 = 64 * 1024 * 1024;
/// The cache namespace for repository images.
pub const DEFAULT_REPOSITORY_CACHE_NAMESPACE: &str = "repository";
/// The cache directory for repository images.
pub const DEFAULT_REPOSITORY_CACHE_DIR_NAME: &str = "image";

/// Stable repository image identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RepositoryImageKey {
    /// The persisted repository snapshot.
    Snapshot,
}

impl RepositoryImageKey {
    /// Return the image file prefix for this key.
    fn file_prefix(&self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
        }
    }
}

/// Common header for one serialized repository image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryImageHeader {
    /// The magic prefix used to identify repository images.
    pub magic: [u8; 4],
    /// The repository image format version.
    pub format_version: u32,
    /// The compiler version that produced the image.
    pub compiler_version: String,
    /// The stable repository image key for the serialized payload.
    pub repository_image_key: RepositoryImageKey,
    /// Hash of the serialized payload bytes.
    pub payload_hash: u64,
    /// Hash of the payload hash and header identity.
    pub validation_hash: u64,
}

impl RepositoryImageHeader {
    /// Create a new repository image header.
    pub fn new(repository_image_key: RepositoryImageKey, compiler_version: String) -> Self {
        Self {
            magic: REPOSITORY_IMAGE_MAGIC,
            format_version: REPOSITORY_IMAGE_FORMAT_VERSION,
            compiler_version,
            repository_image_key,
            payload_hash: 0,
            validation_hash: 0,
        }
    }

    /// Validate this header against one expected runtime identity.
    pub fn validate(
        &self,
        expected_key: &RepositoryImageKey,
        expected_compiler_version: &str,
    ) -> Result<(), RepositoryImageError> {
        if self.magic != REPOSITORY_IMAGE_MAGIC {
            return Err(RepositoryImageError::InvalidMagic {
                expected: REPOSITORY_IMAGE_MAGIC,
                found: self.magic,
            });
        }

        if self.format_version != REPOSITORY_IMAGE_FORMAT_VERSION {
            return Err(RepositoryImageError::InvalidFormatVersion {
                expected: REPOSITORY_IMAGE_FORMAT_VERSION,
                found: self.format_version,
            });
        }

        if &self.repository_image_key != expected_key {
            return Err(RepositoryImageError::InvalidRepositoryImageKey {
                expected: expected_key.clone(),
                found: self.repository_image_key.clone(),
            });
        }

        if self.compiler_version != expected_compiler_version {
            return Err(RepositoryImageError::InvalidCompilerVersion {
                expected: expected_compiler_version.to_string(),
                found: self.compiler_version.clone(),
            });
        }

        Ok(())
    }
}

/// One serialized repository image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryImage<T> {
    /// The image header.
    pub header: RepositoryImageHeader,
    /// The serialized payload.
    pub payload: T,
}

impl<T> RepositoryImage<T>
where
    T: Serialize,
{
    /// Create a repository image with a computed payload hash.
    pub fn new(
        mut header: RepositoryImageHeader,
        payload: T,
    ) -> Result<Self, RepositoryImageError> {
        header.payload_hash = repository_payload_hash_from_payload(&payload)?;
        header.validation_hash = repository_validation_hash(
            &header.repository_image_key,
            &header.compiler_version,
            header.payload_hash,
        )?;

        Ok(Self { header, payload })
    }

    /// Serialize this repository image to bytes.
    pub fn serialize(&self) -> Result<Vec<u8>, RepositoryImageError> {
        self.serialize_with_limit(REPOSITORY_IMAGE_LIMIT_BYTES)
    }

    /// Serialize this repository image to bytes with one explicit size limit.
    pub(crate) fn serialize_with_limit(&self, limit: u64) -> Result<Vec<u8>, RepositoryImageError> {
        let header_bytes = serialize_payload_with_limit(&self.header, limit)?;
        let header_length = u32::try_from(header_bytes.len()).map_err(|_| {
            RepositoryImageError::SizeLimitExceeded {
                limit,
                actual: header_bytes.len() as u64,
            }
        })?;
        let payload_bytes = serialize_payload_with_limit(&self.payload, limit)?;
        let total_bytes = REPOSITORY_IMAGE_HEADER_LENGTH_BYTES as u64
            + header_bytes.len() as u64
            + payload_bytes.len() as u64;

        if total_bytes > limit {
            return Err(RepositoryImageError::SizeLimitExceeded {
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

impl<T> RepositoryImage<T>
where
    T: DeserializeOwned,
{
    /// Deserialize one repository image from bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<Self, RepositoryImageError> {
        let (header, payload_bytes) = split_repository_image_bytes(bytes)?;
        validate_repository_image_payload_hash_bytes(&header, payload_bytes)?;
        validate_repository_image_validation_hash(&header)?;

        let payload =
            deserialize_payload_with_limit::<T>(payload_bytes, REPOSITORY_IMAGE_LIMIT_BYTES)?;

        Ok(Self { header, payload })
    }
}

impl RepositoryImageHeader {
    /// Deserialize one prefixed repository image header from bytes.
    pub fn deserialize_prefixed(bytes: &[u8]) -> Result<Self, RepositoryImageError> {
        let (header, _) = split_repository_image_header_bytes(bytes)?;
        validate_repository_image_validation_hash(&header)?;

        Ok(header)
    }
}

/// One persisted repository snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySnapshot {
    /// The persisted shared string pool.
    pub strings: StringPool,
}

impl RepositorySnapshot {
    /// Create a repository snapshot from the current shared string pool.
    pub fn from_strings(strings: &StringPool) -> Self {
        Self {
            strings: strings.clone(),
        }
    }
}

/// Errors that can occur while reading or writing repository images.
#[derive(Debug)]
pub enum RepositoryImageError {
    /// The image header magic did not match.
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },
    /// The image header format version did not match.
    InvalidFormatVersion { expected: u32, found: u32 },
    /// The image key did not match the expected runtime key.
    InvalidRepositoryImageKey {
        /// The expected image key.
        expected: RepositoryImageKey,
        /// The found image key.
        found: RepositoryImageKey,
    },
    /// The image compiler version did not match the current runtime.
    InvalidCompilerVersion {
        /// The expected compiler version.
        expected: String,
        /// The found compiler version.
        found: String,
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
    /// The validation hash did not match.
    InvalidValidationHash { expected: u64, found: u64 },
    /// The image backend failed to read or write.
    Io(std::io::Error),
}

impl fmt::Display for RepositoryImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic { expected, found } => {
                write!(
                    formatter,
                    "invalid repository image magic: expected {expected:?}, found {found:?}"
                )
            }
            Self::InvalidFormatVersion { expected, found } => {
                write!(
                    formatter,
                    "invalid repository image format version: expected {expected}, found {found}"
                )
            }
            Self::InvalidRepositoryImageKey { expected, found } => {
                write!(
                    formatter,
                    "invalid repository image key: expected {expected:?}, found {found:?}"
                )
            }
            Self::InvalidCompilerVersion { expected, found } => {
                write!(
                    formatter,
                    "invalid repository image compiler version: expected {expected}, found {found}"
                )
            }
            Self::Serialize(error) => {
                write!(formatter, "repository image serialize failed: {error}")
            }
            Self::Deserialize(error) => {
                write!(formatter, "repository image deserialize failed: {error}")
            }
            Self::InvalidLayout(message) => {
                write!(formatter, "invalid repository image layout: {message}")
            }
            Self::SizeLimitExceeded { limit, actual } => {
                write!(
                    formatter,
                    "repository image size limit exceeded: limit {limit} bytes, actual {actual} bytes"
                )
            }
            Self::InvalidPayloadHash { expected, found } => {
                write!(
                    formatter,
                    "invalid repository image payload hash: expected {expected:016x}, found {found:016x}"
                )
            }
            Self::InvalidValidationHash { expected, found } => {
                write!(
                    formatter,
                    "invalid repository image validation hash: expected {expected:016x}, found {found:016x}"
                )
            }
            Self::Io(error) => write!(formatter, "repository image io error: {error}"),
        }
    }
}

impl std::error::Error for RepositoryImageError {}

/// One image store for persisted repository snapshots.
#[derive(Debug)]
pub struct RepositoryImageStore<'a> {
    /// The cache store backing the images.
    store: &'a dyn CacheStore,
    /// Base directory for persisted repository images.
    image_root: PathBuf,
}

impl<'a> RepositoryImageStore<'a> {
    /// Create a repository image store for a cache root.
    pub fn new(store: &'a dyn CacheStore, cache_root: &Path) -> Self {
        let image_root = cache_root
            .join(DEFAULT_REPOSITORY_CACHE_NAMESPACE)
            .join(DEFAULT_REPOSITORY_CACHE_DIR_NAME);

        Self { store, image_root }
    }

    /// Load one persisted repository image by stable image key.
    pub fn load<T>(
        &self,
        repository_image_key: &RepositoryImageKey,
    ) -> Result<Option<RepositoryImage<T>>, RepositoryImageError>
    where
        T: DeserializeOwned,
    {
        let Some(bytes) = self.load_bytes(repository_image_key)? else {
            return Ok(None);
        };
        let image = RepositoryImage::<T>::deserialize(&bytes)?;

        Ok(Some(image))
    }

    /// Load one persisted repository image header by stable image key.
    pub fn load_header(
        &self,
        repository_image_key: &RepositoryImageKey,
    ) -> Result<Option<RepositoryImageHeader>, RepositoryImageError> {
        let image_path = self.image_path(repository_image_key);
        let lock_path = self.lock_path(repository_image_key);

        self.store.with_shared_lock(
            &lock_path,
            || -> Result<Option<RepositoryImageHeader>, RepositoryImageError> {
                if let Some(metadata) = self.store.metadata(&image_path)?
                    && metadata.size_bytes > REPOSITORY_IMAGE_LIMIT_BYTES
                {
                    return Err(RepositoryImageError::SizeLimitExceeded {
                        limit: REPOSITORY_IMAGE_LIMIT_BYTES,
                        actual: metadata.size_bytes,
                    });
                }

                let Some(prefix) = self
                    .store
                    .read_prefix(&image_path, REPOSITORY_IMAGE_HEADER_LENGTH_BYTES)?
                else {
                    return Ok(None);
                };

                if prefix.len() < REPOSITORY_IMAGE_HEADER_LENGTH_BYTES {
                    return Err(RepositoryImageError::InvalidLayout(
                        "missing repository image header length prefix",
                    ));
                }

                let header_length = u32::from_le_bytes(
                    prefix[0..REPOSITORY_IMAGE_HEADER_LENGTH_BYTES]
                        .try_into()
                        .map_err(|_| {
                            RepositoryImageError::InvalidLayout(
                                "invalid repository image header length",
                            )
                        })?,
                ) as usize;
                let total_prefix = REPOSITORY_IMAGE_HEADER_LENGTH_BYTES + header_length;
                let Some(header_bytes) = self.store.read_prefix(&image_path, total_prefix)? else {
                    return Ok(None);
                };
                let header = RepositoryImageHeader::deserialize_prefixed(&header_bytes)?;

                self.store.touch(&image_path)?;

                Ok(Some(header))
            },
        )
    }

    /// Save one persisted repository image.
    pub fn save<T>(&self, image: &RepositoryImage<T>) -> Result<(), RepositoryImageError>
    where
        T: Serialize,
    {
        let image_path = self.image_path(&image.header.repository_image_key);
        let lock_path = self.lock_path(&image.header.repository_image_key);

        self.store
            .with_exclusive_lock(&lock_path, || -> Result<(), RepositoryImageError> {
                let bytes = image.serialize()?;
                self.store.write_atomic(&image_path, &bytes)?;

                Ok(())
            })
    }

    /// Resolve the image path for one stable image key.
    fn image_path(&self, repository_image_key: &RepositoryImageKey) -> PathBuf {
        let prefix = repository_image_key.file_prefix();
        let hash = self.image_key_hash(repository_image_key);

        self.image_root.join(format!("{prefix}-{hash:016x}.bin"))
    }

    /// Resolve the lock path for one stable image key.
    fn lock_path(&self, repository_image_key: &RepositoryImageKey) -> PathBuf {
        let prefix = repository_image_key.file_prefix();
        let hash = self.image_key_hash(repository_image_key);

        self.image_root.join(format!("{prefix}-{hash:016x}.lock"))
    }

    /// Load raw image bytes for one stable image key.
    fn load_bytes(
        &self,
        repository_image_key: &RepositoryImageKey,
    ) -> Result<Option<Vec<u8>>, RepositoryImageError> {
        let image_path = self.image_path(repository_image_key);
        let lock_path = self.lock_path(repository_image_key);

        self.store.with_shared_lock(
            &lock_path,
            || -> Result<Option<Vec<u8>>, RepositoryImageError> {
                if let Some(metadata) = self.store.metadata(&image_path)?
                    && metadata.size_bytes > REPOSITORY_IMAGE_LIMIT_BYTES
                {
                    return Err(RepositoryImageError::SizeLimitExceeded {
                        limit: REPOSITORY_IMAGE_LIMIT_BYTES,
                        actual: metadata.size_bytes,
                    });
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
    fn image_key_hash(&self, repository_image_key: &RepositoryImageKey) -> u64 {
        let mut hasher = FxHasher::default();
        repository_image_key.hash(&mut hasher);
        hasher.finish()
    }
}

impl From<std::io::Error> for RepositoryImageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<CacheStoreError> for RepositoryImageError {
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

/// Compute one payload hash for a serializable repository payload.
fn repository_payload_hash_from_payload<T: Serialize>(
    payload: &T,
) -> Result<u64, RepositoryImageError> {
    let bytes = serialize_payload_with_limit(payload, REPOSITORY_IMAGE_LIMIT_BYTES)?;

    Ok(payload_hash_from_bytes(&bytes))
}

/// Compute one validation hash from the image key, compiler version, and payload hash.
fn repository_validation_hash(
    repository_image_key: &RepositoryImageKey,
    compiler_version: &str,
    payload_hash: u64,
) -> Result<u64, RepositoryImageError> {
    let mut hasher = FxHasher::default();
    let key_bytes =
        postcard::to_allocvec(repository_image_key).map_err(RepositoryImageError::Serialize)?;

    key_bytes.hash(&mut hasher);
    compiler_version.hash(&mut hasher);
    payload_hash.hash(&mut hasher);

    Ok(hasher.finish())
}

/// Serialize one payload with one explicit size limit.
fn serialize_payload_with_limit<T: Serialize>(
    payload: &T,
    limit: u64,
) -> Result<Vec<u8>, RepositoryImageError> {
    let bytes = postcard::to_allocvec(payload).map_err(RepositoryImageError::Serialize)?;
    let actual = bytes.len() as u64;

    if actual > limit {
        return Err(RepositoryImageError::SizeLimitExceeded { limit, actual });
    }

    Ok(bytes)
}

/// Deserialize one payload with one explicit size limit.
fn deserialize_payload_with_limit<T: DeserializeOwned>(
    bytes: &[u8],
    limit: u64,
) -> Result<T, RepositoryImageError> {
    let actual = bytes.len() as u64;

    if actual > limit {
        return Err(RepositoryImageError::SizeLimitExceeded { limit, actual });
    }

    postcard::from_bytes(bytes).map_err(RepositoryImageError::Deserialize)
}

/// Split one repository image into header and payload bytes.
fn split_repository_image_bytes(
    bytes: &[u8],
) -> Result<(RepositoryImageHeader, &[u8]), RepositoryImageError> {
    let (header, payload_offset) = split_repository_image_header_bytes(bytes)?;

    Ok((header, &bytes[payload_offset..]))
}

/// Split one repository image prefix into header and payload offset.
fn split_repository_image_header_bytes(
    bytes: &[u8],
) -> Result<(RepositoryImageHeader, usize), RepositoryImageError> {
    if bytes.len() < REPOSITORY_IMAGE_HEADER_LENGTH_BYTES {
        return Err(RepositoryImageError::InvalidLayout(
            "missing repository image header length prefix",
        ));
    }

    let header_length = u32::from_le_bytes(
        bytes[0..REPOSITORY_IMAGE_HEADER_LENGTH_BYTES]
            .try_into()
            .map_err(|_| {
                RepositoryImageError::InvalidLayout("invalid repository image header length")
            })?,
    ) as usize;
    let payload_offset = REPOSITORY_IMAGE_HEADER_LENGTH_BYTES + header_length;

    if bytes.len() < payload_offset {
        return Err(RepositoryImageError::InvalidLayout(
            "repository image payload missing after header",
        ));
    }

    let header = deserialize_payload_with_limit::<RepositoryImageHeader>(
        &bytes[REPOSITORY_IMAGE_HEADER_LENGTH_BYTES..payload_offset],
        REPOSITORY_IMAGE_LIMIT_BYTES,
    )?;

    Ok((header, payload_offset))
}

/// Validate one repository image payload hash against serialized payload bytes.
fn validate_repository_image_payload_hash_bytes(
    header: &RepositoryImageHeader,
    bytes: &[u8],
) -> Result<(), RepositoryImageError> {
    let found = payload_hash_from_bytes(bytes);

    if found != header.payload_hash {
        return Err(RepositoryImageError::InvalidPayloadHash {
            expected: header.payload_hash,
            found,
        });
    }

    Ok(())
}

/// Validate one repository image validation hash.
fn validate_repository_image_validation_hash(
    header: &RepositoryImageHeader,
) -> Result<(), RepositoryImageError> {
    let found = repository_validation_hash(
        &header.repository_image_key,
        &header.compiler_version,
        header.payload_hash,
    )?;

    if found != header.validation_hash {
        return Err(RepositoryImageError::InvalidValidationHash {
            expected: header.validation_hash,
            found,
        });
    }

    Ok(())
}
