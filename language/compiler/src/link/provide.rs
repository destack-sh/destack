use super::binary::BinaryLinker;
use super::script::ScriptLinker;
use super::state::LinkState;
use crate::{Compiler, CompilerResult, LinkError};
use destack_artifact::{ArtifactPayload, EmitFormat, PackageOutput};
use destack_source::{PackageId, TargetId};
use destack_workspace::{ProviderContext, TargetDiscoveryError};

impl Compiler {
    /// Build one package output.
    pub(crate) fn provide_package_output(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = LinkState::new(package, target, context);
        let output = self.link_target(state.package, &state.target, state.context)?;

        Ok(ArtifactPayload::PackageOutput(output))
    }

    /// Link all modules for one target.
    pub(crate) fn link_target(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<PackageOutput> {
        // load package and target configuration
        let package = self.package(context.revision(), package_id);
        let config = self.destack_config_for_package(context, package_id);
        let package_path = package.path.clone();
        let root_directory = config
            .as_ref()
            .and_then(|config| config.compiler.root_dir.clone());
        let target =
            self.effective_target(context, *target_id)
                .ok_or(LinkError::MissingTarget {
                    anchor: (package_id).into(),
                    package: package_id,
                    target: *target_id,
                })?;
        let mut modules = self
            .target_module_ids(context, target_id)
            .map_err(Self::target_discovery_error)?;
        modules.sort_unstable();
        let package_directory = self.package_directory(package_path);

        // dispatch through the selected linker family
        let output = match target.emit {
            EmitFormat::Js | EmitFormat::Ts | EmitFormat::Html => ScriptLinker::new(
                self,
                context,
                &package_directory,
                root_directory.as_deref(),
                &target,
                target_id,
                package_id,
            )
            .link_target(&modules)?,
            EmitFormat::Wasm | EmitFormat::Native => BinaryLinker::new(
                self,
                context,
                &package_directory,
                root_directory.as_deref(),
                &target,
                target_id,
                package_id,
            )
            .link_target(&modules)?,
        };

        Ok(output)
    }

    /// Return the package directory used for linked output resolution.
    fn package_directory(&self, package_path: Option<std::path::PathBuf>) -> std::path::PathBuf {
        package_path.unwrap_or_else(|| self.repository.workspace_root().to_path_buf())
    }

    /// Map one target discovery failure into a link error.
    fn target_discovery_error(error: TargetDiscoveryError) -> LinkError {
        match error {
            TargetDiscoveryError::RepositoryRead {
                package,
                target,
                error,
            } => LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message: error.to_string(),
            },
            TargetDiscoveryError::MissingPackage { package, target } => LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message: "target package not found".to_string(),
            },
            TargetDiscoveryError::MissingPackagePath { package, target } => {
                LinkError::InvalidTarget {
                    anchor: package.into(),
                    package,
                    target,
                    message: "entry based discovery requires package path".to_string(),
                }
            }
            TargetDiscoveryError::MissingModulePath {
                package,
                target,
                path,
            } => LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message: format!("target module path not found: {}", path.display()),
            },
            TargetDiscoveryError::MissingTarget { package, target } => LinkError::MissingTarget {
                anchor: package.into(),
                package,
                target,
            },
        }
    }
}
