use crate::ArtifactTaskKeyExt;
use crate::compile::Compiler;
use destack_artifact::{
    ArtifactCache, ArtifactContentId, ArtifactDependency, ArtifactImage, ArtifactImageDependency,
    ArtifactImageError, ArtifactImageHeader, ArtifactKey, ArtifactStore,
};
use destack_workspace::CacheMode;
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The persisted language cache abi.
pub(crate) const LANGUAGE_CACHE_ABI: &str = "language-cache-v1";

impl Compiler {
    /// Build persisted image dependencies for the current task attempt.
    fn current_image_dependencies(
        &self,
        artifact_key: &ArtifactKey,
    ) -> Result<Vec<ArtifactImageDependency>, ArtifactImageError> {
        if artifact_key.family().persisted_image_validation()
            == destack_artifact::PersistedImageValidation::SelfContained
        {
            return Ok(Vec::new());
        }

        let mut image_dependencies = Vec::new();
        for requirement in self.current_requirements() {
            let image_key = requirement
                .key
                .image_key_with(|profile_id| self.profile(profile_id).key.clone());
            let Some(content_id) = self.load_expected_artifact_content_id(&image_key)? else {
                let error = std::io::Error::other(format!(
                    "missing persisted dependency image for '{}'",
                    requirement.key.name()
                ));
                return Err(ArtifactImageError::Io(error));
            };

            image_dependencies.push(ArtifactImageDependency {
                key: image_key,
                content_id,
            });
        }

        Ok(image_dependencies)
    }

    /// Return whether one persisted image dependency list is currently satisfied.
    pub(crate) fn persisted_image_dependencies_are_satisfied(
        &self,
        dependencies: &[ArtifactImageDependency],
    ) -> Result<bool, ArtifactImageError> {
        for dependency in dependencies {
            let Some(content_id) = self.load_expected_artifact_content_id(&dependency.key)? else {
                return Ok(false);
            };

            if content_id != dependency.content_id {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Resolve the effective workspace cache mode.
    pub(crate) fn workspace_cache_mode(&self) -> CacheMode {
        let Some(revision) = self.current_execution_revision() else {
            return CacheMode::Off;
        };

        self.repository
            .workspace_options(revision)
            .ok()
            .flatten()
            .map(|options| options.cache.mode)
            .unwrap_or(CacheMode::Off)
    }

    /// Resolve the persisted artifact cache when disk mode is enabled.
    pub(crate) fn artifact_cache(&self) -> Option<ArtifactCache<'_>> {
        if self.workspace_cache_mode() != CacheMode::Disk {
            return None;
        }

        Some(self.repository.artifact_cache(LANGUAGE_CACHE_ABI))
    }

    /// Load one persisted artifact image when its header still matches.
    pub(crate) fn load_image<T>(
        &self,
        expected: ArtifactImageHeader,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned,
    {
        let Some(artifact_cache) = self.artifact_cache() else {
            return Ok(None);
        };
        let Some(image) = artifact_cache.load::<T>(&expected.artifact_image_key)? else {
            return Ok(None);
        };

        if !expected.matches(&image.header) {
            return Ok(None);
        }

        if !self.persisted_image_dependencies_are_satisfied(&image.header.dependencies)? {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Load one persisted image header when its header still matches.
    pub(crate) fn load_image_header(
        &self,
        expected: &ArtifactImageHeader,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        let Some(artifact_cache) = self.artifact_cache() else {
            return Ok(None);
        };
        let Some(header) = artifact_cache.load_header(&expected.artifact_image_key)? else {
            return Ok(None);
        };

        if !expected.matches(&header) {
            return Ok(None);
        }

        if !self.persisted_image_dependencies_are_satisfied(&header.dependencies)? {
            return Ok(None);
        }

        Ok(Some(header))
    }

    /// Load one current persisted content id when its header still matches.
    pub(crate) fn load_current_content_id(
        &self,
        expected: &ArtifactImageHeader,
    ) -> Result<Option<ArtifactContentId>, ArtifactImageError> {
        let Some(artifact_cache) = self.artifact_cache() else {
            return Ok(None);
        };
        let Some(header) = self.load_image_header(expected)? else {
            return Ok(None);
        };

        artifact_cache.load_current_content_id(&header.artifact_image_key)
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

    /// Restore one cached artifact image through the compiler publication path.
    pub(crate) fn restore_cached_artifact<T, F, P>(
        &self,
        artifact_key: ArtifactKey,
        load: F,
        publish: P,
    ) -> Option<T>
    where
        T: Clone,
        F: FnOnce(&Self) -> Result<Option<T>, ArtifactImageError>,
        P: FnOnce(&ArtifactStore, destack_artifact::ArtifactVersion, T),
    {
        let payload = self.load_artifact(&artifact_key, load)?;
        self.publish_artifact(artifact_key, payload.clone(), publish);

        Some(payload)
    }

    /// Store one persisted artifact image when disk mode is enabled.
    pub(crate) fn store_image<T>(
        &self,
        artifact_key: &ArtifactKey,
        header: ArtifactImageHeader,
        payload: T,
    ) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        let Some(artifact_cache) = self.artifact_cache() else {
            return Ok(());
        };
        let dependencies = self.current_image_dependencies(artifact_key)?;
        let image = ArtifactImage::new(header.with_dependencies(dependencies), payload)?;

        artifact_cache.save(&image).map(|_| ())
    }

    /// Store one live artifact image in the artifact store.
    pub(crate) fn store_artifact<T, F>(&self, artifact_key: &ArtifactKey, payload: &T, store: F)
    where
        F: FnOnce(&Self, ArtifactDependency, &T) -> Result<(), ArtifactImageError>,
    {
        // skip artifact image writes when persistent cache is disabled
        if self.workspace_cache_mode() != CacheMode::Disk {
            return;
        }

        let artifact_dependency = self.artifact_dependency_for_key(artifact_key);

        if let Err(error) = store(self, artifact_dependency, payload) {
            let artifact = artifact_key.name();
            tracing::warn!(?error, %artifact, "compile.cache.store_failed");
        }
    }
}
