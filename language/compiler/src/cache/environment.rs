use crate::compile::Compiler;
use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    IntrinsicEnvironment, LanguageEnvironment, LibraryEnvironment, ProfileKey,
};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Revision};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::CacheHasher;

/// Persistent image context for one profile scoped environment artifact.
#[derive(Debug, Clone)]
pub(crate) struct EnvironmentImageContext {
    /// The profile key for the image.
    profile_key: ProfileKey,
    /// Hash of the effective compiler configuration.
    config_hash: u64,
}

impl EnvironmentImageContext {
    /// Build one image header for a stable environment image key.
    fn header(&self, image_key: ArtifactImageKey) -> ArtifactImageHeader {
        ArtifactImageHeader::new(image_key, self.config_hash)
    }
}

#[allow(dead_code)]
impl Compiler {
    /// Build the current expected language environment image header.
    pub(crate) fn language_environment_image_header(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.language_environment_image_context(revision, profile_id)?;
        let image_key = ArtifactImageKey::LanguageEnvironment {
            profile: context.profile_key.clone(),
        };

        Some(context.header(image_key))
    }

    /// Build the current expected intrinsic environment image header.
    pub(crate) fn intrinsic_environment_image_header(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.intrinsic_environment_image_context(revision, profile_id)?;
        let image_key = ArtifactImageKey::IntrinsicEnvironment {
            profile: context.profile_key.clone(),
        };

        Some(context.header(image_key))
    }

    /// Build the current expected library environment image header.
    pub(crate) fn library_environment_image_header(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.library_environment_image_context(revision, profile_id)?;
        let image_key = ArtifactImageKey::LibraryEnvironment {
            profile: context.profile_key.clone(),
        };

        Some(context.header(image_key))
    }

    /// Load one persisted profile environment image entry when disk mode is enabled.
    fn load_environment_image_entry<T>(
        &self,
        revision: Revision,
        context: &EnvironmentImageContext,
        image_key: ArtifactImageKey,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned + Serialize,
    {
        let expected = context.header(image_key);
        self.load_image::<T>(revision, expected)
    }

    /// Build one persistent image context for a profile scoped environment artifact.
    fn environment_image_context(
        &self,
        profile_id: ProfileId,
        environment_input_hash: u64,
    ) -> EnvironmentImageContext {
        let profile = self.profile(profile_id);

        // image validity for environments is profile, compiler behavior, and source scoped
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&profile.key);
        hasher.hash_value(&environment_input_hash);

        EnvironmentImageContext {
            profile_key: profile.key.clone(),
            config_hash: hasher.finish(),
        }
    }

    /// Hash one builtin module set by exact module ids and current source content.
    fn environment_module_source_hash(
        &self,
        revision: Revision,
        module_ids: &[ModuleId],
    ) -> Option<u64> {
        // normalize the module set before hashing
        let mut module_ids = module_ids.to_vec();
        module_ids.sort_unstable();
        module_ids.dedup();

        // hash the exact source inputs for this environment
        let mut hasher = CacheHasher::new();
        for module_id in module_ids {
            let source_hash = self.module_source_hash(revision, module_id)?;
            hasher.hash_value(&module_id);
            hasher.hash_value(&source_hash);
        }

        Some(hasher.finish())
    }

    /// Build the image context for one language environment.
    fn language_environment_image_context(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<EnvironmentImageContext> {
        let builtins = self.repository.builtins.as_ref();
        let mut module_ids = builtins.language_symbol_module_ids();
        module_ids.sort_unstable();
        module_ids.dedup();

        let environment_input_hash = self.environment_module_source_hash(revision, &module_ids)?;

        Some(self.environment_image_context(profile_id, environment_input_hash))
    }

    /// Build the image context for one intrinsic environment.
    fn intrinsic_environment_image_context(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<EnvironmentImageContext> {
        let builtins = self.repository.builtins.as_ref();
        let module_ids = builtins.intrinsic_module_ids();
        let environment_input_hash = self.environment_module_source_hash(revision, &module_ids)?;

        Some(self.environment_image_context(profile_id, environment_input_hash))
    }

    /// Build the image context for one library environment.
    fn library_environment_image_context(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<EnvironmentImageContext> {
        // the disabled case still has a stable empty library environment
        if !self.options.load_libraries {
            return Some(self.environment_image_context(profile_id, 0));
        }

        let selection = self
            .builtin_library_selection_from_input(revision, profile_id)
            .ok()?;

        // hash the exact ordered library surface and backing module sources
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&selection.ordered_libraries);
        hasher.hash_value(&selection.ambient_modules);

        for module_batch in &selection.modules_to_resolve {
            hasher.hash_value(&module_batch.len());

            for &module_id in module_batch {
                let source_hash = self.module_source_hash(revision, module_id)?;
                hasher.hash_value(&module_id);
                hasher.hash_value(&source_hash);
            }
        }

        Some(self.environment_image_context(profile_id, hasher.finish()))
    }

    /// Load one persisted profile environment image when disk mode is enabled.
    fn load_environment_image<T>(
        &self,
        revision: Revision,
        context: &EnvironmentImageContext,
        image_key: ArtifactImageKey,
    ) -> Result<Option<T>, ArtifactImageError>
    where
        T: DeserializeOwned + Serialize,
    {
        let Some(image) = self.load_environment_image_entry::<T>(revision, context, image_key)?
        else {
            return Ok(None);
        };

        Ok(Some(image.payload))
    }

    /// Persist one profile environment image when disk mode is enabled.
    fn store_environment_image<T>(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        context: &EnvironmentImageContext,
        image_key: ArtifactImageKey,
        payload: T,
    ) -> Result<(), ArtifactImageError>
    where
        T: DeserializeOwned + Serialize,
    {
        let header = context.header(image_key);

        self.store_image(revision, artifact_key, header, payload)
    }

    /// Load one persisted language environment image.
    pub(crate) fn load_language_environment_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Option<LanguageEnvironment>, ArtifactImageError> {
        let Some(context) = self.language_environment_image_context(revision, profile_id) else {
            return Ok(None);
        };
        let image_key = ArtifactImageKey::LanguageEnvironment {
            profile: context.profile_key.clone(),
        };

        self.load_environment_image(revision, &context, image_key)
    }

    /// Persist one language environment image.
    pub(crate) fn store_language_environment_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
        environment: LanguageEnvironment,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.language_environment_image_context(revision, profile_id) else {
            return Ok(());
        };
        let image_key = ArtifactImageKey::LanguageEnvironment {
            profile: context.profile_key.clone(),
        };
        let artifact_key = ArtifactKey::language_environment(profile_id);

        self.store_environment_image(revision, &artifact_key, &context, image_key, environment)
    }

    /// Load one persisted intrinsic environment image.
    pub(crate) fn load_intrinsic_environment_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Option<IntrinsicEnvironment>, ArtifactImageError> {
        let Some(context) = self.intrinsic_environment_image_context(revision, profile_id) else {
            return Ok(None);
        };
        let image_key = ArtifactImageKey::IntrinsicEnvironment {
            profile: context.profile_key.clone(),
        };

        self.load_environment_image(revision, &context, image_key)
    }

    /// Persist one intrinsic environment image.
    pub(crate) fn store_intrinsic_environment_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
        environment: IntrinsicEnvironment,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.intrinsic_environment_image_context(revision, profile_id) else {
            return Ok(());
        };
        let image_key = ArtifactImageKey::IntrinsicEnvironment {
            profile: context.profile_key.clone(),
        };
        let artifact_key = ArtifactKey::intrinsic_environment(profile_id);

        self.store_environment_image(revision, &artifact_key, &context, image_key, environment)
    }

    /// Load one persisted library environment image.
    pub(crate) fn load_library_environment_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Option<LibraryEnvironment>, ArtifactImageError> {
        let Some(context) = self.library_environment_image_context(revision, profile_id) else {
            return Ok(None);
        };
        let image_key = ArtifactImageKey::LibraryEnvironment {
            profile: context.profile_key.clone(),
        };

        self.load_environment_image(revision, &context, image_key)
    }

    /// Persist one library environment image.
    pub(crate) fn store_library_environment_image(
        &self,
        revision: Revision,
        profile_id: ProfileId,
        environment: LibraryEnvironment,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.library_environment_image_context(revision, profile_id) else {
            return Ok(());
        };
        let image_key = ArtifactImageKey::LibraryEnvironment {
            profile: context.profile_key.clone(),
        };
        let artifact_key = ArtifactKey::library_environment(profile_id);

        self.store_environment_image(revision, &artifact_key, &context, image_key, environment)
    }
}
