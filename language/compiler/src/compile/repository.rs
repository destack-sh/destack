use std::path::Path;
use std::sync::Arc;

use tspp_artifact::{ArtifactDependencySet, SourceDependency};
use tspp_repository::{
    Environment, ManifestFile, Module, Package, Profile, ProviderContext, Revision, Target,
};
use tspp_source::{File, FileId, ModuleId, PackageId, ProfileId, TargetId, Uri};

use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Return one derived semantic profile by id for one explicit revision.
    pub(crate) fn profile(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> CompilerResult<Profile> {
        let profile = self
            .repository
            .profile(revision, profile_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load profile {profile_id:?}: {error}"),
            })?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing compiler profile for {profile_id:?}"),
            })?;

        Ok(profile.as_ref().clone())
    }

    /// Return one module from one repository revision.
    pub(crate) fn module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> CompilerResult<Arc<Module>> {
        self.repository
            .module(revision, module_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load module {module_id:?}: {error}"),
            })?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing module for {module_id:?}"),
            })
    }

    /// Return one file from one provider attempt revision.
    pub(crate) fn file(
        &self,
        context: &dyn ProviderContext,
        file_id: FileId,
    ) -> CompilerResult<Arc<File>> {
        let revision = context.revision();

        self.repository
            .file(revision, file_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load file {file_id:?}: {error}"),
            })?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing file for {file_id:?}"),
            })
    }

    /// Return one package from one repository revision.
    pub(crate) fn package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> CompilerResult<Arc<Package>> {
        self.repository
            .package(revision, package_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load package {package_id:?}: {error}"),
            })?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing package for {package_id:?}"),
            })
    }

    /// Return the ambient environment captured by one repository revision.
    pub(crate) fn environment(&self, revision: Revision) -> CompilerResult<Arc<Environment>> {
        self.repository
            .environment(revision)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load environment for {revision}: {error}"),
            })
    }

    /// Load one package manifest and record its declaration file dependencies.
    pub(crate) fn manifest_for_package(
        &self,
        context: &dyn ProviderContext,
        package_id: PackageId,
    ) -> CompilerResult<Option<Arc<ManifestFile>>> {
        let revision = context.revision();
        let config = self
            .repository
            .manifest_for_package_id(revision, package_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load package config {package_id:?}: {error}"),
            })?;

        Ok(config)
    }

    /// Return one package's config declaration files as source dependencies.
    pub(crate) fn package_config_sources(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> CompilerResult<Vec<SourceDependency>> {
        let config = self
            .repository
            .manifest_for_package_id(revision, package_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load package config {package_id:?}: {error}"),
            })?;
        let Some(config) = config else {
            return Ok(Vec::new());
        };

        // every declaration that built the effective config feeds the fingerprint
        let mut sources = Vec::with_capacity(config.file_ids.len());
        for file_id in &config.file_ids {
            let blob = self
                .repository
                .file_blob(revision, *file_id)
                .map_err(|error| CompilerError::Internal {
                    message: format!(
                        "failed to load configuration File Blob for {file_id:?}: {error}"
                    ),
                })?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("missing configuration File Blob for {file_id:?}"),
                })?;
            sources.push(SourceDependency::file(*file_id, blob.id));
        }

        Ok(sources)
    }

    /// Observe one package's config declaration files as source dependencies.
    pub(crate) fn observe_package_config(
        &self,
        context: &dyn ProviderContext,
        package_id: PackageId,
        dependencies: &mut ArtifactDependencySet,
    ) -> CompilerResult<()> {
        for source in self.package_config_sources(context.revision(), package_id)? {
            dependencies.observe(source);
        }

        Ok(())
    }

    /// Return one target or built-in.
    ///
    /// Callers that resolve a target declare its package config separately
    /// through [`Self::observe_package_config`], so this only reads the target.
    pub(crate) fn target_or_builtin(
        &self,
        context: &dyn ProviderContext,
        target_id: TargetId,
    ) -> CompilerResult<Option<Target>> {
        self.repository
            .target_or_builtin(context.revision(), target_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load target {target_id:?}: {error}"),
            })
    }

    /// Return one target name from the repository model.
    pub(crate) fn target_name(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> CompilerResult<String> {
        self.repository
            .target_name(revision, target_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load target name {target_id:?}: {error}"),
            })
    }

    /// Resolve the profile id selected by one target in one revision.
    pub(crate) fn profile_id_for_target(
        &self,
        revision: Revision,
        target_id: &TargetId,
    ) -> CompilerResult<ProfileId> {
        let profile = self
            .repository
            .profile_for_target(revision, *target_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to resolve profile for target {target_id:?}: {error}"),
            })?;

        Ok(profile.id())
    }

    /// Return the module id for one path in a sealed revision.
    pub(crate) fn module_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> CompilerResult<Option<ModuleId>> {
        self.repository
            .module_id_for_path(revision, path)
            .map_err(|error| CompilerError::Internal {
                message: format!(
                    "failed to resolve module path '{}': {error}",
                    path.display()
                ),
            })
    }

    /// Return the module id for one URI in a sealed revision.
    pub(crate) fn module_id_for_uri(
        &self,
        revision: Revision,
        uri: &Uri,
    ) -> CompilerResult<Option<ModuleId>> {
        self.repository
            .module_id_for_uri(revision, uri)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to resolve module URI '{uri}': {error}"),
            })
    }
}
