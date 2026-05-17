use std::sync::Arc;

#[cfg(feature = "native")]
use destack_artifact::MirOptimized;
use destack_artifact::{
    ArtifactKey, ArtifactStore, ArtifactVersion, Data, DirBound, DirChecked, DirElaborated,
    DirExpanded, DirExported, DirImported, DirMaterialized, DirParsed, GlobalEnvironment,
    MirLowered, MirVerified, ModuleOutput, PackageOutput,
};
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};
use destack_workspace::{ProviderContext, ProviderError};

use crate::Compiler;

impl Compiler {
    /// Return one required payload or report artifact store corruption.
    pub(crate) fn require_artifact<T>(
        &self,
        version: &ArtifactVersion,
        get: impl FnOnce(&ArtifactStore, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        get(&self.artifacts, version).ok_or(ProviderError::Corrupt { version: *version })
    }

    /// Require and return the parsed DIR payload.
    pub fn dir_parsed(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> Result<Arc<DirParsed>, ProviderError> {
        let artifact_key = ArtifactKey::dir_parsed(module);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| artifacts.dir_parsed(version))
    }

    /// Require and return the parsed data payload.
    pub fn data(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> Result<Arc<Data>, ProviderError> {
        let artifact_key = ArtifactKey::data(module);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| artifacts.data(version))
    }

    /// Require and return the global environment payload.
    pub fn global_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<Arc<GlobalEnvironment>, ProviderError> {
        let artifact_key = ArtifactKey::global_environment(profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.global_environment(version)
        })
    }

    /// Require and return the bound DIR payload.
    pub fn dir_bound(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirBound>, ProviderError> {
        let artifact_key = ArtifactKey::dir_bound(module, profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| artifacts.dir_bound(version))
    }

    /// Require and return the imported DIR payload.
    pub fn dir_imported(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirImported>, ProviderError> {
        let artifact_key = ArtifactKey::dir_imported(module, profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.dir_imported(version)
        })
    }

    /// Require and return the expanded DIR payload.
    pub fn dir_expanded(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExpanded>, ProviderError> {
        let artifact_key = ArtifactKey::dir_expanded(module, profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.dir_expanded(version)
        })
    }

    /// Require and return the exported DIR payload.
    pub fn dir_exported(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExported>, ProviderError> {
        let artifact_key = ArtifactKey::dir_exported(module, profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.dir_exported(version)
        })
    }

    /// Require and return the checked DIR payload.
    pub fn dir_checked(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirChecked>, ProviderError> {
        let artifact_key = ArtifactKey::dir_checked(module, profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.dir_checked(version)
        })
    }

    /// Require and return the materialized DIR payload.
    pub fn dir_materialized(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirMaterialized>, ProviderError> {
        let artifact_key = ArtifactKey::dir_materialized(module, profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.dir_materialized(version)
        })
    }

    /// Require and return the elaborated DIR payload.
    pub fn dir_elaborated(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirElaborated>, ProviderError> {
        let artifact_key = ArtifactKey::dir_elaborated(module, profile);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.dir_elaborated(version)
        })
    }

    /// Require and return the lowered MIR payload.
    pub fn mir_lowered(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirLowered>, ProviderError> {
        let artifact_key = ArtifactKey::mir_lowered(module, profile, *target);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.mir_lowered(version)
        })
    }

    /// Require and return the verified MIR marker.
    pub fn mir_verified(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirVerified>, ProviderError> {
        let artifact_key = ArtifactKey::mir_verified(module, profile, *target);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.mir_verified(version)
        })
    }

    /// Require and return the optimized MIR payload.
    #[cfg(feature = "native")]
    pub fn mir_optimized(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirOptimized>, ProviderError> {
        let artifact_key = ArtifactKey::mir_optimized(module, profile, *target);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.mir_optimized(version)
        })
    }

    /// Require and return the generated module output.
    pub fn module_output(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        target: &TargetId,
    ) -> Result<Arc<ModuleOutput>, ProviderError> {
        let artifact_key = ArtifactKey::module_output(module, *target);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.module_output(version)
        })
    }

    /// Require and return the linked package output.
    pub fn package_output(
        &self,
        context: &dyn ProviderContext,
        package: PackageId,
        target: &TargetId,
    ) -> Result<Arc<PackageOutput>, ProviderError> {
        let artifact_key = ArtifactKey::package_output(package, *target);
        let version = context.require(artifact_key)?;

        self.require_artifact(&version, |artifacts, version| {
            artifacts.package_output(version)
        })
    }
}
