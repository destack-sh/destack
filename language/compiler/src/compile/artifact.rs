use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactStore, ArtifactVersion, Ast, DirAnalyzed, DirBase, DirDeclared,
    DirElaborated, DirInterface, DirPatched, DirPrepared, DirResolved, IntrinsicEnvironment,
    LanguageEnvironment, LibraryEnvironment, MirBase, MirOptimized, ModuleGraph, ModuleOutput,
    PackageOutput,
};
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};

use crate::{Compiler, CompilerContext, RequirementError};

impl Compiler {
    /// Return one current live artifact payload and retain its exact version for this scope.
    fn current_artifact<T>(
        &self,
        artifact_key: ArtifactKey,
        load: impl FnOnce(&ArtifactStore, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Option<Arc<T>> {
        let version = self.artifact_version_for_key(&artifact_key);
        let payload = load(&self.artifacts, &version)?;

        // do not retain the artifact currently being built
        if self.current_artifact_key() != Some(artifact_key) {
            self.retain_current_artifact_version(&version);
        }

        Some(payload)
    }

    /// Return the current module graph for one profile.
    pub(crate) fn module_graph(&self, profile: ProfileId) -> Option<Arc<ModuleGraph>> {
        self.current_artifact(ArtifactKey::module_graph(profile), |artifacts, version| {
            artifacts.module_graph(version)
        })
    }

    /// Return the current language environment for one profile.
    pub(crate) fn language_environment(
        &self,
        profile: ProfileId,
    ) -> Option<Arc<LanguageEnvironment>> {
        self.current_artifact(
            ArtifactKey::language_environment(profile),
            |artifacts, version| artifacts.language_environment(version),
        )
    }

    /// Return the current intrinsic environment for one profile.
    pub(crate) fn intrinsic_environment(
        &self,
        profile: ProfileId,
    ) -> Option<Arc<IntrinsicEnvironment>> {
        self.current_artifact(
            ArtifactKey::intrinsic_environment(profile),
            |artifacts, version| artifacts.intrinsic_environment(version),
        )
    }

    /// Return the current library environment for one profile.
    pub(crate) fn library_environment(
        &self,
        profile: ProfileId,
    ) -> Option<Arc<LibraryEnvironment>> {
        self.current_artifact(
            ArtifactKey::library_environment(profile),
            |artifacts, version| artifacts.library_environment(version),
        )
    }

    /// Return the current AST for one module.
    pub(crate) fn ast(&self, module: ModuleId) -> Option<Arc<Ast>> {
        self.current_artifact(ArtifactKey::ast(module), |artifacts, version| {
            artifacts.ast(version)
        })
    }

    /// Return the current base DIR for one module.
    pub(crate) fn dir_base(&self, module: ModuleId) -> Option<Arc<DirBase>> {
        self.current_artifact(ArtifactKey::dir_base(module), |artifacts, version| {
            artifacts.dir_base(version)
        })
    }

    /// Return the current prepared DIR for one module profile.
    pub(crate) fn dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirPrepared>> {
        self.current_artifact(
            ArtifactKey::dir_prepared(module, profile),
            |artifacts, version| artifacts.dir_prepared(version),
        )
    }

    /// Return the current resolved DIR for one module profile.
    pub(crate) fn dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirResolved>> {
        self.current_artifact(
            ArtifactKey::dir_resolved(module, profile),
            |artifacts, version| artifacts.dir_resolved(version),
        )
    }

    /// Return the current declared DIR for one module profile.
    pub(crate) fn dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirDeclared>> {
        self.current_artifact(
            ArtifactKey::dir_declared(module, profile),
            |artifacts, version| artifacts.dir_declared(version),
        )
    }

    /// Return the current interface DIR for one module profile.
    pub(crate) fn dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirInterface>> {
        self.current_artifact(
            ArtifactKey::dir_interface(module, profile),
            |artifacts, version| artifacts.dir_interface(version),
        )
    }

    /// Return the current analyzed DIR for one module profile.
    pub(crate) fn dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirAnalyzed>> {
        self.current_artifact(
            ArtifactKey::dir_analyzed(module, profile),
            |artifacts, version| artifacts.dir_analyzed(version),
        )
    }

    /// Return the current elaborated DIR for one module profile.
    pub(crate) fn dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirElaborated>> {
        self.current_artifact(
            ArtifactKey::dir_elaborated(module, profile),
            |artifacts, version| artifacts.dir_elaborated(version),
        )
    }

    /// Return the current patched DIR for one module profile.
    pub(crate) fn dir_patched(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirPatched>> {
        self.current_artifact(
            ArtifactKey::dir_patched(module, profile),
            |artifacts, version| artifacts.dir_patched(version),
        )
    }

    /// Return the current base MIR for one module profile target.
    pub(crate) fn mir_base(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<MirBase>> {
        self.current_artifact(
            ArtifactKey::mir_base(module, profile, *target),
            |artifacts, version| artifacts.mir_base(version),
        )
    }

    /// Return the current optimized MIR for one module profile target.
    pub(crate) fn mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<MirOptimized>> {
        self.current_artifact(
            ArtifactKey::mir_optimized(module, profile, *target),
            |artifacts, version| artifacts.mir_optimized(version),
        )
    }

    /// Return the current generated module artifact for one target.
    pub(crate) fn module_output(
        &self,
        module: ModuleId,
        target: &TargetId,
    ) -> Option<Arc<ModuleOutput>> {
        self.current_artifact(
            ArtifactKey::module_output(module, *target),
            |artifacts, version| artifacts.module_output(version),
        )
    }

    /// Return the current package output for one target.
    pub(crate) fn package_output(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> Option<Arc<PackageOutput>> {
        self.current_artifact(
            ArtifactKey::package_output(package, *target),
            |artifacts, version| artifacts.package_output(version),
        )
    }
}

#[allow(dead_code)]
impl CompilerContext<'_> {
    /// Require one prepared DIR artifact and return the current payload.
    pub(crate) fn require_artifact_dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirPrepared>, RequirementError> {
        self.require_artifact(ArtifactKey::dir_prepared(module, profile))?;

        Ok(self
            .dir_prepared(module, profile)
            .unwrap_or_else(|| panic!("missing prepared DIR artifact after require")))
    }

    /// Require one resolved DIR artifact and return the current payload.
    pub(crate) fn require_artifact_dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirResolved>, RequirementError> {
        self.require_artifact(ArtifactKey::dir_resolved(module, profile))?;

        Ok(self
            .dir_resolved(module, profile)
            .unwrap_or_else(|| panic!("missing resolved DIR artifact after require")))
    }

    /// Require one declared DIR artifact and return the current payload.
    pub(crate) fn require_artifact_dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, RequirementError> {
        self.require_artifact(ArtifactKey::dir_declared(module, profile))?;

        Ok(self
            .dir_declared(module, profile)
            .unwrap_or_else(|| panic!("missing declared DIR artifact after require")))
    }

    /// Require one interface DIR artifact and return the current payload.
    pub(crate) fn require_artifact_dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirInterface>, RequirementError> {
        self.require_artifact(ArtifactKey::dir_interface(module, profile))?;

        Ok(self
            .dir_interface(module, profile)
            .unwrap_or_else(|| panic!("missing interface DIR artifact after require")))
    }

    /// Require one analyzed DIR artifact and return the current payload.
    pub(crate) fn require_artifact_dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirAnalyzed>, RequirementError> {
        self.require_artifact(ArtifactKey::dir_analyzed(module, profile))?;

        Ok(self
            .dir_analyzed(module, profile)
            .unwrap_or_else(|| panic!("missing analyzed DIR artifact after require")))
    }

    /// Return one current live artifact payload and retain its exact version for this scope.
    fn current_artifact<T>(
        &self,
        artifact_key: ArtifactKey,
        load: impl FnOnce(&ArtifactStore, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Option<Arc<T>> {
        let version = self.artifact_version(&artifact_key);
        let payload = load(&self.compiler().artifacts, &version)?;

        // do not retain the artifact currently being built
        if self.compiler().current_artifact_key() != Some(artifact_key) {
            self.compiler().retain_current_artifact_version(&version);
        }

        Some(payload)
    }

    /// Return the current module graph for one profile.
    pub(crate) fn module_graph(&self, profile: ProfileId) -> Option<Arc<ModuleGraph>> {
        self.current_artifact(ArtifactKey::module_graph(profile), |artifacts, version| {
            artifacts.module_graph(version)
        })
    }

    /// Return the current language environment for one profile.
    pub(crate) fn language_environment(
        &self,
        profile: ProfileId,
    ) -> Option<Arc<LanguageEnvironment>> {
        self.current_artifact(
            ArtifactKey::language_environment(profile),
            |artifacts, version| artifacts.language_environment(version),
        )
    }

    /// Return the current intrinsic environment for one profile.
    pub(crate) fn intrinsic_environment(
        &self,
        profile: ProfileId,
    ) -> Option<Arc<IntrinsicEnvironment>> {
        self.current_artifact(
            ArtifactKey::intrinsic_environment(profile),
            |artifacts, version| artifacts.intrinsic_environment(version),
        )
    }

    /// Return the current library environment for one profile.
    pub(crate) fn library_environment(
        &self,
        profile: ProfileId,
    ) -> Option<Arc<LibraryEnvironment>> {
        self.current_artifact(
            ArtifactKey::library_environment(profile),
            |artifacts, version| artifacts.library_environment(version),
        )
    }

    /// Return the current AST for one module.
    pub(crate) fn ast(&self, module: ModuleId) -> Option<Arc<Ast>> {
        self.current_artifact(ArtifactKey::ast(module), |artifacts, version| {
            artifacts.ast(version)
        })
    }

    /// Return the current base DIR for one module.
    pub(crate) fn dir_base(&self, module: ModuleId) -> Option<Arc<DirBase>> {
        self.current_artifact(ArtifactKey::dir_base(module), |artifacts, version| {
            artifacts.dir_base(version)
        })
    }

    /// Return the current prepared DIR for one module profile.
    pub(crate) fn dir_prepared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirPrepared>> {
        self.current_artifact(
            ArtifactKey::dir_prepared(module, profile),
            |artifacts, version| artifacts.dir_prepared(version),
        )
    }

    /// Return the current resolved DIR for one module profile.
    pub(crate) fn dir_resolved(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirResolved>> {
        self.current_artifact(
            ArtifactKey::dir_resolved(module, profile),
            |artifacts, version| artifacts.dir_resolved(version),
        )
    }

    /// Return the current declared DIR for one module profile.
    pub(crate) fn dir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirDeclared>> {
        self.current_artifact(
            ArtifactKey::dir_declared(module, profile),
            |artifacts, version| artifacts.dir_declared(version),
        )
    }

    /// Return the current interface DIR for one module profile.
    pub(crate) fn dir_interface(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirInterface>> {
        self.current_artifact(
            ArtifactKey::dir_interface(module, profile),
            |artifacts, version| artifacts.dir_interface(version),
        )
    }

    /// Return the current analyzed DIR for one module profile.
    pub(crate) fn dir_analyzed(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirAnalyzed>> {
        self.current_artifact(
            ArtifactKey::dir_analyzed(module, profile),
            |artifacts, version| artifacts.dir_analyzed(version),
        )
    }

    /// Return the current elaborated DIR for one module profile.
    pub(crate) fn dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirElaborated>> {
        self.current_artifact(
            ArtifactKey::dir_elaborated(module, profile),
            |artifacts, version| artifacts.dir_elaborated(version),
        )
    }

    /// Return the current patched DIR for one module profile.
    pub(crate) fn dir_patched(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirPatched>> {
        self.current_artifact(
            ArtifactKey::dir_patched(module, profile),
            |artifacts, version| artifacts.dir_patched(version),
        )
    }

    /// Return the current base MIR for one module profile target.
    pub(crate) fn mir_base(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<MirBase>> {
        self.current_artifact(
            ArtifactKey::mir_base(module, profile, *target),
            |artifacts, version| artifacts.mir_base(version),
        )
    }

    /// Return the current optimized MIR for one module profile target.
    pub(crate) fn mir_optimized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Option<Arc<MirOptimized>> {
        self.current_artifact(
            ArtifactKey::mir_optimized(module, profile, *target),
            |artifacts, version| artifacts.mir_optimized(version),
        )
    }

    /// Return the current generated module artifact for one target.
    pub(crate) fn module_output(
        &self,
        module: ModuleId,
        target: &TargetId,
    ) -> Option<Arc<ModuleOutput>> {
        self.current_artifact(
            ArtifactKey::module_output(module, *target),
            |artifacts, version| artifacts.module_output(version),
        )
    }

    /// Return the current package output for one target.
    pub(crate) fn package_output(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> Option<Arc<PackageOutput>> {
        self.current_artifact(
            ArtifactKey::package_output(package, *target),
            |artifacts, version| artifacts.package_output(version),
        )
    }
}
