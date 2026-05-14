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
    /// Return one artifact version without adding a direct dependency.
    pub(crate) fn artifact_version(
        &self,
        context: &dyn ProviderContext,
        artifact_key: ArtifactKey,
    ) -> Result<ArtifactVersion, ProviderError> {
        self.repository
            .artifact_version(context.revision(), &artifact_key)
            .map_err(|error| ProviderError::Internal {
                message: format!("failed to read artifact version: {error}"),
            })?
            .ok_or_else(|| ProviderError::Internal {
                message: format!("artifact was not ready: {artifact_key:?}"),
            })
    }

    /// Return one required payload or report artifact store corruption.
    pub(crate) fn required_artifact<T>(
        &self,
        version: &ArtifactVersion,
        load: impl FnOnce(&ArtifactStore, &ArtifactVersion) -> Option<Arc<T>>,
    ) -> Result<Arc<T>, ProviderError> {
        load(&self.artifacts, version).ok_or(ProviderError::Corrupt { version: *version })
    }

    /// Return the parsed DIR payload.
    pub fn dir_parsed(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> Result<Arc<DirParsed>, ProviderError> {
        let artifact_key = ArtifactKey::dir_parsed(module);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| artifacts.dir_parsed(version))
    }

    /// Return the parsed data payload.
    pub fn data(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> Result<Arc<Data>, ProviderError> {
        let artifact_key = ArtifactKey::data(module);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| artifacts.data(version))
    }

    /// Return the global environment payload.
    pub fn global_environment(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
    ) -> Result<Arc<GlobalEnvironment>, ProviderError> {
        let artifact_key = ArtifactKey::global_environment(profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.global_environment(version)
        })
    }

    /// Return the bound DIR payload.
    pub fn dir_bound(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirBound>, ProviderError> {
        let artifact_key = ArtifactKey::dir_bound(module, profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| artifacts.dir_bound(version))
    }

    /// Return the imported DIR payload.
    pub fn dir_imported(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirImported>, ProviderError> {
        let artifact_key = ArtifactKey::dir_imported(module, profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_imported(version)
        })
    }

    /// Return the expanded DIR payload.
    pub fn dir_expanded(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExpanded>, ProviderError> {
        let artifact_key = ArtifactKey::dir_expanded(module, profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_expanded(version)
        })
    }

    /// Return the exported DIR payload.
    pub fn dir_exported(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirExported>, ProviderError> {
        let artifact_key = ArtifactKey::dir_exported(module, profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_exported(version)
        })
    }

    /// Return the checked DIR payload.
    pub fn dir_checked(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirChecked>, ProviderError> {
        let artifact_key = ArtifactKey::dir_checked(module, profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_checked(version)
        })
    }

    /// Return the materialized DIR payload.
    pub fn dir_materialized(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirMaterialized>, ProviderError> {
        let artifact_key = ArtifactKey::dir_materialized(module, profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_materialized(version)
        })
    }

    /// Return the elaborated DIR payload.
    pub fn dir_elaborated(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<Arc<DirElaborated>, ProviderError> {
        let artifact_key = ArtifactKey::dir_elaborated(module, profile);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.dir_elaborated(version)
        })
    }

    /// Return the lowered MIR payload.
    pub fn mir_lowered(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirLowered>, ProviderError> {
        let artifact_key = ArtifactKey::mir_lowered(module, profile, *target);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.mir_lowered(version)
        })
    }

    /// Return the verified MIR marker.
    pub fn mir_verified(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirVerified>, ProviderError> {
        let artifact_key = ArtifactKey::mir_verified(module, profile, *target);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.mir_verified(version)
        })
    }

    /// Return the optimized MIR payload.
    #[cfg(feature = "native")]
    pub fn mir_optimized(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<Arc<MirOptimized>, ProviderError> {
        let artifact_key = ArtifactKey::mir_optimized(module, profile, *target);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.mir_optimized(version)
        })
    }

    /// Return the generated module output.
    pub fn module_output(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
        target: &TargetId,
    ) -> Result<Arc<ModuleOutput>, ProviderError> {
        let artifact_key = ArtifactKey::module_output(module, *target);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.module_output(version)
        })
    }

    /// Return the linked package output.
    pub fn package_output(
        &self,
        context: &dyn ProviderContext,
        package: PackageId,
        target: &TargetId,
    ) -> Result<Arc<PackageOutput>, ProviderError> {
        let artifact_key = ArtifactKey::package_output(package, *target);
        let version = self.artifact_version(context, artifact_key)?;

        self.required_artifact(&version, |artifacts, version| {
            artifacts.package_output(version)
        })
    }
}
