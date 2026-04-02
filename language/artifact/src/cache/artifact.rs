use std::fmt;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{
    ARTIFACT_IMAGE_LIMIT_BYTES, ArtifactCacheLayout, ArtifactImage, ArtifactImageError,
    ArtifactImageHeader, ArtifactImageKey, CacheStore, CacheStoreError,
};

const ARTIFACT_CONTENT_ID_LENGTH_BYTES: usize = 16;

/// Immutable persisted artifact content id.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactContentId(pub [u8; ARTIFACT_CONTENT_ID_LENGTH_BYTES]);

impl ArtifactContentId {
    /// Create one content id from canonical artifact image bytes.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let digest = blake3::hash(bytes);
        let mut content_id = [0_u8; ARTIFACT_CONTENT_ID_LENGTH_BYTES];
        content_id.copy_from_slice(&digest.as_bytes()[..ARTIFACT_CONTENT_ID_LENGTH_BYTES]);
        Self(content_id)
    }

    /// Return this content id as lowercase hex.
    pub fn to_hex(self) -> String {
        let mut hex = String::with_capacity(self.0.len() * 2);

        for byte in self.0 {
            use std::fmt::Write;

            write!(&mut hex, "{byte:02x}").expect("writing to one string should succeed");
        }

        hex
    }
}

impl fmt::Debug for ArtifactContentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ac{}", self.to_hex())
    }
}

impl fmt::Display for ArtifactContentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "ac{}", self.to_hex())
    }
}

/// Persisted cache of canonical artifact images.
#[derive(Debug)]
pub struct ArtifactCache<'a> {
    /// The cache store backing this persisted cache.
    store: &'a dyn CacheStore,
    /// The persisted artifact cache layout.
    layout: ArtifactCacheLayout,
}

impl<'a> ArtifactCache<'a> {
    /// Create one persisted artifact cache.
    pub fn new(store: &'a dyn CacheStore, layout: &ArtifactCacheLayout) -> Self {
        Self {
            store,
            layout: layout.clone(),
        }
    }

    /// Load one current artifact image by stable key.
    pub fn load<T>(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned,
    {
        // current bytes
        let Some((content_id, bytes)) =
            self.with_lock(|| self.read_current_image(artifact_image_key))?
        else {
            return Ok(None);
        };

        // content id validation
        self.validate_content_id(content_id, &bytes)?;

        // payload decode
        let image = ArtifactImage::<T>::deserialize(&bytes)?;

        Ok(Some(image))
    }

    /// Load one current artifact image header by stable key.
    pub fn load_header(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        // current bytes
        let Some((content_id, bytes)) =
            self.with_lock(|| self.read_current_image(artifact_image_key))?
        else {
            return Ok(None);
        };

        // content id validation
        self.validate_content_id(content_id, &bytes)?;

        // header decode
        let (header, _) = ArtifactImageHeader::split_from_bytes(&bytes)?;

        Ok(Some(header))
    }

    /// Load one current artifact content id by stable key.
    pub fn load_current_content_id(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactContentId>, ArtifactImageError> {
        self.with_lock(|| self.read_current_content_id(artifact_image_key))
    }

    /// Persist one artifact image and set its current content id for the stable key.
    pub fn save<T>(&self, image: &ArtifactImage<T>) -> Result<ArtifactContentId, ArtifactImageError>
    where
        T: Serialize,
    {
        // content bytes
        let bytes = image.serialize()?;
        let content_id = ArtifactContentId::from_bytes(&bytes);

        // publish content
        self.with_lock(|| {
            self.write_content_bytes(content_id, &bytes)?;
            self.write_current_content_id(&image.header.artifact_image_key, content_id)?;
            self.prune_non_current_contents()?;

            Ok(())
        })?;

        Ok(content_id)
    }

    /// Run one cache operation under the artifact cache lock.
    fn with_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ArtifactImageError>,
    ) -> Result<T, ArtifactImageError> {
        let lock_path = self.layout.artifact_lock_path();
        let mut operation = Some(operation);
        let mut output = None;

        self.store.with_lock(&lock_path, &mut || {
            let result = operation
                .take()
                .expect("artifact cache lock should run exactly once")();
            output = Some(result);
        })?;

        output.expect("artifact cache lock should produce one value")
    }

    /// Read one current content id and its bytes.
    fn read_current_image(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<(ArtifactContentId, Vec<u8>)>, ArtifactImageError> {
        // current content id
        let Some(content_id) = self.read_current_content_id(artifact_image_key)? else {
            return Ok(None);
        };

        // content bytes
        let content_path = self.content_path(content_id);
        let Some(metadata) = self.store.metadata(&content_path)? else {
            return Ok(None);
        };
        if metadata.size_bytes > ARTIFACT_IMAGE_LIMIT_BYTES {
            return Err(ArtifactImageError::SizeLimitExceeded {
                limit: ARTIFACT_IMAGE_LIMIT_BYTES,
                actual: metadata.size_bytes,
            });
        }

        let Some(bytes) = self.store.read(&content_path)? else {
            return Ok(None);
        };

        // liveness hint
        self.store.touch(&content_path)?;

        Ok(Some((content_id, bytes)))
    }

    /// Read one current content id for one stable image key.
    fn read_current_content_id(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<Option<ArtifactContentId>, ArtifactImageError> {
        let current_path = self.current_content_id_path(artifact_image_key)?;
        let Some(bytes) = self.store.read(&current_path)? else {
            return Ok(None);
        };

        postcard::from_bytes(&bytes).map_err(ArtifactImageError::Deserialize)
    }

    /// Write one current content id for one stable image key.
    fn write_current_content_id(
        &self,
        artifact_image_key: &ArtifactImageKey,
        content_id: ArtifactContentId,
    ) -> Result<(), ArtifactImageError> {
        let current_path = self.current_content_id_path(artifact_image_key)?;
        let bytes = postcard::to_allocvec(&content_id).map_err(ArtifactImageError::Serialize)?;

        self.store.write(&current_path, &bytes)?;

        Ok(())
    }

    /// Write one content blob when it does not already exist.
    fn write_content_bytes(
        &self,
        content_id: ArtifactContentId,
        bytes: &[u8],
    ) -> Result<(), ArtifactImageError> {
        let content_path = self.content_path(content_id);

        if !self.store.exists(&content_path)? {
            self.store.write(&content_path, bytes)?;
        }

        Ok(())
    }

    /// Remove persisted contents that are no longer current.
    fn prune_non_current_contents(&self) -> Result<(), ArtifactImageError> {
        let mut reachable_content_paths = std::collections::HashSet::new();

        // current content ids
        for current_path in self.store.list(&self.layout.current_root())? {
            let Some(bytes) = self.store.read(&current_path)? else {
                continue;
            };
            let content_id = postcard::from_bytes::<ArtifactContentId>(&bytes)
                .map_err(ArtifactImageError::Deserialize)?;

            reachable_content_paths.insert(self.content_path(content_id));
        }

        // unreachable contents
        for content_path in self.store.list(&self.layout.content_root())? {
            if reachable_content_paths.contains(&content_path) {
                continue;
            }

            self.store.remove(&content_path)?;
        }

        Ok(())
    }

    /// Validate that one content id matches one canonical byte payload.
    fn validate_content_id(
        &self,
        expected_content_id: ArtifactContentId,
        bytes: &[u8],
    ) -> Result<(), ArtifactImageError> {
        let actual_content_id = ArtifactContentId::from_bytes(bytes);
        if actual_content_id != expected_content_id {
            return Err(ArtifactImageError::InvalidContentId {
                expected: expected_content_id,
                found: actual_content_id,
            });
        }

        Ok(())
    }

    /// Return the persisted content path for one content id.
    fn content_path(&self, content_id: ArtifactContentId) -> PathBuf {
        let content_hex = content_id.to_hex();
        let shard = &content_hex[0..2];

        self.layout
            .content_root()
            .join(shard)
            .join(format!("{content_hex}.bin"))
    }

    /// Return the current content id path for one image key.
    fn current_content_id_path(
        &self,
        artifact_image_key: &ArtifactImageKey,
    ) -> Result<PathBuf, ArtifactImageError> {
        let image_key_bytes =
            postcard::to_allocvec(artifact_image_key).map_err(ArtifactImageError::Serialize)?;
        let image_key_token = ArtifactContentId::from_bytes(&image_key_bytes).to_hex();
        let shard = &image_key_token[0..2];

        Ok(self
            .layout
            .current_root()
            .join(shard)
            .join(format!("{image_key_token}.bin")))
    }
}

impl From<CacheStoreError> for ArtifactImageError {
    fn from(error: CacheStoreError) -> Self {
        match error {
            CacheStoreError::Io(error) => Self::Io(error),
        }
    }
}
