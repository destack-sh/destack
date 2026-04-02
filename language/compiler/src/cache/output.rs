use crate::compile::Compiler;

use destack_artifact::{
    ArtifactImage, ArtifactImageError, ArtifactImageHeader, ArtifactImageKey, ArtifactKey,
    PackageOutput,
};
use destack_source::{ModuleId, PackageId, TargetId};
use destack_workspace::{Target, TargetDiscovery, TargetDiscoveryIssue};

use super::{CacheHasher, repository_package};

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
            self.config_hash,
        )
    }
}

impl Compiler {
    /// Build the current expected package output image header.
    pub(crate) fn package_output_image_header(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> Option<ArtifactImageHeader> {
        let context = self.package_output_image_context(package_id, target_id)?;

        Some(context.header(package_id, *target_id))
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
        let expected = context.header(package_id, *target_id);
        self.load_image::<PackageOutput>(expected)
    }

    /// Resolve the current target configuration for one package target.
    fn target_config_for_package_output(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
    ) -> Option<Target> {
        let package = repository_package(self, package_id).ok()?;

        package.targets.get(target_id).cloned()
    }

    /// Discover the current module set for one package target.
    fn package_target_modules(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        target: &Target,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryIssue> {
        let package = repository_package(self, package_id).map_err(|_| {
            TargetDiscoveryIssue::MissingPackagePath {
                package: package_id,
                target: *target_id,
            }
        })?;
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
            hasher.hash_value(&self.artifact_dependency_for_key(&ArtifactKey::dir_patched(
                module_id,
                self.profile_id_for_target(module_id, target_id)?,
            )));

            if let Some(profile_id) = self.profile_id_for_target(module_id, target_id) {
                let profile = self.profile(profile_id);
                hasher.hash_value(&profile.key);
            }
        }

        Some(PackageOutputImageContext {
            config_hash: hasher.finish(),
        })
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
        let artifact_key = ArtifactKey::package_output(package_id, *target_id);
        let header = context.header(package_id, *target_id);

        self.store_image(&artifact_key, header, output.clone())
    }
}
