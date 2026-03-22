use crate::ArtifactTaskKeyExt;
use crate::compile::Compiler;
use destack_workspace::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageRequirement, ArtifactKey,
    ArtifactPayload, ArtifactStore, CacheMode, CacheValidate,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

impl Compiler {
    /// Build persisted requirement proofs for the current task attempt.
    fn current_image_requirements(
        &self,
        artifact_key: &ArtifactKey,
    ) -> Result<Vec<ArtifactImageRequirement>, ArtifactImageError> {
        if artifact_key.family().persisted_image_validation()
            == destack_workspace::PersistedImageValidation::SelfContained
        {
            return Ok(Vec::new());
        }

        let mut image_requirements = Vec::new();
        for requirement in self.current_requirements() {
            let image_key = requirement
                .key
                .image_key_with(|profile_id| self.program.profile(profile_id).key.clone());
            let Some(validation_hash) =
                self.load_expected_artifact_image_validation_hash(&image_key)?
            else {
                let error = std::io::Error::other(format!(
                    "missing persisted dependency proof for '{}'",
                    requirement.key.name()
                ));
                return Err(ArtifactImageError::Io(error));
            };

            image_requirements.push(ArtifactImageRequirement {
                key: image_key,
                validation_hash,
            });
        }

        Ok(image_requirements)
    }

    /// Return whether one persisted requirement proof list is currently satisfied.
    pub(crate) fn persisted_image_requirements_are_satisfied(
        &self,
        requirements: &[ArtifactImageRequirement],
    ) -> Result<bool, ArtifactImageError> {
        for requirement in requirements {
            let Some(validation_hash) =
                self.load_expected_artifact_image_validation_hash(&requirement.key)?
            else {
                return Ok(false);
            };

            if validation_hash != requirement.validation_hash {
                return Ok(false);
            }
        }

        Ok(true)
    }

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

        if !self.persisted_image_requirements_are_satisfied(&image.header.requirements)? {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Load one persisted image header when its header still matches.
    pub(crate) fn load_image_header(
        &self,
        expected: &ArtifactImageHeader,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        let Some(artifact_store) = self.artifact_store() else {
            return Ok(None);
        };
        let Some(header) = artifact_store.load_header(&expected.artifact_image_key)? else {
            return Ok(None);
        };

        if !expected.matches(&header) {
            return Ok(None);
        }

        if !self.persisted_image_requirements_are_satisfied(&header.requirements)? {
            return Ok(None);
        }

        Ok(Some(header))
    }

    /// Load one persisted image validation hash when its header still matches.
    pub(crate) fn load_image_validation_hash(
        &self,
        expected: &ArtifactImageHeader,
    ) -> Result<Option<u64>, ArtifactImageError> {
        Ok(self
            .load_image_header(expected)?
            .map(|header| header.validation_hash))
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
        artifact_key: &ArtifactKey,
        header: ArtifactImageHeader,
        payload: T,
    ) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        let Some(artifact_store) = self.artifact_store() else {
            return Ok(());
        };
        let requirements = self.current_image_requirements(artifact_key)?;
        let image = ArtifactImage::new(header.with_requirements(requirements), payload)?;

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
