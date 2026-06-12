use super::js::JsLinker;
use super::native::NativeLinker;
use super::state::LinkState;
use crate::{Compiler, CompilerError, CompilerResult, LinkError};
use destack_artifact::{ArtifactPayload, EmitFormat, PackageOutput};
use destack_repository::{ArtifactReader, ProviderContext, RepositoryError};
use destack_source::{PackageId, TargetId};

impl Compiler {
    /// Build one package output.
    pub(crate) fn provide_package_output(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = LinkState::new(package, target, context);
        let artifacts = self.artifact_reader(context);
        let output = self.link_target(state.package, &state.target, state.context, &artifacts)?;

        Ok(ArtifactPayload::PackageOutput(output))
    }

    /// Link all modules for one target.
    pub(crate) fn link_target<'a>(
        &'a self,
        package_id: PackageId,
        target_id: &TargetId,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
    ) -> CompilerResult<PackageOutput> {
        // load package and target configuration
        let package = self.package(context.revision(), package_id)?;
        let config = self.destack_for_package(context, package_id)?;
        let package_path = package.path.clone();
        let root_directory = config
            .as_ref()
            .and_then(|config| config.compiler.root_dir.clone());
        let target =
            self.target_or_builtin(context, *target_id)?
                .ok_or(LinkError::MissingTarget {
                    anchor: (package_id).into(),
                    package: package_id,
                    target: *target_id,
                })?;
        let mut modules = self
            .repository
            .modules_for_target(context.revision(), *target_id)
            .map_err(|error| Self::target_module_error(package_id, *target_id, error))?;
        modules.sort_unstable();
        modules.dedup();
        let package_directory = self.package_directory(package_path);

        // dispatch through the selected linker family
        let output = match target.emit {
            EmitFormat::Js | EmitFormat::Ts => JsLinker::new(
                self,
                context,
                artifacts,
                &package_directory,
                root_directory.as_deref(),
                &target,
                target_id,
                package_id,
            )?
            .link_target(&modules)?,
            EmitFormat::Wasm | EmitFormat::Native => NativeLinker::new(
                self,
                context,
                artifacts,
                &package_directory,
                root_directory.as_deref(),
                &target,
                target_id,
                package_id,
            )?
            .link_target(&modules)?,
        };

        Ok(output)
    }

    /// Return the package directory used for linked output resolution.
    fn package_directory(&self, package_path: Option<std::path::PathBuf>) -> std::path::PathBuf {
        package_path.unwrap_or_else(|| self.repository.path().to_path_buf())
    }

    /// Map one target module discovery failure into a link diagnostic.
    fn target_module_error(
        package: PackageId,
        target: TargetId,
        error: RepositoryError,
    ) -> LinkError {
        match error {
            RepositoryError::MissingTarget { .. } => LinkError::MissingTarget {
                anchor: package.into(),
                package,
                target,
            },
            error => LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message: error.to_string(),
            },
        }
    }

    /// Map one compiler boundary failure into a link diagnostic.
    pub(crate) fn link_error(package: PackageId, error: CompilerError) -> LinkError {
        LinkError::Internal {
            anchor: package.into(),
            package,
            message: format!("{error:?}"),
        }
    }
}
