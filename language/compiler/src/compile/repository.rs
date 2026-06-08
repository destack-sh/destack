use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactDependency, ArtifactPathState};
use destack_repository::{
    CompilerOptions, DestackFile, Module, Package, Profile, ProviderContext, Revision, Target,
};
use destack_source::{File, FileId, ModuleId, PackageId, ProfileId, TargetId, Uri};

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

    /// Return one file from one provider attempt revision and record its content dependency.
    pub(crate) fn file(
        &self,
        context: &dyn ProviderContext,
        file_id: FileId,
    ) -> CompilerResult<Arc<File>> {
        let revision = context.revision();
        let file = self
            .repository
            .file(revision, file_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load file {file_id:?}: {error}"),
            })?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing file for {file_id:?}"),
            })?;

        self.track_file_content(context, file_id)?;

        Ok(file)
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

    /// Load one package Destack config and record its declaration file dependencies.
    pub(crate) fn destack_for_package(
        &self,
        context: &dyn ProviderContext,
        package_id: PackageId,
    ) -> CompilerResult<Option<Arc<DestackFile>>> {
        let revision = context.revision();
        let config = self
            .repository
            .destack_for_package_id(revision, package_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load package config {package_id:?}: {error}"),
            })?;

        // track every declaration that built the effective config
        if let Some(config) = config.as_ref() {
            for file_id in &config.file_ids {
                self.track_file_content(context, *file_id)?;
            }
        }

        Ok(config)
    }

    /// Return workspace compiler options for one module.
    pub(crate) fn workspace_compiler_options(
        &self,
        context: &dyn ProviderContext,
        module: &Module,
    ) -> CompilerResult<CompilerOptions> {
        let config = self.destack_for_package(context, module.package_id)?;
        let options = config
            .as_ref()
            .map(|config| config.compiler.clone())
            .unwrap_or_default();

        Ok(options)
    }

    /// Record one source file content dependency.
    pub(crate) fn track_file_content(
        &self,
        context: &dyn ProviderContext,
        file_id: FileId,
    ) -> CompilerResult<()> {
        let revision = context.revision();
        let content_id = self
            .repository
            .file_content_id(revision, file_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load file content id for {file_id:?}: {error}"),
            })?
            .ok_or_else(|| CompilerError::Internal {
                message: format!("missing file content id for {file_id:?}"),
            })?;

        context.track(ArtifactDependency::path_state(
            file_id,
            ArtifactPathState::File,
        ));
        context.track(ArtifactDependency::file_content(file_id, content_id));

        Ok(())
    }

    /// Return one target or built-in and record its source config dependencies.
    pub(crate) fn target_or_builtin(
        &self,
        context: &dyn ProviderContext,
        target_id: TargetId,
    ) -> CompilerResult<Option<Target>> {
        let _config = self.destack_for_package(context, target_id.package_id())?;
        let target = self
            .repository
            .target_or_builtin(context.revision(), target_id)
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to load target {target_id:?}: {error}"),
            })?;

        Ok(target)
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

    /// Resolve the target profile id for one module in one revision.
    pub(crate) fn profile_id_for_target(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: &TargetId,
    ) -> CompilerResult<ProfileId> {
        let profile = self
            .repository
            .profile_for_module_target(revision, module_id, *target_id)
            .map_err(|error| CompilerError::Internal {
                message: format!(
                    "failed to resolve target profile for module {module_id:?} target {target_id:?}: {error}"
                ),
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

    /// Normalize one workspace logical path.
    pub(crate) fn normalize_workspace_path(&self, path: PathBuf) -> Option<PathBuf> {
        let mut normalized = PathBuf::new();

        // fold lexical path components
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        return None;
                    }
                }
                Component::Normal(component) => normalized.push(component),
                Component::RootDir | Component::Prefix(_) => {}
            }
        }

        Some(normalized)
    }
}
