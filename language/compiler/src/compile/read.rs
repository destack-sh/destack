use std::sync::Arc;

use destack_artifact::ArtifactDependency;
use destack_source::{File, FileId, ModuleId, PackageId, ProfileId, TargetId};
use destack_workspace::{
    DestackConfig, Module, Package, ProviderContext, Revision, Target, TargetDiscoveryError,
};

use crate::Compiler;

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

    /// Load one package Destack config and record its file dependency.
    pub(crate) fn destack_config_for_package(
        &self,
        context: &dyn ProviderContext,
        package_id: PackageId,
    ) -> Option<Arc<DestackConfig>> {
        let revision = context.revision();
        let package = self.package(revision, package_id);
        if let Some(file_id) = package.destack_file_id {
            let content_id = self
                .repository
                .file_content_id(revision, file_id)
                .unwrap_or_else(|error| panic!("failed to load package config content id: {error}"))
                .unwrap_or_else(|| panic!("missing package config content id for {package_id:?}"));

            context.track(ArtifactDependency::file_content(file_id, content_id));
        }

        self.repository
            .destack_config_for_package_id(revision, package_id)
            .unwrap_or_else(|error| panic!("failed to load package config: {error}"))
    }

    /// Return one target or built-in and record its configuration dependency.
    pub(crate) fn target_or_builtin(
        &self,
        context: &dyn ProviderContext,
        target_id: TargetId,
    ) -> Option<Target> {
        let target = self
            .repository
            .target_or_builtin(context.revision(), target_id)
            .unwrap_or_else(|error| panic!("failed to load target: {error}"))?;

        context.track(ArtifactDependency::target_configuration(
            target_id,
            target.configuration_key(),
        ));

        Some(target)
    }

    /// Return module ids selected by one target and record the target module dependency.
    pub(crate) fn target_module_ids(
        &self,
        context: &dyn ProviderContext,
        target_id: &TargetId,
    ) -> Result<Vec<ModuleId>, TargetDiscoveryError> {
        let mut module_ids = self
            .repository
            .target_module_ids(context.revision(), *target_id)?;

        module_ids.sort_unstable();
        module_ids.dedup();
        context.track(ArtifactDependency::target_modules(
            *target_id,
            module_ids.iter().copied(),
        ));

        Ok(module_ids)
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
