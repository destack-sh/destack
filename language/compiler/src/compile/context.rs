use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_source::{File, FileId, ModuleId, PackageId, ProfileId, TargetId};
use destack_workspace::{
    CompilerOptions, Module, Package, PackageOptions, Profile, RepositorySnapshot, Revision,
    TsCompilerOptions, TsConfigDeclaration, TsConfigOptions,
};

use super::{Compiler, ModuleCheckOptions};

/// One pinned compiler execution view for one retained repository revision.
#[derive(Debug, Clone)]
pub struct CompilerContext<'a> {
    /// The shared compiler engine.
    compiler: &'a Compiler,
    /// The pinned repository snapshot.
    snapshot: RepositorySnapshot,
    /// The running artifact key, when this context belongs to one provide attempt.
    artifact_key: Option<ArtifactKey>,
}

impl<'a> CompilerContext<'a> {
    /// Build one compiler context from explicit parts.
    pub(crate) fn new(
        compiler: &'a Compiler,
        snapshot: RepositorySnapshot,
        artifact_key: Option<ArtifactKey>,
    ) -> Self {
        Self {
            compiler,
            snapshot,
            artifact_key,
        }
    }

    /// Return the shared compiler engine.
    pub fn compiler(&self) -> &'a Compiler {
        self.compiler
    }

    /// Return the pinned repository snapshot.
    pub fn snapshot(&self) -> &RepositorySnapshot {
        &self.snapshot
    }

    /// Return the retained revision identity.
    pub fn revision(&self) -> Revision {
        self.snapshot.revision()
    }

    /// Return the running artifact key when this context belongs to one provide attempt.
    pub fn artifact_key(&self) -> Option<ArtifactKey> {
        self.artifact_key
    }

    /// Return one module from this pinned revision.
    pub fn module(&self, module_id: ModuleId) -> Arc<Module> {
        self.snapshot
            .module(module_id)
            .unwrap_or_else(|error| panic!("failed to load module: {error}"))
            .unwrap_or_else(|| panic!("missing module for {module_id:?}"))
    }

    /// Return one file from this pinned revision.
    pub fn file(&self, file_id: FileId) -> Arc<File> {
        self.snapshot
            .file(file_id)
            .unwrap_or_else(|error| panic!("failed to load file: {error}"))
            .unwrap_or_else(|| panic!("missing file for {file_id:?}"))
    }

    /// Return one package from this pinned revision.
    pub fn package(&self, package_id: PackageId) -> Arc<Package> {
        self.snapshot
            .package(package_id)
            .unwrap_or_else(|error| panic!("failed to load package: {error}"))
            .unwrap_or_else(|| panic!("missing package for {package_id:?}"))
    }

    /// Return one tsconfig declaration from this pinned revision.
    pub fn tsconfig(&self, tsconfig_file_id: FileId) -> Arc<TsConfigDeclaration> {
        self.compiler
            .repository
            .tsconfig_declaration(self.revision(), tsconfig_file_id)
            .unwrap_or_else(|error| panic!("failed to load tsconfig: {error}"))
            .unwrap_or_else(|| panic!("missing tsconfig for {tsconfig_file_id:?}"))
    }

    /// Return whether one module is a code module in this pinned revision.
    pub fn is_code_module(&self, module_id: ModuleId) -> bool {
        self.module(module_id).is_code()
    }

    /// Load package config options for one package in this pinned revision.
    pub fn package_options(&self, package_id: PackageId) -> Option<PackageOptions> {
        self.compiler
            .repository
            .package_options(self.revision(), package_id)
            .unwrap_or_else(|error| panic!("failed to load package options: {error}"))
    }

    /// Return all workspace package ids in this pinned revision.
    pub fn workspace_package_ids(&self) -> Vec<PackageId> {
        self.compiler
            .repository
            .workspace_package_ids(self.revision())
            .unwrap_or_else(|error| panic!("failed to load workspace package ids: {error}"))
    }

    /// Load compiler options for one module in this pinned revision.
    pub fn compiler_options_for_module(&self, module: &Module) -> Option<CompilerOptions> {
        self.package_options(module.package_id)
            .map(|options| options.compiler)
    }

    /// Load tsconfig options for one module in this pinned revision.
    pub fn tsconfig_options_for_module(&self, module: &Module) -> Option<TsConfigOptions> {
        self.compiler
            .repository
            .tsconfig_options_for_module(self.revision(), module)
            .ok()
            .flatten()
    }

    /// Load TypeScript compiler options for one module in this pinned revision.
    pub fn ts_compiler_options_for_module(&self, module: &Module) -> Option<TsCompilerOptions> {
        self.tsconfig_options_for_module(module)
            .map(|options| options.compiler)
    }

    /// Return module compatibility options for one module.
    pub(crate) fn module_check_options_for_module(
        &self,
        module_id: ModuleId,
    ) -> ModuleCheckOptions {
        let module = self.module(module_id);
        let module = module.as_ref();
        let mut options = self
            .compiler_options_for_module(module)
            .map(|options| ModuleCheckOptions::from_workspace(&options))
            .unwrap_or_default();

        if let Some(ts_options) = self.ts_compiler_options_for_module(module) {
            options.apply_typescript(&ts_options);
        }

        options
    }

    /// Resolve the default profile for one module in this pinned revision.
    pub fn default_profile_for_module(&self, module_id: ModuleId) -> Profile {
        self.compiler
            .repository
            .default_profile_for_module(self.revision(), module_id)
            .unwrap_or_else(|error| panic!("failed to resolve default profile: {error}"))
    }

    /// Resolve the default profile id for one module in this pinned revision.
    pub fn default_profile_id_for_module(&self, module_id: ModuleId) -> ProfileId {
        self.default_profile_for_module(module_id).id()
    }

    /// Resolve the target profile for one module in this pinned revision.
    pub fn profile_for_target(&self, module_id: ModuleId, target_id: &TargetId) -> Option<Profile> {
        self.compiler
            .repository
            .profile_for_target(self.revision(), module_id, target_id)
            .unwrap_or_else(|error| panic!("failed to resolve target profile: {error}"))
    }

    /// Resolve the target profile id for one module in this pinned revision.
    pub fn profile_id_for_target(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> Option<ProfileId> {
        self.profile_for_target(module_id, target_id)
            .map(|profile| profile.id())
    }

    /// Resolve the target profile id for one module in this pinned revision, or fall back to the default profile.
    pub fn profile_id_for_target_or_default(
        &self,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> ProfileId {
        self.profile_id_for_target(module_id, target_id)
            .unwrap_or_else(|| self.default_profile_id_for_module(module_id))
    }
}
