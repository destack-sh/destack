use crate::ArtifactTaskKeyExt;
use crate::compile::{Compiler, CompilerContext};
use destack_artifact::{
    ArtifactCache, ArtifactContentId, ArtifactImage, ArtifactImageDependency, ArtifactImageError,
    ArtifactImageHeader, ArtifactKey, ArtifactStamp, ArtifactStore, ArtifactVersion,
    PersistedImageValidation,
};
use destack_workspace::{CacheMode, Revision};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// The persisted language cache abi.
pub(crate) const LANGUAGE_CACHE_ABI: &str = "language-cache-v1";

impl Compiler {
    /// Load one stored image header when the cache entry still matches.
    fn load_matching_image_header(
        &self,
        revision: Revision,
        expected: &ArtifactImageHeader,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        if expected
            .artifact_image_key
            .family()
            .persisted_image_validation()
            .is_none()
        {
            return Ok(None);
        }

        let Some(artifact_cache) = self.artifact_cache(revision) else {
            return Ok(None);
        };
        let Some(header) = artifact_cache.load_header(&expected.artifact_image_key)? else {
            return Ok(None);
        };

        if !expected.matches(&header) {
            return Ok(None);
        }

        Ok(Some(header))
    }

    /// Build persisted image dependencies for the current task attempt.
    fn current_image_dependencies(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> Result<Option<Vec<ArtifactImageDependency>>, ArtifactImageError> {
        let Some(validation) = artifact_key.family().persisted_image_validation() else {
            return Ok(None);
        };

        if validation == PersistedImageValidation::SelfContained {
            return Ok(Some(Vec::new()));
        }

        let mut image_dependencies = Vec::new();
        for requirement in self.current_requirements() {
            let image_key = requirement
                .key
                .image_key_with(|profile_id| self.profile(profile_id).key.clone());
            let Some(content_id) = self.load_expected_artifact_content_id(revision, &image_key)?
            else {
                return Ok(None);
            };

            image_dependencies.push(ArtifactImageDependency {
                key: image_key,
                content_id,
            });
        }

        Ok(Some(image_dependencies))
    }

    /// Return whether one persisted image dependency list is currently satisfied.
    pub(crate) fn persisted_image_dependencies_are_satisfied(
        &self,
        revision: Revision,
        dependencies: &[ArtifactImageDependency],
    ) -> Result<bool, ArtifactImageError> {
        self.persisted_image_dependencies_are_satisfied_with_active(
            revision,
            dependencies,
            &mut Vec::new(),
        )
    }

    /// Return whether one persisted image dependency list is currently satisfied.
    fn persisted_image_dependencies_are_satisfied_with_active(
        &self,
        revision: Revision,
        dependencies: &[ArtifactImageDependency],
        active_keys: &mut Vec<destack_artifact::ArtifactImageKey>,
    ) -> Result<bool, ArtifactImageError> {
        for dependency in dependencies {
            let Some(content_id) = self.load_expected_artifact_content_id_with_active(
                revision,
                &dependency.key,
                active_keys,
            )?
            else {
                return Ok(false);
            };

            if content_id != dependency.content_id {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Resolve the effective workspace cache mode.
    pub(crate) fn workspace_cache_mode(&self, revision: Revision) -> CacheMode {
        self.repository
            .workspace_options(revision)
            .ok()
            .flatten()
            .map(|options| options.cache.mode)
            .unwrap_or(CacheMode::Off)
    }

    /// Resolve the persisted artifact cache when disk mode is enabled.
    pub(crate) fn artifact_cache(&self, revision: Revision) -> Option<ArtifactCache<'_>> {
        if self.workspace_cache_mode(revision) != CacheMode::Disk {
            return None;
        }

        Some(self.repository.artifact_cache(LANGUAGE_CACHE_ABI))
    }

    /// Load one persisted artifact image when its header still matches.
    pub(crate) fn load_image<T>(
        &self,
        revision: Revision,
        expected: ArtifactImageHeader,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned,
    {
        if expected
            .artifact_image_key
            .family()
            .persisted_image_validation()
            .is_none()
        {
            return Ok(None);
        }

        let Some(artifact_cache) = self.artifact_cache(revision) else {
            return Ok(None);
        };
        let Some(image) = artifact_cache.load::<T>(&expected.artifact_image_key)? else {
            return Ok(None);
        };

        if !expected.matches(&image.header) {
            return Ok(None);
        }

        if !self.persisted_image_dependencies_are_satisfied(revision, &image.header.dependencies)? {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Load one persisted image header when its header still matches.
    fn load_image_header_with_active(
        &self,
        revision: Revision,
        expected: &ArtifactImageHeader,
        active_keys: &mut Vec<destack_artifact::ArtifactImageKey>,
    ) -> Result<Option<ArtifactImageHeader>, ArtifactImageError> {
        if expected
            .artifact_image_key
            .family()
            .persisted_image_validation()
            .is_none()
        {
            return Ok(None);
        }

        let Some(header) = self.load_matching_image_header(revision, expected)? else {
            return Ok(None);
        };

        if !self.persisted_image_dependencies_are_satisfied_with_active(
            revision,
            &header.dependencies,
            active_keys,
        )? {
            return Ok(None);
        }

        Ok(Some(header))
    }

    /// Load one current persisted content id when its header still matches.
    pub(crate) fn load_current_content_id_with_active(
        &self,
        revision: Revision,
        expected: &ArtifactImageHeader,
        active_keys: &mut Vec<destack_artifact::ArtifactImageKey>,
    ) -> Result<Option<ArtifactContentId>, ArtifactImageError> {
        if expected
            .artifact_image_key
            .family()
            .persisted_image_validation()
            .is_none()
        {
            return Ok(None);
        }

        let Some(artifact_cache) = self.artifact_cache(revision) else {
            return Ok(None);
        };

        // break recursive validation across cyclic persisted dependency graphs
        if active_keys.contains(&expected.artifact_image_key) {
            let Some(header) = self.load_matching_image_header(revision, expected)? else {
                return Ok(None);
            };

            return artifact_cache.load_current_content_id(&header.artifact_image_key);
        }

        active_keys.push(expected.artifact_image_key.clone());
        let header = self.load_image_header_with_active(revision, expected, active_keys);
        active_keys.pop();

        let Some(header) = header? else {
            return Ok(None);
        };

        artifact_cache.load_current_content_id(&header.artifact_image_key)
    }

    /// Load one cached artifact image.
    pub(crate) fn load_artifact<T, F>(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        load: F,
    ) -> Option<T>
    where
        F: FnOnce(&Self) -> Result<Option<T>, ArtifactImageError>,
    {
        // skip artifact image loading when persistent cache is disabled
        if self.workspace_cache_mode(revision) != CacheMode::Disk {
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
        revision: Revision,
        artifact_key: ArtifactKey,
        load: F,
        publish: P,
    ) -> Option<T>
    where
        T: Clone,
        F: FnOnce(&Self) -> Result<Option<T>, ArtifactImageError>,
        P: FnOnce(&ArtifactStore, ArtifactVersion, T),
    {
        let payload = self.load_artifact(revision, &artifact_key, load)?;
        self.publish_artifact(artifact_key, payload.clone(), publish);

        Some(payload)
    }

    /// Store one persisted artifact image when disk mode is enabled.
    pub(crate) fn store_image<T>(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        header: ArtifactImageHeader,
        payload: T,
    ) -> Result<(), ArtifactImageError>
    where
        T: Serialize,
    {
        if artifact_key.family().persisted_image_validation().is_none() {
            return Ok(());
        }

        let Some(artifact_cache) = self.artifact_cache(revision) else {
            return Ok(());
        };
        let Some(dependencies) = self.current_image_dependencies(revision, artifact_key)? else {
            return Ok(());
        };
        let image = ArtifactImage::new(header.with_dependencies(dependencies), payload)?;

        artifact_cache.save(&image).map(|_| ())
    }
}

impl CompilerContext<'_> {
    /// Restore one cached artifact image through the compiler publication path.
    pub(crate) fn restore_cached_artifact<T, F, P>(
        &self,
        artifact_key: ArtifactKey,
        load: F,
        publish: P,
    ) -> Option<T>
    where
        T: Clone,
        F: FnOnce(&Compiler) -> Result<Option<T>, ArtifactImageError>,
        P: FnOnce(&ArtifactStore, ArtifactVersion, T),
    {
        let payload = self
            .compiler()
            .load_artifact(self.revision(), &artifact_key, load)?;
        self.publish_artifact(artifact_key, payload.clone(), publish);

        Some(payload)
    }

    /// Store one persisted artifact image when disk mode is enabled.
    pub(crate) fn store_artifact<T, F>(&self, artifact_key: &ArtifactKey, payload: &T, store: F)
    where
        F: FnOnce(&Compiler, ArtifactStamp, &T) -> Result<(), ArtifactImageError>,
    {
        // skip artifact image writes when persistent cache is disabled
        if self.compiler().workspace_cache_mode(self.revision()) != CacheMode::Disk {
            return;
        }

        let artifact_stamp = self.artifact_stamp(artifact_key);

        if let Err(error) = store(self.compiler(), artifact_stamp, payload) {
            let artifact = artifact_key.name();
            tracing::warn!(?error, %artifact, "compile.cache.store_failed");
        }
    }
}
