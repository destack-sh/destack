use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{ArtifactDependency, TargetKey};
use destack_core::StableHasher;
use destack_source::{File, FileId, ModuleId, PackageId, ProfileId, TargetId};
use destack_workspace::{
    CompilerOptions, EntryResolutionMode, Module, Package, PackageOptions, ProviderContext,
    Revision, Target, TargetDiscoveryError, TargetDiscoveryOptions,
};

use crate::{Compiler, LowerOptions};

const TARGET_CONFIGURATION_DOMAIN: &[u8] = b"destack.compiler.target-configuration.v1";

impl Compiler {
    /// Return one module from one repository revision.
    pub(crate) fn module(&self, revision: Revision, module_id: ModuleId) -> Arc<Module> {
        self.repository
            .module(revision, module_id)
            .unwrap_or_else(|error| panic!("failed to load module: {error}"))
            .unwrap_or_else(|| panic!("missing module for {module_id:?}"))
    }

    /// Return one file from one provider attempt revision and record its content dependency.
    pub(crate) fn file(&self, context: &dyn ProviderContext, file_id: FileId) -> Arc<File> {
        let revision = context.revision();
        let file = self
            .repository
            .file(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to load file: {error}"))
            .unwrap_or_else(|| panic!("missing file for {file_id:?}"));
        let content_id = self
            .repository
            .file_content_id(revision, file_id)
            .unwrap_or_else(|error| panic!("failed to load file content id: {error}"))
            .unwrap_or_else(|| panic!("missing file content id for {file_id:?}"));

        context.track(ArtifactDependency::file_content(file_id, content_id));

        file
    }

    /// Return one package from one repository revision.
    pub(crate) fn package(&self, revision: Revision, package_id: PackageId) -> Arc<Package> {
        self.repository
            .package(revision, package_id)
            .unwrap_or_else(|error| panic!("failed to load package: {error}"))
            .unwrap_or_else(|| panic!("missing package for {package_id:?}"))
    }

    /// Return whether one module is a code module in one revision.
    pub(crate) fn is_code_module(&self, revision: Revision, module_id: ModuleId) -> bool {
        self.module(revision, module_id).is_code()
    }

    /// Load package config options for one package and record its config dependency.
    pub(crate) fn package_options(
        &self,
        context: &dyn ProviderContext,
        package_id: PackageId,
    ) -> Option<PackageOptions> {
        let revision = context.revision();
        let package = self.package(revision, package_id);
        if let Some(file_id) = package.destack_file_id {
            let content_id = self
                .repository
                .file_content_id(revision, file_id)
                .unwrap_or_else(|error| {
                    panic!("failed to load package options content id: {error}")
                })
                .unwrap_or_else(|| panic!("missing package options content id for {package_id:?}"));

            context.track(ArtifactDependency::file_content(file_id, content_id));
        }

        self.repository
            .package_options(revision, package_id)
            .unwrap_or_else(|error| panic!("failed to load package options: {error}"))
    }

    /// Return one effective target and record its configuration dependency.
    pub(crate) fn effective_target(
        &self,
        context: &dyn ProviderContext,
        target_id: TargetId,
    ) -> Option<Target> {
        let target = self
            .repository
            .effective_target(context.revision(), target_id)
            .unwrap_or_else(|error| panic!("failed to load target: {error}"))?;

        let target_key = target_key(&target);
        context.track(ArtifactDependency::target_configuration(
            target_id, target_key,
        ));

        Some(target)
    }

    /// Return module ids selected by one target and record the target module dependency.
    pub(crate) fn target_module_ids(
        &self,
        context: &dyn ProviderContext,
        _package_id: PackageId,
        _package_path: &Option<PathBuf>,
        _target: &Target,
        target_id: &TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        let options = TargetDiscoveryOptions {
            entry_resolution: EntryResolutionMode::Strict,
        };
        let mut module_ids =
            self.repository
                .target_module_ids(context.revision(), *target_id, &options)?;

        module_ids.sort_unstable();
        module_ids.dedup();
        context.track(ArtifactDependency::target_modules(
            *target_id,
            module_ids.iter().copied(),
        ));

        Ok(module_ids)
    }

    /// Load workspace compiler configuration for one module and record its package config dependency.
    pub(crate) fn workspace_compiler_options(
        &self,
        context: &dyn ProviderContext,
        module: &Module,
    ) -> Option<CompilerOptions> {
        self.package_options(context, module.package_id)
            .map(|options| options.compiler)
    }

    /// Resolve lower options for one module.
    pub(crate) fn lower_options(
        &self,
        _context: &dyn ProviderContext,
        _module: &Module,
    ) -> LowerOptions {
        LowerOptions::default()
    }

    /// Resolve the default profile id for one module in one revision.
    pub(crate) fn default_profile_id(&self, revision: Revision, module_id: ModuleId) -> ProfileId {
        self.repository
            .module_profile(revision, module_id)
            .unwrap_or_else(|error| panic!("failed to resolve default profile: {error}"))
            .id()
    }

    /// Resolve the target profile id for one module in one revision.
    pub(crate) fn target_profile_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Option<ProfileId> {
        self.repository
            .module_target_profile(revision, module_id, *target_id)
            .unwrap_or_else(|error| panic!("failed to resolve target profile: {error}"))
            .map(|profile| profile.id())
    }

    /// Resolve the target profile id for one module, or fall back to the default profile.
    pub(crate) fn target_profile_id_or_default(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> ProfileId {
        self.target_profile_id(revision, module_id, target_id)
            .unwrap_or_else(|| self.default_profile_id(revision, module_id))
    }
}

/// Build one effective target key for artifact dependency identity.
fn target_key(target: &Target) -> TargetKey {
    let mut hasher = StableHasher::new();
    hasher.update_len_prefixed(TARGET_CONFIGURATION_DOMAIN);
    target.hash(&mut hasher);

    TargetKey::new(hasher.finish_u128())
}
