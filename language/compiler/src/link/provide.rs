use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use tspp_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, Bundle, Output};
use tspp_program::Object;
use tspp_repository::{ArtifactReader, ProviderContext, ProviderError, RepositoryError, Target};
use tspp_source::{ModuleId, PackageId, ProductId, TargetId};

use crate::{Compiler, CompilerError, CompilerResult, LinkError};

use super::ProductLinker;
use super::js::JsLinker;
use super::program::ProgramLinker;
use super::state::LinkState;

/// One target resolved for linking.
struct ResolvedTarget {
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
    /// Collect inputs for one bundle.
    pub(crate) fn collect_bundle(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let artifacts = self.artifact_reader(context);
        let resolved = self.resolve_link_target(package, &target, context)?;
        let mut dependencies = ArtifactDependencySet::default();
        self.observe_package_config(context, package, &mut dependencies)?;

        // declare the reachable artifact closure of the selected linker family
        match resolved.target.output {
            Output::Bundle => self
                .js_linker(package, &target, context, &artifacts, &resolved)?
                .collect_modules(&resolved.modules, &mut dependencies)?,
            Output::Program => {
                return Err(LinkError::InvalidTarget {
                    anchor: package.into(),
                    package,
                    target,
                    message: "Program targets do not produce bundles".to_string(),
                }
                .into());
            }
        }

        Ok(dependencies)
    }

    /// Build one bundle.
    pub(crate) fn provide_bundle(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let state = LinkState::new(package, target, context);
        let artifacts = self.artifact_reader(context);
        let output = self.link_target(state.package, &state.target, state.context, &artifacts)?;

        Ok(ArtifactPayload::Bundle(Arc::new(output)))
    }

    /// Collect inputs for one linked Program.
    pub(crate) fn collect_program(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let resolved = self.resolve_link_target(package, &target, context)?;
        if resolved.target.output != Output::Program {
            return Err(LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message: "script targets do not produce Program artifacts".to_string(),
            }
            .into());
        }

        let artifacts = self.artifact_reader(context);
        let mut dependencies = ArtifactDependencySet::default();
        let mut modules = resolved.modules;
        let mut discovered: HashSet<ModuleId> = modules.iter().copied().collect();
        let mut index = 0;

        // walk the object dependency graph from the target roots
        while let Some(&module) = modules.get(index) {
            dependencies.require(ArtifactKey::object(module, target));
            match artifacts.read::<Object>((module, target)) {
                Ok(object) => {
                    for dependency in object.dependencies() {
                        if discovered.insert(*dependency) {
                            modules.push(*dependency);
                        }
                    }
                }
                Err(ProviderError::Blocked { .. }) => dependencies.mark_partial(),
                Err(error) => return Err(error.into()),
            }
            index += 1;
        }

        // observe target package configuration
        self.observe_package_config(context, package, &mut dependencies)?;

        Ok(dependencies)
    }

    /// Build one linked Program.
    pub(crate) fn provide_program(
        &self,
        package: PackageId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let resolved = self.resolve_link_target(package, &target, context)?;
        if resolved.target.output != Output::Program {
            return Err(LinkError::InvalidTarget {
                anchor: package.into(),
                package,
                target,
                message: "script targets do not produce Program artifacts".to_string(),
            }
            .into());
        }

        let artifacts = self.artifact_reader(context);
        let roots = resolved.modules.clone();
        let mut modules = resolved.modules;
        let mut discovered: HashSet<ModuleId> = modules.iter().copied().collect();
        let mut objects = Vec::new();
        let mut index = 0;

        // load the complete object dependency graph in stable breadth-first order
        while let Some(&module) = modules.get(index) {
            let object = artifacts
                .read::<Object>((module, target))
                .map_err(CompilerError::from)?;
            for dependency in object.dependencies() {
                if discovered.insert(*dependency) {
                    modules.push(*dependency);
                }
            }
            objects.push((module, object));
            index += 1;
        }
        // link module objects into one Program
        let program = ProgramLinker::new(package, objects, self.strings())?
            .with_roots(roots)
            .link()?;

        Ok(ArtifactPayload::Program(Arc::new(program)))
    }

    /// Collect inputs for one product.
    pub(crate) fn collect_product(
        &self,
        package: PackageId,
        product: ProductId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        ProductLinker::new(self, package, product, context)?.collect()
    }

    /// Build one product.
    pub(crate) fn provide_product(
        &self,
        package: PackageId,
        product: ProductId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let artifacts = self.artifact_reader(context);
        let output = ProductLinker::new(self, package, product, context)?.link(&artifacts)?;

        Ok(ArtifactPayload::Product(Arc::new(output)))
    }

    /// Link all modules for one target.
    pub(crate) fn link_target<'a>(
        &'a self,
        package_id: PackageId,
        target_id: &'a TargetId,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
    ) -> CompilerResult<Bundle> {
        let resolved = self.resolve_link_target(package_id, target_id, context)?;

        // dispatch through the selected linker family
        let output = match resolved.target.output {
            Output::Bundle => self
                .js_linker(package_id, target_id, context, artifacts, &resolved)?
                .link_target(&resolved.modules)?,
            Output::Program => {
                return Err(LinkError::InvalidTarget {
                    anchor: package_id.into(),
                    package: package_id,
                    target: *target_id,
                    message: "Program targets do not produce bundles".to_string(),
                }
                .into());
            }
        };

        Ok(output)
    }

    /// Resolve one package target for linking.
    fn resolve_link_target(
        &self,
        package_id: PackageId,
        target_id: &TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ResolvedTarget> {
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

        Ok(ResolvedTarget {
            target,
            modules,
            package_directory,
            root_directory,
        })
    }

    /// Build the JS linker for one resolved target.
    fn js_linker<'a>(
        &'a self,
        package_id: PackageId,
        target_id: &'a TargetId,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        resolved: &'a ResolvedTarget,
    ) -> CompilerResult<JsLinker<'a>> {
        let linker = JsLinker::new(
            self,
            context,
            artifacts,
            &resolved.package_directory,
            resolved.root_directory.as_deref(),
            &resolved.target,
            target_id,
            package_id,
        )?;

        Ok(linker)
    }

    /// Return the package directory used for linked output resolution.
    fn package_directory(&self, package_path: Option<PathBuf>) -> PathBuf {
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
