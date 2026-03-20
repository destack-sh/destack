use crate::ArtifactTaskKeyExt;
use crate::compile::Compiler;
use destack_workspace::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactKey, ArtifactPayload,
    ArtifactStore, CacheMode, CacheValidate,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

impl Compiler {
    /// Resolve the effective workspace cache mode.
    pub(crate) fn workspace_cache_mode(&self) -> CacheMode {
        self.session
            .workspace_config()
            .as_ref()
            .map(|config| config.options.cache.mode)
            .unwrap_or(CacheMode::Off)
    }

    /// Resolve the effective workspace cache validation mode.
    pub(crate) fn workspace_cache_validate(&self) -> CacheValidate {
        self.session
            .workspace_config()
            .as_ref()
            .map(|config| config.options.cache.validate)
            .unwrap_or(CacheValidate::Strict)
    }

    /// Resolve the persisted artifact store when disk mode is enabled.
    pub(crate) fn artifact_store(&self) -> Option<ArtifactStore<'_>> {
        if self.workspace_cache_mode() != CacheMode::Disk {
            return None;
        }

        let cache_root = self.session.workspace_cache_dir();
        Some(ArtifactStore::new(
            self.session.cache_store.as_ref(),
            &cache_root,
        ))
    }

    /// Load one persisted artifact image when its header still matches.
    pub(crate) fn load_image<T>(
        &self,
        expected: ArtifactImageHeader,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned,
    {
        let Some(artifact_store) = self.artifact_store() else {
            return Ok(None);
        };
        let Some(image) = artifact_store.load::<T>(&expected.artifact_image_key)? else {
            return Ok(None);
        };

        if !expected.matches(&image.header) {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Load one cached artifact image.
    pub(crate) fn load_artifact<T, F>(&self, artifact_key: &ArtifactKey, load: F) -> Option<T>
    where
        F: FnOnce(&Self) -> Result<Option<T>, ArtifactImageError>,
    {
        // skip artifact image loading when persistent cache is disabled
        if self.workspace_cache_mode() != CacheMode::Disk {
            return None;
        }

        match load(self) {
            Ok(Some(payload)) => Some(payload),

            // no cached artifact
            Ok(None) => None,

            // cache load failure
            Err(error) => {
                let artifact = artifact_key.name();
                tracing::warn!(?error, %artifact, "compile.cache.load_failed");
                None
            }
        }
    }

    /// Load one cached artifact image and publish it into the live registry.
    pub(crate) fn load_published_artifact<T, F>(
        &self,
        artifact_key: ArtifactKey,
        load: F,
    ) -> Option<T>
    where
        T: Clone + Into<ArtifactPayload>,
        F: FnOnce(&Self) -> Result<Option<T>, ArtifactImageError>,
    {
        let payload = self.load_artifact(&artifact_key, load)?;
        self.program
            .artifacts
            .publish(artifact_key, payload.clone());

        Some(payload)
    }

    /// Store one persisted artifact image when disk mode is enabled.
    pub(crate) fn store_image<T>(
        &self,
        header: ArtifactImageHeader,
        payload: T,
    ) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        let Some(artifact_store) = self.artifact_store() else {
            return Ok(());
        };
        let image = ArtifactImage::new(header, payload)?;

        artifact_store.save(&image)
    }

    /// Store one live artifact image in the artifact store.
    pub(crate) fn store_artifact<T, F>(&self, artifact_key: &ArtifactKey, payload: &T, store: F)
    where
        F: FnOnce(&Self, &T) -> Result<(), ArtifactImageError>,
    {
        // skip artifact image writes when persistent cache is disabled
        if self.workspace_cache_mode() != CacheMode::Disk {
            return;
        }

        if let Err(error) = store(self, payload) {
            let artifact = artifact_key.name();
            tracing::warn!(?error, %artifact, "compile.cache.store_failed");
        }
    }
}
