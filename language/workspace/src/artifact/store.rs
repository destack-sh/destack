use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rustc_hash::FxHasher;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    ARTIFACT_STORE_DIR_NAME, CacheStore, CacheStoreError, DEFAULT_COMPILER_CACHE_NAMESPACE,
};

use super::{ARTIFACT_IMAGE_LIMIT_BYTES, ArtifactImage, ArtifactImageError, ArtifactImageKey};

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
            .join(DEFAULT_COMPILER_CACHE_NAMESPACE)
            .join(ARTIFACT_STORE_DIR_NAME);

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
        let image_path = self.image_path(artifact_image_key);
        let lock_path = self.lock_path(artifact_image_key);
        self.store
            .with_shared_lock(
                &lock_path,
                || -> Result<Option<ArtifactImage<T>>, CacheStoreError> {
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

                    // read and decode the image
                    let Some(bytes) = self.store.read(&image_path)? else {
                        return Ok(None);
                    };
                    let image =
                        ArtifactImage::<T>::deserialize(&bytes).map_err(CacheStoreError::from)?;

                    // refresh access tracking after a successful load
                    self.store.touch(&image_path)?;

                    Ok(Some(image))
                },
            )
            .map_err(ArtifactImageError::from)
    }

    /// Save one persisted artifact image.
    pub fn save<T>(&self, image: &ArtifactImage<T>) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        let image_path = self.image_path(&image.header.artifact_image_key);
        let lock_path = self.lock_path(&image.header.artifact_image_key);
        self.store
            .with_exclusive_lock(&lock_path, || -> Result<(), CacheStoreError> {
                // serialize the image before writing
                let bytes = image.serialize().map_err(CacheStoreError::from)?;

                // publish the new bytes atomically
                self.store.write_atomic(&image_path, &bytes)?;

                Ok(())
            })
            .map_err(ArtifactImageError::from)
    }

    /// Resolve the image path for one stable image key.
    fn image_path(&self, artifact_image_key: &ArtifactImageKey) -> PathBuf {
        let family = artifact_image_family_name(artifact_image_key);
        let hash = self.image_key_hash(artifact_image_key);

        self.image_root
            .join(family)
            .join(format!("{hash:016x}.bin"))
    }

    /// Resolve the lock path for one stable image key.
    fn lock_path(&self, artifact_image_key: &ArtifactImageKey) -> PathBuf {
        let family = artifact_image_family_name(artifact_image_key);
        let hash = self.image_key_hash(artifact_image_key);

        self.image_root
            .join(family)
            .join(format!("{hash:016x}.lock"))
    }

    /// Hash one stable image key for filesystem storage.
    fn image_key_hash(&self, artifact_image_key: &ArtifactImageKey) -> u64 {
        let mut hasher = FxHasher::default();
        artifact_image_key.hash(&mut hasher);
        hasher.finish()
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

/// Return the stable directory name for one persisted artifact image family.
fn artifact_image_family_name(artifact_image_key: &ArtifactImageKey) -> &'static str {
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
