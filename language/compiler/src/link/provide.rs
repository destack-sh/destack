use super::js::JsLinker;
use super::native::NativeLinker;
use super::state::LinkState;
use crate::{Compiler, CompilerError, CompilerResult, LinkError};
use destack_artifact::{ArtifactDependencySet, ArtifactPayload, EmitFormat, PackageOutput};
use destack_repository::{ArtifactReader, ProviderContext, RepositoryError, Target};
use destack_source::{ModuleId, PackageId, TargetId};
use std::path::PathBuf;
use std::sync::Arc;

/// The shared inputs resolved before linking one package target.
struct TargetLinkSetup {
    /// The resolved target configuration.
    target: Target,
    /// The target's link root modules in stable order.
    modules: Vec<ModuleId>,
    /// The package directory backing output resolution.
    package_directory: PathBuf,
    /// The configured root directory for output layout.
    root_directory: Option<PathBuf>,
}

impl Compiler {
    /// Collect inputs for one package output.
    pub(crate) fn collect_package_output(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let artifacts = self.artifact_reader(context.revision());
        let setup = self.target_link_setup(package, &target, context)?;
        let mut dependencies = ArtifactDependencySet::default();
        self.observe_package_config(context, package, &mut dependencies)?;

        // declare the reachable artifact closure of the selected linker family
        match setup.target.emit {
            EmitFormat::Js | EmitFormat::Ts => self
                .js_linker(package, &target, context, &artifacts, &setup)?
                .collect_modules(&setup.modules, &mut dependencies)?,
            EmitFormat::Wasm | EmitFormat::Native => self
                .native_linker(package, &target, context, &artifacts, &setup)?
                .collect_modules(&setup.modules, &mut dependencies),
        }

        Ok(dependencies)
    }

    /// Build one package output.
    pub(crate) fn provide_package_output(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = LinkState::new(package, target, context);
        let artifacts = self.artifact_reader(context.revision());
        let output = self.link_target(state.package, &state.target, state.context, &artifacts)?;

        Ok(ArtifactPayload::PackageOutput(Arc::new(output)))
    }

    /// Link all modules for one target.
    pub(crate) fn link_target<'a>(
        &'a self,
        package_id: PackageId,
        target_id: &'a TargetId,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
    ) -> CompilerResult<PackageOutput> {
        let setup = self.target_link_setup(package_id, target_id, context)?;

        // dispatch through the selected linker family
        let output = match setup.target.emit {
            EmitFormat::Js | EmitFormat::Ts => self
                .js_linker(package_id, target_id, context, artifacts, &setup)?
                .link_target(&setup.modules)?,
            EmitFormat::Wasm | EmitFormat::Native => self
                .native_linker(package_id, target_id, context, artifacts, &setup)?
                .link_target(&setup.modules)?,
        };

        Ok(output)
    }

    /// Resolve the shared inputs for linking one package target.
    fn target_link_setup(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<TargetLinkSetup> {
        let package = self.package(context.revision(), package_id)?;
        let config = self.destack_for_package(context, package_id)?;
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
        let package_directory = self.package_directory(package.path.clone());

        Ok(TargetLinkSetup {
            target,
            modules,
            package_directory,
            root_directory,
        })
    }

    /// Build the JS linker for one resolved target setup.
    fn js_linker<'a>(
        &'a self,
        package_id: PackageId,
        target_id: &'a TargetId,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        setup: &'a TargetLinkSetup,
    ) -> CompilerResult<JsLinker<'a>> {
        let linker = JsLinker::new(
            self,
            context,
            artifacts,
            &setup.package_directory,
            setup.root_directory.as_deref(),
            &setup.target,
            target_id,
            package_id,
        )?;

        Ok(linker)
    }

    /// Build the native linker for one resolved target setup.
    fn native_linker<'a>(
        &'a self,
        package_id: PackageId,
        target_id: &'a TargetId,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        setup: &'a TargetLinkSetup,
    ) -> CompilerResult<NativeLinker<'a>> {
        let linker = NativeLinker::new(
            self,
            context,
            artifacts,
            &setup.package_directory,
            setup.root_directory.as_deref(),
            &setup.target,
            target_id,
            package_id,
        )?;

        Ok(linker)
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
