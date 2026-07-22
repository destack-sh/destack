use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ComponentGraphProjection, TargetArch,
};
use destack_core::FxIndexMap;
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{ModuleId, TargetId};

use crate::lower::{LowerModuleState, ModuleLowerer};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for MIR of one module and target.
    pub(crate) fn collect_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
        dependencies.require(ArtifactKey::dir_checked(module, profile));
        dependencies.require(ArtifactKey::dir_materialized(module, profile));

        // project the component membership around the lowered module
        let graph_key = ArtifactKey::component_graph(profile);
        dependencies.project(graph_key, ComponentGraphProjection::ComponentOf(module));
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                self.observe_package_config(context, target.package_id(), &mut dependencies)?;

                return Ok(dependencies);
            }
            Err(error) => return Err(error.into()),
        };
        let component = graph
            .component(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("component graph does not contain MIR module {module:?}"),
            })?;

        // require every checked module that can contribute a runtime declaration
        let mut components = vec![component];
        components.extend(graph.transitive_dependencies(component));
        for component in components {
            dependencies.project(graph_key, ComponentGraphProjection::Members(component));
            dependencies.project(graph_key, ComponentGraphProjection::Dependencies(component));

            for dependency in graph.members(component) {
                dependencies.require(ArtifactKey::dir_parsed(*dependency));
                dependencies.require(ArtifactKey::dir_bound(*dependency, profile));
                dependencies.require(ArtifactKey::dir_expanded(*dependency, profile));
                dependencies.require(ArtifactKey::dir_resolved(*dependency, profile));
                dependencies.require(ArtifactKey::dir_checked(*dependency, profile));
                dependencies.require(ArtifactKey::dir_materialized(*dependency, profile));
            }
        }

        // observe package config for target resolution
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Provide MIR for one module and target.
    pub(crate) fn provide_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // resolve the target ABI
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("target '{target}' not found"),
                })?;
        let pointer_bytes = target_config
            .native
            .arch
            .as_ref()
            .map(TargetArch::pointer_bytes)
            .unwrap_or(8);

        // require the component graph selected during collection
        let artifacts = self.artifact_reader(context.revision());
        let graph = artifacts
            .component_graph(profile)
            .map_err(CompilerError::from)?;
        let component = graph
            .component(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("component graph does not contain MIR module {module:?}"),
            })?;

        // load the sealed check output of every reachable module
        let mut modules = FxIndexMap::default();
        let mut components = vec![component];
        components.extend(graph.transitive_dependencies(component));
        for component in components {
            for dependency in graph.members(component) {
                let parsed = artifacts
                    .dir_parsed(*dependency)
                    .map_err(CompilerError::from)?;
                let bound = artifacts
                    .dir_bound(*dependency, profile)
                    .map_err(CompilerError::from)?;
                let expanded = artifacts
                    .dir_expanded(*dependency, profile)
                    .map_err(CompilerError::from)?;
                let resolved = artifacts
                    .dir_resolved(*dependency, profile)
                    .map_err(CompilerError::from)?;
                let checked = artifacts
                    .dir_checked(*dependency, profile)
                    .map_err(CompilerError::from)?;
                let materialized = artifacts
                    .dir_materialized(*dependency, profile)
                    .map_err(CompilerError::from)?;
                let path = self.module_symbol_path(context, *dependency)?;
                modules.insert(
                    *dependency,
                    LowerModuleState::new(
                        parsed,
                        &bound,
                        &expanded,
                        &resolved,
                        &checked,
                        &materialized,
                        path,
                    ),
                );
            }
        }

        // lower the module against the repository string pool
        let strings = self.repository.string_pool();
        let mut lowerer = ModuleLowerer::new(module, strings, modules, pointer_bytes);
        let (lowered, mut errors) = lowerer.lower()?;

        // emit every lowering diagnostic and fail the artifact when any occurred
        let Some(last) = errors.pop() else {
            return Ok(ArtifactPayload::MirLowered(Arc::new(lowered)));
        };
        for error in errors {
            context.emit(error.as_ref())?;
        }

        Err(CompilerError::Diagnostic(last))
    }

    /// Return the canonical symbol path of one module.
    ///
    /// Named packages namespace their modules as `{package}.{path}` with the
    /// module path relative to the package root; the anonymous root package
    /// contributes its module paths bare.
    // TODO(lower): version-disambiguate package prefixes once dependency
    // resolution can hold two versions of one package in a program.
    fn module_symbol_path(
        &self,
        context: &dyn ProviderContext,
        module: ModuleId,
    ) -> CompilerResult<String> {
        let record = self.module(context.revision(), module)?;
        let package = self.package(context.revision(), record.package_id)?;

        // resolve the module path relative to its package root
        let uri = record.uri.to_string();
        let base = package.uri.to_string();
        let path = uri.strip_prefix(&base).unwrap_or(&uri);
        let path = path
            .rsplit_once("://")
            .map(|(_, path)| path)
            .unwrap_or(path);
        let path = path.strip_suffix(".ds").unwrap_or(path);
        let path = path.trim_matches('/').replace('/', ".");

        Ok(match &package.name {
            Some(name) if path.is_empty() => name.clone(),
            Some(name) => format!("{name}.{path}"),
            None => path,
        })
    }
}
