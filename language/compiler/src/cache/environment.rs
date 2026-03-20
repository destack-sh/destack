use crate::compile::Compiler;
use destack_source::ProfileVersion;
use destack_workspace::{
    ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, IntrinsicEnvironment,
    LanguageEnvironment, LibraryEnvironment, ProfileId, ProfileKey,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{CacheHasher, compiler_version};

/// Persistent image context for one profile scoped environment artifact.
#[derive(Debug, Clone)]
pub(crate) struct EnvironmentImageContext {
    /// The stable profile key for the image.
    profile_key: ProfileKey,
    /// The profile version used when producing the image.
    profile_version: ProfileVersion,
    /// Hash of the effective compiler configuration.
    config_hash: u64,
}

impl EnvironmentImageContext {
    /// Build one image header for a stable environment image key.
    fn header(&self, image_key: ArtifactImageKey) -> ArtifactImageHeader {
        ArtifactImageHeader::new(
            image_key,
            compiler_version(),
            Some(self.profile_version),
            self.config_hash,
            None,
            0,
        )
    }
}

impl Compiler {
    /// Build one persistent image context for a profile scoped environment artifact.
    fn environment_image_context(&self, profile_id: ProfileId) -> EnvironmentImageContext {
        let profile = self.program.profile(profile_id);

        // image validity for environments is profile and compiler configuration scoped
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&profile.key);

        EnvironmentImageContext {
            profile_key: profile.key.clone(),
            profile_version: profile.version,
            config_hash: hasher.finish(),
        }
    }

    /// Load one persisted profile environment image when disk mode is enabled.
    fn load_environment_image<T>(
        &self,
        context: &EnvironmentImageContext,
        image_key: ArtifactImageKey,
    ) -> Result<Option<T>, ArtifactImageError>
    where
        T: DeserializeOwned + Serialize,
    {
        let expected = context.header(image_key);
        let Some(image) = self.load_image::<T>(expected)? else {
            return Ok(None);
        };

        Ok(Some(image.payload))
    }

    /// Persist one profile environment image when disk mode is enabled.
    fn store_environment_image<T>(
        &self,
        context: &EnvironmentImageContext,
        image_key: ArtifactImageKey,
        payload: T,
    ) -> Result<(), ArtifactImageError>
    where
        T: DeserializeOwned + Serialize,
    {
        let header = context.header(image_key);

        self.store_image(header, payload)
    }

    /// Load one persisted language environment image.
    pub(crate) fn load_language_environment_image(
        &self,
        profile_id: ProfileId,
    ) -> Result<Option<LanguageEnvironment>, ArtifactImageError> {
        let context = self.environment_image_context(profile_id);
        let image_key = ArtifactImageKey::LanguageEnvironment {
            profile: context.profile_key.clone(),
        };

        self.load_environment_image(&context, image_key)
    }

    /// Persist one language environment image.
    pub(crate) fn store_language_environment_image(
        &self,
        profile_id: ProfileId,
        environment: LanguageEnvironment,
    ) -> Result<(), ArtifactImageError> {
        let context = self.environment_image_context(profile_id);
        let image_key = ArtifactImageKey::LanguageEnvironment {
            profile: context.profile_key.clone(),
        };

        self.store_environment_image(&context, image_key, environment)
    }

    /// Load one persisted intrinsic environment image.
    pub(crate) fn load_intrinsic_environment_image(
        &self,
        profile_id: ProfileId,
    ) -> Result<Option<IntrinsicEnvironment>, ArtifactImageError> {
        let context = self.environment_image_context(profile_id);
        let image_key = ArtifactImageKey::IntrinsicEnvironment {
            profile: context.profile_key.clone(),
        };

        self.load_environment_image(&context, image_key)
    }

    /// Persist one intrinsic environment image.
    pub(crate) fn store_intrinsic_environment_image(
        &self,
        profile_id: ProfileId,
        environment: IntrinsicEnvironment,
    ) -> Result<(), ArtifactImageError> {
        let context = self.environment_image_context(profile_id);
        let image_key = ArtifactImageKey::IntrinsicEnvironment {
            profile: context.profile_key.clone(),
        };

        self.store_environment_image(&context, image_key, environment)
    }

    /// Load one persisted library environment image.
    pub(crate) fn load_library_environment_image(
        &self,
        profile_id: ProfileId,
    ) -> Result<Option<LibraryEnvironment>, ArtifactImageError> {
        let context = self.environment_image_context(profile_id);
        let image_key = ArtifactImageKey::LibraryEnvironment {
            profile: context.profile_key.clone(),
        };

        self.load_environment_image(&context, image_key)
    }

    /// Persist one library environment image.
    pub(crate) fn store_library_environment_image(
        &self,
        profile_id: ProfileId,
        environment: LibraryEnvironment,
    ) -> Result<(), ArtifactImageError> {
        let context = self.environment_image_context(profile_id);
        let image_key = ArtifactImageKey::LibraryEnvironment {
            profile: context.profile_key.clone(),
        };

        self.store_environment_image(&context, image_key, environment)
    }
}
