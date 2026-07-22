use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ComponentGraphProjection, TargetArch,
};
use destack_core::FxIndexMap;
use destack_repository::{ProfileId, ProviderContext};
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
        dependencies.require(ArtifactKey::dir_checked(module, profile));
        dependencies.require(ArtifactKey::dir_materialized(module, profile));

        // require the sealed stacks of every component sibling
        for sibling in self.component_siblings(module, profile, context, &mut dependencies)? {
            dependencies.require(ArtifactKey::dir_parsed(sibling));
            dependencies.require(ArtifactKey::dir_bound(sibling, profile));
            dependencies.require(ArtifactKey::dir_expanded(sibling, profile));
            dependencies.require(ArtifactKey::dir_checked(sibling, profile));
            dependencies.require(ArtifactKey::dir_materialized(sibling, profile));
        }

        // observe package config for target resolution
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Return the other members of one module's component, when the graph is ready.
    fn component_siblings(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
        dependencies: &mut ArtifactDependencySet,
    ) -> CompilerResult<Vec<ModuleId>> {
        // project the component membership around this module
        let graph_key = ArtifactKey::component_graph(profile);
        let artifacts = self.artifact_reader(context.revision());
        let graph = match artifacts.component_graph(profile) {
            Ok(graph) => graph,
            Err(_) => {
                dependencies.mark_partial();

                return Ok(Vec::new());
            }
        };
        let Some(component) = graph.component(module) else {
            return Ok(Vec::new());
        };

        // close over the component and everything it depends on
        let mut components = vec![component];
        let mut index = 0;
        while index < components.len() {
            let current = components[index];
            index += 1;
            dependencies.project(graph_key, ComponentGraphProjection::Members(current));
            dependencies.project(graph_key, ComponentGraphProjection::Dependencies(current));
            for dependency in graph.dependencies(current) {
                if !components.contains(dependency) {
                    components.push(*dependency);
                }
            }
        }

        let siblings = components
            .iter()
            .flat_map(|component| graph.members(*component))
            .copied()
            .filter(|sibling| *sibling != module)
            .collect();

        Ok(siblings)
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

        // load provider inputs
        let artifacts = self.artifact_reader(context.revision());
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module, profile)
            .map_err(CompilerError::from)?;
        let materialized = artifacts
            .dir_materialized(module, profile)
            .map_err(CompilerError::from)?;

        // load the sealed check output of every reachable module
        let mut modules = FxIndexMap::default();
        if let Ok(graph) = artifacts.component_graph(profile)
            && let Some(component) = graph.component(module)
        {
            let mut components = vec![component];
            let mut index = 0;
            while index < components.len() {
                let current = components[index];
                index += 1;
                for dependency in graph.dependencies(current) {
                    if !components.contains(dependency) {
                        components.push(*dependency);
                    }
                }
            }
            let siblings: Vec<_> = components
                .iter()
                .flat_map(|component| graph.members(*component))
                .copied()
                .collect();
            for sibling in &siblings {
                if *sibling == module {
                    continue;
                }
                let parsed = artifacts
                    .dir_parsed(*sibling)
                    .map_err(CompilerError::from)?;
                let bound = artifacts
                    .dir_bound(*sibling, profile)
                    .map_err(CompilerError::from)?;
                let expanded = artifacts
                    .dir_expanded(*sibling, profile)
                    .map_err(CompilerError::from)?;
                let checked = artifacts
                    .dir_checked(*sibling, profile)
                    .map_err(CompilerError::from)?;
                let materialized = artifacts
                    .dir_materialized(*sibling, profile)
                    .map_err(CompilerError::from)?;
                let path = self.module_symbol_path(context, *sibling)?;
                modules.insert(
                    *sibling,
                    LowerModuleState::new(parsed, &bound, &expanded, &checked, &materialized, path),
                );
            }
        }

        // lower the module against the repository string pool
        let strings = self.repository.string_pool();
        let path = self.module_symbol_path(context, module)?;
        modules.insert(
            module,
            LowerModuleState::new(parsed, &bound, &expanded, &checked, &materialized, path),
        );
        let mut lowerer = ModuleLowerer::new(module, strings, modules);
        let (lowered, mut errors) = lowerer.lower(pointer_bytes)?;

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
