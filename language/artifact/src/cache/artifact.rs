use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{
    ARTIFACT_IMAGE_LIMIT_BYTES, ArtifactImage, ArtifactImageCacheLayout, ArtifactImageError,
    ArtifactVersion, CacheStore, CacheStoreError,
};

/// Cache of canonical artifact images.
#[derive(Debug)]
pub struct ArtifactImageCache<'a> {
    /// The cache store backing this artifact image cache.
    store: &'a dyn CacheStore,
    /// The artifact image cache layout.
    layout: ArtifactImageCacheLayout,
}

impl<'a> ArtifactImageCache<'a> {
    /// Create one artifact image cache.
    pub fn new(store: &'a dyn CacheStore, layout: &ArtifactImageCacheLayout) -> Self {
        Self {
            store,
            layout: layout.clone(),
        }
    }

    /// Load one exact artifact image.
    pub fn load<T>(
        &self,
        expected: &ArtifactVersion,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned,
    {
        // image bytes
        let Some(bytes) = self.read_image_bytes(expected)? else {
            return Ok(None);
        };

        // payload decode
        let image = ArtifactImage::<T>::deserialize(&bytes)?;
        if image.version() != *expected {
            return Err(ArtifactImageError::Version {
                expected: *expected,
                found: image.version(),
            });
        }

        Ok(Some(image))
    }

    /// Save one exact artifact image.
    pub fn save<T>(&self, image: &ArtifactImage<T>) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        // image bytes
        let bytes = image.serialize()?;

        // publish image
        self.with_write_lock(|| {
            self.write_image_bytes(&image.version(), &bytes)?;

            Ok(())
        })?;

        Ok(())
    }

    /// Run one write operation under the artifact cache lock.
    fn with_write_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ArtifactImageError>,
    ) -> Result<T, ArtifactImageError> {
        let lock_path = self.layout.image_lock_path();
        let mut operation = Some(operation);
        let mut output = None;

        self.store.with_exclusive_lock(&lock_path, &mut || {
            let Some(operation) = operation.take() else {
                panic!("artifact cache lock should run exactly once");
            };
            let result = operation();
            output = Some(result);
        })?;

        match output {
            Some(output) => output,
            None => panic!("artifact cache lock should produce one value"),
        }
    }

    /// Read one exact image.
    fn read_image_bytes(
        &self,
        expected: &ArtifactVersion,
    ) -> Result<Option<Vec<u8>>, ArtifactImageError> {
        // image bytes
        let image_path = self.image_path(expected)?;
        let Some(byte_len) = self.store.byte_len(&image_path)? else {
            return Ok(None);
        };
        if byte_len > ARTIFACT_IMAGE_LIMIT_BYTES {
            return Err(ArtifactImageError::Size {
                limit: ARTIFACT_IMAGE_LIMIT_BYTES,
                actual: byte_len,
            });
        }

        let Some(bytes) = self.store.read(&image_path)? else {
            return Ok(None);
        };

        Ok(Some(bytes))
    }

    /// Write one exact image blob.
    fn write_image_bytes(
        &self,
        version: &ArtifactVersion,
        bytes: &[u8],
    ) -> Result<(), ArtifactImageError> {
        let image_path = self.image_path(version)?;
        if let Some(existing_bytes) = self.store.read(&image_path)? {
            if existing_bytes != bytes {
                return Err(ArtifactImageError::Conflict { version: *version });
            }

            return Ok(());
        }

        match self.store.write_once(&image_path, bytes) {
            Ok(()) => {}
            Err(CacheStoreError::AlreadyExists) => {
                let Some(existing_bytes) = self.store.read(&image_path)? else {
                    return Err(ArtifactImageError::Corrupt(
                        "cache entry disappeared after write conflict",
                    ));
                };
                if existing_bytes != bytes {
                    return Err(ArtifactImageError::Conflict { version: *version });
                }
            }
            Err(error) => return Err(error.into()),
        }

        Ok(())
    }

    /// Return the cached image path for one exact header.
    fn image_path(&self, version: &ArtifactVersion) -> Result<PathBuf, ArtifactImageError> {
        let image_key_bytes = postcard::to_allocvec(version).map_err(ArtifactImageError::Codec)?;
        let image_token = artifact_image_token(&image_key_bytes);
        let shard = &image_token[0..2];

        Ok(self
            .layout
            .image_root()
            .join(shard)
            .join(format!("{image_token}.bin")))
    }
}

/// Return the stable path token for one artifact image version.
fn artifact_image_token(bytes: &[u8]) -> String {
    let digest = blake3::hash(bytes);

    digest.to_hex().to_string()
}

impl From<CacheStoreError> for ArtifactImageError {
    fn from(error: CacheStoreError) -> Self {
        Self::Cache(error)
    }
}
