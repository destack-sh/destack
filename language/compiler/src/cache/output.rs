use crate::compile::Compiler;

use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    ModuleOutput, PackageOutput,
};
use destack_source::{ModuleId, ModuleVersion, PackageId, ProfileVersion};
use destack_workspace::{ProfileId, Target, TargetDiscovery, TargetDiscoveryIssue, TargetId};

use super::{CacheHasher, compiler_version};

/// Persistent image context for one module output artifact.
#[derive(Debug, Clone)]
struct ModuleOutputImageContext {
    /// The profile version used when producing the image.
    profile_version: ProfileVersion,
    /// Hash of the effective compiler configuration and target.
    config_hash: u64,
    /// The module version used when producing the image.
    module_version: ModuleVersion,
}

impl ModuleOutputImageContext {
    /// Build one image header for one module output artifact.
    fn header(&self, module_id: ModuleId, target_id: TargetId) -> ArtifactImageHeader {
        ArtifactImageHeader::new(
            ArtifactImageKey::ModuleOutput {
                module: module_id,
                target: target_id,
            },
            compiler_version(),
            Some(self.profile_version),
            self.config_hash,
            None,
            0,
        )
    }
}

/// Persistent image context for one package output artifact.
#[derive(Debug, Clone)]
struct PackageOutputImageContext {
    /// Hash of the effective target config and discovered module state.
    config_hash: u64,
}

impl PackageOutputImageContext {
    /// Build one image header for one package output artifact.
    fn header(&self, package_id: PackageId, target_id: TargetId) -> ArtifactImageHeader {
        ArtifactImageHeader::new(
            ArtifactImageKey::PackageOutput {
                package: package_id,
                target: target_id,
            },
            compiler_version(),
            None,
            self.config_hash,
            None,
            0,
        )
    }
}

impl Compiler {
    /// Build the current expected module output image header.
    pub(crate) fn module_output_image_header(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<ArtifactImageHeader> {
        let context =
            self.module_output_image_context(module_id, module_version, profile_id, target_id)?;

        Some(context.header(module_id, target_id.clone()))
    }

    /// Build the current expected package output image header.
    pub(crate) fn package_output_image_header(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.package_output_image_context(package_id, target_id)?;

        Some(context.header(package_id, target_id.clone()))
    }

    /// Load one module output image entry.
    fn load_module_output_image_entry(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Result<Option<ArtifactImage<ModuleOutput>>, ArtifactImageError> {
        let Some(context) =
            self.module_output_image_context(module_id, module_version, profile_id, target_id)
        else {
            return Ok(None);
        };
        let expected = context.header(module_id, target_id.clone());
        let Some(image) = self.load_image::<ModuleOutput>(expected)? else {
            return Ok(None);
        };
        if self.module_version(module_id) != context.module_version {
            return Ok(None);
        }

        Ok(Some(image))
    }

    /// Load one package output image entry.
    fn load_package_output_image_entry(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> Result<Option<ArtifactImage<PackageOutput>>, ArtifactImageError> {
        let Some(context) = self.package_output_image_context(package_id, target_id) else {
            return Ok(None);
        };
        let expected = context.header(package_id, target_id.clone());
        self.load_image::<PackageOutput>(expected)
    }

    /// Resolve the current target configuration for one module target.
    fn target_config_for_module_output(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Option<Target> {
        let module = self.program.modules.get(module_id);
        let package = self.program.packages.get(module.package_id);
        let package = package.read();

        package.targets.get(target_id).cloned()
    }

    /// Resolve the current target configuration for one package target.
    fn target_config_for_package_output(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> Option<Target> {
        let package = self.program.packages.get(package_id);
        let package = package.read();

        package.targets.get(target_id).cloned()
    }

    /// Build one persistent image context for one module output artifact.
    fn module_output_image_context(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Option<ModuleOutputImageContext> {
        let profile = self.program.profile(profile_id);
        let target = self.target_config_for_module_output(module_id, target_id)?;

        // module output images are scoped by compiler behavior, profile identity, and target config
        let mut hasher = CacheHasher::new();
        hasher.hash_compiler_options(&self.options);
        hasher.hash_value(&profile.key);
        hasher.hash_value(&target);

        Some(ModuleOutputImageContext {
            profile_version: profile.version,
            config_hash: hasher.finish(),
            module_version,
        })
    }

    /// Discover the current module set for one package target.
    fn package_target_modules(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        target: &Target,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        let package = self.program.packages.get(package_id);
        let package = package.read();
        let package_path = package.path.clone();

        match target.discovery {
            TargetDiscovery::Entry => {
                self.discover_entry_modules(package_id, &package_path, target, target_id)
            }
            TargetDiscovery::Include => {
                self.discover_include_modules(package_id, &package_path, target)
            }
        }
    }

    /// Build one persistent image context for one package output artifact.
    fn package_output_image_context(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> Option<PackageOutputImageContext> {
        let target = self.target_config_for_package_output(package_id, target_id)?;
        let mut modules = self
            .package_target_modules(package_id, target_id, &target)
            .ok()?;
        modules.sort_unstable();

        // package output images are scoped by target config and discovered module state
        let mut hasher = CacheHasher::new();
        hasher.hash_value(&target);
        for module_id in modules {
            hasher.hash_value(&module_id);
            hasher.hash_value(&self.module_version(module_id));

            if let Some(profile_id) = self.program.profile_id_for_target(module_id, target_id) {
                let profile = self.program.profile(profile_id);
                hasher.hash_value(&profile.key);
                hasher.hash_value(&profile.version);
            }
        }

        Some(PackageOutputImageContext {
            config_hash: hasher.finish(),
        })
    }

    /// Load one module output image.
    pub(crate) fn load_module_output_image(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        target_id: &TargetId,
    ) -> Result<Option<ModuleOutput>, ArtifactImageError> {
        Ok(self
            .load_module_output_image_entry(module_id, module_version, profile_id, target_id)?
            .map(|image| image.payload))
    }

    /// Persist one module output image.
    pub(crate) fn store_module_output_image(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: &TargetId,
        output: &ModuleOutput,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.module_output_image_context(
            module_id,
            self.module_version(module_id),
            profile_id,
            target_id,
        ) else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::module_output(module_id, target_id.clone());
        let header = context.header(module_id, target_id.clone());

        self.store_image(&artifact_key, header, output.clone())
    }

    /// Load one package output image.
    pub(crate) fn load_package_output_image(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> Result<Option<PackageOutput>, ArtifactImageError> {
        Ok(self
            .load_package_output_image_entry(package_id, target_id)?
            .map(|image| image.payload))
    }

    /// Persist one package output image.
    pub(crate) fn store_package_output_image(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        output: &PackageOutput,
    ) -> Result<(), ArtifactImageError> {
        let Some(context) = self.package_output_image_context(package_id, target_id) else {
            return Ok(());
        };
        let artifact_key = ArtifactKey::package_output(package_id, target_id.clone());
        let header = context.header(package_id, target_id.clone());

        self.store_image(&artifact_key, header, output.clone())
    }
}
