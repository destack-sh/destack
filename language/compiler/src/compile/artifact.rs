use std::sync::Arc;

#[cfg(feature = "native-codegen")]
use destack_artifact::MirOptimized;
use destack_artifact::{
    ArtifactKey, ArtifactStore, ArtifactVersion, Ast, Data, DirChecked, DirDeclared, DirElaborated,
    DirExpanded, DirExported, DirImported, GlobalEnvironment, MirLowered, ModuleOutput,
};
use destack_source::{ModuleId, ProfileId, TargetId};
use destack_workspace::{ProviderContext, ProviderError};

use crate::Compiler;

impl Compiler {
    /// Return one required payload or report artifact store corruption.
    pub(crate) fn required_artifact<T>(
        &self,
        version: &ArtifactVersion,
        load: impl FnOnce(&ArtifactStore, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        load(&self.artifacts, version).ok_or(ProviderError::Corrupt { version: *version })
    }

    /// Require one declared DIR artifact and return its payload.
    pub(crate) fn require_dir_declared(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, ProviderError> {
        let artifact_key = ArtifactKey::dir_declared(module, profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_declared(version)
        })
    }

    /// Require one imported DIR artifact and return its payload.
    pub(crate) fn require_dir_imported(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirImported>, ProviderError> {
        let artifact_key = ArtifactKey::dir_imported(module, profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_imported(version)
        })
    }

    /// Require one expanded DIR artifact and return its payload.
    pub(crate) fn require_dir_expanded(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExpanded>, ProviderError> {
        let artifact_key = ArtifactKey::dir_expanded(module, profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_expanded(version)
        })
    }

    /// Require one exported DIR artifact and return its payload.
    pub(crate) fn require_dir_exported(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExported>, ProviderError> {
        let artifact_key = ArtifactKey::dir_exported(module, profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_exported(version)
        })
    }

    /// Require one checked DIR artifact without loading its payload.
    pub(crate) fn require_dir_checked(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::dir_checked(module, profile))?;

        Ok(())
    }

    /// Require one global environment artifact and return its payload.
    pub(crate) fn global_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<Arc<GlobalEnvironment>, ProviderError> {
        let artifact_key = ArtifactKey::global_environment(profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.global_environment(version)
        })
    }

    /// Require one global environment artifact without loading its payload.
    pub(crate) fn require_global_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::global_environment(profile))?;

        Ok(())
    }

    /// Return the AST payload for one exact module output requirement.
    pub(crate) fn ast(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> Result<Arc<Ast>, ProviderError> {
        let artifact_key = ArtifactKey::ast(module);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| artifacts.ast(version))
    }

    /// Return the parsed data payload for one exact module output requirement.
    pub(crate) fn data(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> Result<Arc<Data>, ProviderError> {
        let artifact_key = ArtifactKey::data(module);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| artifacts.data(version))
    }

    /// Return the declared DIR payload for one exact module output requirement.
    pub(crate) fn dir_declared(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirDeclared>, ProviderError> {
        self.require_dir_declared(context, module, profile)
    }

    /// Return the checked DIR payload for one exact module output requirement.
    pub(crate) fn dir_checked(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirChecked>, ProviderError> {
        let artifact_key = ArtifactKey::dir_checked(module, profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_checked(version)
        })
    }

    /// Return the elaborated DIR payload for one exact module output requirement.
    pub(crate) fn dir_elaborated(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirElaborated>, ProviderError> {
        let artifact_key = ArtifactKey::dir_elaborated(module, profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_elaborated(version)
        })
    }

    /// Require one lowered MIR artifact without loading its payload.
    pub(crate) fn require_mir_lowered(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::mir_lowered(module, profile, *target))?;

        Ok(())
    }

    /// Return the lowered MIR payload for one exact module output requirement.
    pub(crate) fn mir_lowered(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirLowered>, ProviderError> {
        let artifact_key = ArtifactKey::mir_lowered(module, profile, *target);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.mir_lowered(version)
        })
    }

    /// Require one verified MIR marker without loading its payload.
    pub(crate) fn require_mir_verified(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::mir_verified(module, profile, *target))?;

        Ok(())
    }

    /// Return the optimized MIR payload for one exact module output requirement.
    #[cfg(feature = "native-codegen")]
    pub(crate) fn mir_optimized(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirOptimized>, ProviderError> {
        let artifact_key = ArtifactKey::mir_optimized(module, profile, *target);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.mir_optimized(version)
        })
    }

    /// Require one optimized MIR artifact without loading its payload.
    #[cfg(feature = "native-codegen")]
    pub(crate) fn require_mir_optimized(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::mir_optimized(module, profile, *target))?;

        Ok(())
    }

    /// Return the generated module output for one exact module output requirement.
    pub(crate) fn module_output(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        target: &TargetId,
    ) -> Result<Arc<ModuleOutput>, ProviderError> {
        let artifact_key = ArtifactKey::module_output(module, *target);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.module_output(version)
        })
    }
}
