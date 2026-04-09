use crate::compile::Compiler;

use super::CacheHasher;
use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    ArtifactStamp, MirBase, MirOptimized, ProfileKey,
};
use destack_source::{ModuleId, TargetId};
use destack_workspace::{ProfileId, Revision, Target};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Persistent image context for one target scoped MIR artifact.
#[derive(Debug, Clone)]
struct MirImageContext {
    /// The profile key for the image.
    profile_key: ProfileKey,
    /// Hash of the effective compiler configuration and target.
    config_hash: u64,
}

impl MirImageContext {
    /// Build one image header for the chosen MIR family.
    fn header(&self, image_key: ArtifactImageKey) -> ArtifactImageHeader {
        ArtifactImageHeader::new(image_key, self.config_hash)
    }
}

impl Compiler {
    /// Build the current expected base MIR image header.
    pub(crate) fn mir_base_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.mir_image_context(revision, module_id, profile_id, target_id)?;

        Some(context.header(ArtifactImageKey::MirBase {
            module: module_id,
            profile: context.profile_key.clone(),
            target: *target_id,
        }))
    }

    /// Build the current expected optimized MIR image header.
    pub(crate) fn mir_optimized_image_header(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.mir_image_context(revision, module_id, profile_id, target_id)?;

        Some(context.header(ArtifactImageKey::MirOptimized {
            module: module_id,
            profile: context.profile_key.clone(),
            target: *target_id,
        }))
    }

    /// Load one MIR image entry.
    fn load_mir_image_entry<T>(
        &self,
        revision: Revision,
        module_id: ModuleId,
        _artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
        target_id: &TargetId,
        image_key: ArtifactImageKey,
    ) -> Result<Option<ArtifactImage<T>>, ArtifactImageError>
    where
        T: DeserializeOwned + Serialize,
    {
        let Some(context) = self.mir_image_context(revision, module_id, profile_id, target_id)
        else {
            return Ok(None);
        };
        let expected = context.header(image_key);
        let Some(image) = self.load_image::<T>(revision, expected)? else {
            return Ok(None);
        };

        Ok(Some(image))
    }

    /// Resolve the current target configuration for one module target.
    fn target_config_for_module_image(
        &self,
        revision: Revision,
        _module_id: ModuleId,
        target_id: &TargetId,
    ) -> Option<Target> {
        self.repository
            .effective_target(revision, *target_id)
            .ok()
            .flatten()
    }

    /// Build one persistent image context for one MIR artifact.
    fn mir_image_context(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<MirImageContext> {
        let profile = self.profile(profile_id);
        let target = self.target_config_for_module_image(revision, module_id, target_id)?;

        // mir images are scoped by compiler behavior, profile identity, and target config
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&profile.key);
        hasher.hash_value(&target);

        Some(MirImageContext {
            profile_key: profile.key.clone(),
            config_hash: hasher.finish(),
        })
    }

    /// Load one base MIR image.
    pub(crate) fn load_mir_base_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Result<Option<MirBase>, ArtifactImageError> {
        let Some(profile_key) = self
            .mir_image_context(revision, module_id, profile_id, target_id)
            .map(|context| context.profile_key)
        else {
            return Ok(None);
        };

        Ok(self
            .load_mir_image_entry(
                revision,
                module_id,
                artifact_stamp,
                profile_id,
                target_id,
                ArtifactImageKey::MirBase {
                    module: module_id,
                    profile: profile_key,
                    target: *target_id,
                },
            )?
            .map(|image| image.payload))
    }

    /// Persist one base MIR image.
    pub(crate) fn store_mir_base_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: &TargetId,
        _artifact_stamp: ArtifactStamp,
        mir: &MirBase,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.mir_image_context(revision, module_id, profile_id, target_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::mir_base(module_id, profile_id, *target_id);
        let header = context.header(ArtifactImageKey::MirBase {
            module: module_id,
            profile: context.profile_key.clone(),
            target: *target_id,
        });

        self.store_image(revision, &artifact_key, header, mir.clone())
    }

    /// Load one optimized MIR image.
    pub(crate) fn load_mir_optimized_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        artifact_stamp: ArtifactStamp,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Result<Option<MirOptimized>, ArtifactImageError> {
        let Some(profile_key) = self
            .mir_image_context(revision, module_id, profile_id, target_id)
            .map(|context| context.profile_key)
        else {
            return Ok(None);
        };

        Ok(self
            .load_mir_image_entry(
                revision,
                module_id,
                artifact_stamp,
                profile_id,
                target_id,
                ArtifactImageKey::MirOptimized {
                    module: module_id,
                    profile: profile_key,
                    target: *target_id,
                },
            )?
            .map(|image| image.payload))
    }

    /// Persist one optimized MIR image.
    pub(crate) fn store_mir_optimized_image(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: &TargetId,
        _artifact_stamp: ArtifactStamp,
        mir: &MirOptimized,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.mir_image_context(revision, module_id, profile_id, target_id)
        else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::mir_optimized(module_id, profile_id, *target_id);
        let header = context.header(ArtifactImageKey::MirOptimized {
            module: module_id,
            profile: context.profile_key.clone(),
            target: *target_id,
        });

        self.store_image(revision, &artifact_key, header, mir.clone())
    }
}
