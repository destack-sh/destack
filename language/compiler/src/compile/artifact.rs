use std::sync::Arc;

#[cfg(feature = "native-codegen")]
use destack_artifact::MirOptimized;
use destack_artifact::{
    AmbientEnvironment, ArtifactKey, ArtifactStore, ArtifactVersion, Ast, Data, DirChecked,
    DirDeclared, DirExported, LanguageEnvironment, MirLowered, ModuleOutput,
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

    /// Require one language environment artifact and return its payload.
    pub(crate) fn language_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<Arc<LanguageEnvironment>, ProviderError> {
        let artifact_key = ArtifactKey::language_environment(profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.language_environment(version)
        })
    }

    /// Require one ambient environment artifact and return its payload.
    pub(crate) fn ambient_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<Arc<AmbientEnvironment>, ProviderError> {
        let artifact_key = ArtifactKey::ambient_environment(profile);
        let version = context.require(artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.ambient_environment(version)
        })
    }

    /// Require one language environment artifact without loading its payload.
    pub(crate) fn require_language_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::language_environment(profile))?;

        Ok(())
    }

    /// Require one ambient environment artifact without loading its payload.
    pub(crate) fn require_ambient_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<(), ProviderError> {
        context.require(ArtifactKey::ambient_environment(profile))?;

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
