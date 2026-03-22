use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rustc_hash::FxHasher;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    CacheStore, CacheStoreError, DEFAULT_LANGUAGE_CACHE_DIR_NAME, DEFAULT_LANGUAGE_CACHE_NAMESPACE,
};

use super::{
    ARTIFACT_IMAGE_HEADER_LENGTH_BYTES, ARTIFACT_IMAGE_LIMIT_BYTES, ArtifactImage,
    ArtifactImageError, ArtifactImageKey,
};

/// Cache backed artifact image reader and writer.
#[derive(Debug)]
pub struct ArtifactStore<'a> {
    /// The cache store backing the images.
    store: &'a dyn CacheStore,
    /// Base directory for persisted artifact images.
    image_root: PathBuf,
}

impl<'a> ArtifactStore<'a> {
    /// Create an artifact store for a cache root.
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
    ) -> Result<Option<super::ArtifactImageHeader>, ArtifactImageError> {
        let image_path = self.image_path(artifact_image_key);
        let lock_path = self.lock_path(artifact_image_key);
        self.store.with_shared_lock(
            &lock_path,
            || -> Result<Option<super::ArtifactImageHeader>, ArtifactImageError> {
                // guard against oversized payloads before reading
                if let Some(metadata) = self.store.metadata(&image_path)?
                    && metadata.size_bytes > ARTIFACT_IMAGE_LIMIT_BYTES
                {
                    return Err(ArtifactImageError::SizeLimitExceeded {
                        limit: ARTIFACT_IMAGE_LIMIT_BYTES,
                        actual: metadata.size_bytes,
                    }
                    .into());
                }

                // read the fixed header length prefix first
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

                // read the exact header bytes next
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
                let header = super::ArtifactImageHeader::deserialize_prefixed(&header_bytes)?;

                // refresh access tracking after a successful load
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
                // serialize the image before writing
                let bytes = image.serialize()?;

                // publish the new bytes atomically
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
                // guard against oversized payloads before reading
                if let Some(metadata) = self.store.metadata(&image_path)?
                    && metadata.size_bytes > ARTIFACT_IMAGE_LIMIT_BYTES
                {
                    return Err(ArtifactImageError::SizeLimitExceeded {
                        limit: ARTIFACT_IMAGE_LIMIT_BYTES,
                        actual: metadata.size_bytes,
                    }
                    .into());
                }

                // read the image bytes
                let Some(bytes) = self.store.read(&image_path)? else {
                    return Ok(None);
                };

                // refresh access tracking after a successful load
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        ArtifactImageKey, ArtifactStore, DiskCacheStore, EmitFormat, EnvSnapshot, Platform,
        ProfileFlags, ProfileKey, Runtime,
    };

    /// Build one stable profile key for artifact store path tests.
    fn test_profile_key() -> ProfileKey {
        ProfileKey::new(
            EmitFormat::Js,
            Runtime::Node,
            Platform::Web,
            None,
            None,
            None,
            Vec::new(),
            false,
            false,
            false,
            EnvSnapshot::Whitelist {
                keys: Vec::new(),
                hash: 0,
            },
            ProfileFlags::default(),
        )
    }

    /// Place artifact images under the language namespace.
    #[test]
    fn test_artifact_store_uses_language_namespace() {
        let store = DiskCacheStore::new();
        let cache_root = PathBuf::from("/workspace/.destack");
        let artifact_store = ArtifactStore::new(&store, &cache_root);
        let artifact_key = ArtifactImageKey::LanguageEnvironment {
            profile: test_profile_key(),
        };

        assert_eq!(
            artifact_store.image_root,
            cache_root.join("language").join("cache")
        );
        assert!(
            artifact_store
                .image_path(&artifact_key)
                .starts_with(cache_root.join("language").join("cache"))
        );
        assert!(
            artifact_store
                .image_path(&artifact_key)
                .file_name()
                .unwrap_or_else(|| panic!("expected artifact image file name"))
                .to_string_lossy()
                .starts_with("language-environment-")
        );
        assert!(
            artifact_store
                .lock_path(&artifact_key)
                .file_name()
                .unwrap_or_else(|| panic!("expected artifact lock file name"))
                .to_string_lossy()
                .starts_with("language-environment-")
        );
    }
}

impl From<CacheStoreError> for ArtifactImageError {
    fn from(error: CacheStoreError) -> Self {
        match error {
            CacheStoreError::Io(error) => Self::Io(error),
        }
    }
}

impl From<ArtifactImageError> for CacheStoreError {
    fn from(error: ArtifactImageError) -> Self {
        Self::Io(std::io::Error::other(error))
    }
}

/// Return the stable file prefix for one persisted artifact image family.
fn artifact_image_file_prefix(artifact_image_key: &ArtifactImageKey) -> &'static str {
    match artifact_image_key {
        ArtifactImageKey::ModuleGraph { .. } => "module-graph",
        ArtifactImageKey::DirBase { .. } => "dir-base",
        ArtifactImageKey::Ast { .. } => "ast",
        ArtifactImageKey::DirPrepared { .. } => "dir-prepared",
        ArtifactImageKey::DirResolved { .. } => "dir-resolved",
        ArtifactImageKey::DirDeclared { .. } => "dir-declared",
        ArtifactImageKey::DirInterface { .. } => "dir-interface",
        ArtifactImageKey::DirAnalyzed { .. } => "dir-analyzed",
        ArtifactImageKey::DirElaborated { .. } => "dir-elaborated",
        ArtifactImageKey::DirPatched { .. } => "dir-patched",
        ArtifactImageKey::MirBase { .. } => "mir-base",
        ArtifactImageKey::MirOptimized { .. } => "mir-optimized",
        ArtifactImageKey::ModuleOutput { .. } => "module-output",
        ArtifactImageKey::PackageOutput { .. } => "package-output",
        ArtifactImageKey::LanguageEnvironment { .. } => "language-environment",
        ArtifactImageKey::IntrinsicEnvironment { .. } => "intrinsic-environment",
        ArtifactImageKey::LibraryEnvironment { .. } => "library-environment",
    }
}
