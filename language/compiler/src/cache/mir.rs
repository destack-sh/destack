use crate::compile::Compiler;

use super::{CacheHasher, compiler_version};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey, MirBase,
    MirOptimized, ProfileId, ProfileKey, Target, TargetId,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Persistent image context for one target scoped MIR artifact.
#[derive(Debug, Clone)]
struct MirImageContext {
    /// The stable profile key for the image.
    profile_key: ProfileKey,
    /// The profile version used when producing the image.
    profile_version: ProfileVersion,
    /// The module version used when producing the image.
    module_version: ModuleVersion,
    /// Hash of the effective compiler configuration and target.
    config_hash: u64,
}

impl MirImageContext {
    /// Build one image header for the chosen MIR family.
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
    /// Build the current expected base MIR image header.
    pub(crate) fn mir_base_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.mir_image_context(module_id, module_version, profile_id, target_id)?;

        Some(context.header(ArtifactImageKey::MirBase {
            module: module_id,
            profile: context.profile_key.clone(),
            target: target_id.clone(),
        }))
    }

    /// Build the current expected optimized MIR image header.
    pub(crate) fn mir_optimized_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.mir_image_context(module_id, module_version, profile_id, target_id)?;

        Some(context.header(ArtifactImageKey::MirOptimized {
            module: module_id,
            profile: context.profile_key.clone(),
            target: target_id.clone(),
        }))
    }

    /// Load one MIR image entry.
    fn load_mir_image_entry<T>(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
        image_key: ArtifactImageKey,
        version: impl FnOnce(&T) -> ModuleVersion,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned + Serialize,
    {
        let Some(context) =
            self.mir_image_context(module_id, module_version, profile_id, target_id)
        else {
            return Ok(None);
        };
        let expected = context.header(image_key);
        let Some(image) = self.load_image::<T>(expected)? else {
            return Ok(None);
        };

        if version(&image.payload) != context.module_version {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Resolve the current target configuration for one module target.
    fn target_config_for_module_image(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Option<Target> {
        let module = self.program.modules.get(module_id);
        let package = self.program.packages.get(module.package_id);
        let package = package.read();

        package.targets.get(target_id).cloned()
    }

    /// Build one persistent image context for one MIR artifact.
    fn mir_image_context(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<MirImageContext> {
        let profile = self.program.profile(profile_id);
        let target = self.target_config_for_module_image(module_id, target_id)?;

        // mir images are scoped by compiler behavior, profile identity, and target config
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&profile.key);
        hasher.hash_value(&target);

        Some(MirImageContext {
            profile_key: profile.key.clone(),
            profile_version: profile.version,
            module_version,
            config_hash: hasher.finish(),
        })
    }

    /// Load one base MIR image.
    pub(crate) fn load_mir_base_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Result<Option<MirBase>, ArtifactImageError> {
        let Some(profile_key) = self
            .mir_image_context(module_id, module_version, profile_id, target_id)
            .map(|context| context.profile_key)
        else {
            return Ok(None);
        };

        Ok(self
            .load_mir_image_entry(
                module_id,
                module_version,
                profile_id,
                target_id,
                ArtifactImageKey::MirBase {
                    module: module_id,
                    profile: profile_key,
                    target: target_id.clone(),
                },
                |mir: &MirBase| mir.version,
            )?
            .map(|image| image.payload))
    }

    /// Persist one base MIR image.
    pub(crate) fn store_mir_base_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: &TargetId,
        mir: &MirBase,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.mir_image_context(module_id, mir.version, profile_id, target_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::mir_base(module_id, profile_id, target_id.clone());
        let header = context.header(ArtifactImageKey::MirBase {
            module: module_id,
            profile: context.profile_key.clone(),
            target: target_id.clone(),
        });

        self.store_image(&artifact_key, header, mir.clone())
    }

    /// Load one optimized MIR image.
    pub(crate) fn load_mir_optimized_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Result<Option<MirOptimized>, ArtifactImageError> {
        let Some(profile_key) = self
            .mir_image_context(module_id, module_version, profile_id, target_id)
            .map(|context| context.profile_key)
        else {
            return Ok(None);
        };

        Ok(self
            .load_mir_image_entry(
                module_id,
                module_version,
                profile_id,
                target_id,
                ArtifactImageKey::MirOptimized {
                    module: module_id,
                    profile: profile_key,
                    target: target_id.clone(),
                },
                |mir: &MirOptimized| mir.version,
            )?
            .map(|image| image.payload))
    }

    /// Persist one optimized MIR image.
    pub(crate) fn store_mir_optimized_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: &TargetId,
        mir: &MirOptimized,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.mir_image_context(module_id, mir.version, profile_id, target_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::mir_optimized(module_id, profile_id, target_id.clone());
        let header = context.header(ArtifactImageKey::MirOptimized {
            module: module_id,
            profile: context.profile_key.clone(),
            target: target_id.clone(),
        });

        self.store_image(&artifact_key, header, mir.clone())
    }
}
