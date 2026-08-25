use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, ArtifactProjectionKey, DirBound,
    DirChecked, DirDeclared, DirElaborated, DirExpanded, DirMaterialized, DirParsed,
};
use destack_core::FxIndexMap;
use destack_repository::{ProfileId, ProviderContext, ProviderError};
use destack_source::{ModuleId, TargetId};

use crate::lower::{LowerModuleState, ModuleLowerer};
use crate::{Compiler, CompilerError, CompilerResult, LowerError};

impl Compiler {
    /// Collect the lowering inputs for one module and target.
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

        // require the artifact stack of every reachable module
        for reachable in self.reachable_modules(module, profile, context, &mut dependencies)? {
            dependencies.require(ArtifactKey::dir_parsed(reachable));
            dependencies.require(ArtifactKey::dir_bound(reachable, profile));
            dependencies.require(ArtifactKey::dir_expanded(reachable, profile));
            dependencies.require(ArtifactKey::dir_checked(reachable, profile));
            dependencies.require(ArtifactKey::dir_materialized(reachable, profile));
        }

        // observe package config for target resolution
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Return other modules reachable through one module's reference graph.
    fn reachable_modules(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
        dependencies: &mut ArtifactDependencySet,
    ) -> CompilerResult<Vec<ModuleId>> {
        // walk the import closure read by lowering, requiring the root
        //  edges first so a blocked read schedules the graph
        let graph_key = ArtifactKey::module_graph(profile);
        dependencies.require_projection(graph_key, ArtifactProjectionKey::ModuleGraphEdges(module));
        let artifacts = self.artifact_reader(context);
        let graph = match artifacts.module_graph_reader(profile) {
            Ok(graph) => graph,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(Vec::new());
            }
            Err(error) => return Err(error.into()),
        };
        let reachable = graph.reachable(&[module])?;
        for current in reachable.iter().copied() {
            dependencies
                .require_projection(graph_key, ArtifactProjectionKey::ModuleGraphEdges(current));
        }

        // collect every reachable module other than this one
        let modules = reachable
            .into_iter()
            .filter(|reachable| *reachable != module)
            .collect();

        Ok(modules)
    }

    /// Lower one module for one target.
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
        let target_layout = self
            .target_layout(&target_config, target)
            .map_err(|message| LowerError::InvalidTarget {
                anchor: target.package_id().into(),
                package: target.package_id(),
                target,
                message,
            })?;

        // load provider inputs
        let artifacts = self.artifact_reader(context);
        let parsed = artifacts
            .read::<DirParsed>(module)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .read::<DirBound>((module, profile))
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .read::<DirExpanded>((module, profile))
            .map_err(CompilerError::from)?;
        let declared = artifacts
            .read::<DirDeclared>((module, profile))
            .map_err(CompilerError::from)?;
        let elaborated = artifacts
            .read::<DirElaborated>((module, profile))
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .read::<DirChecked>((module, profile))
            .map_err(CompilerError::from)?;
        let materialized = artifacts
            .read::<DirMaterialized>((module, profile))
            .map_err(CompilerError::from)?;

        // resolve this module's import closure
        let graph = artifacts
            .module_graph_reader(profile)
            .map_err(CompilerError::from)?;
        let reachable = graph.reachable(&[module])?;

        // load the state of every other reachable module
        let mut modules = FxIndexMap::default();
        for reachable in reachable {
            if reachable == module {
                continue;
            }

            // load the bound and checked state of every reachable module
            let parsed = artifacts
                .read::<DirParsed>(reachable)
                .map_err(CompilerError::from)?;
            let bound = artifacts
                .read::<DirBound>((reachable, profile))
                .map_err(CompilerError::from)?;
            let expanded = artifacts
                .read::<DirExpanded>((reachable, profile))
                .map_err(CompilerError::from)?;
            let declared = artifacts
                .read::<DirDeclared>((reachable, profile))
                .map_err(CompilerError::from)?;
            let elaborated = artifacts
                .read::<DirElaborated>((reachable, profile))
                .map_err(CompilerError::from)?;
            let checked = artifacts
                .read::<DirChecked>((reachable, profile))
                .map_err(CompilerError::from)?;
            let materialized = artifacts
                .read::<DirMaterialized>((reachable, profile))
                .map_err(CompilerError::from)?;
            let path = self.module_symbol_path(context, reachable)?;
            modules.insert(
                reachable,
                LowerModuleState::new(
                    parsed,
                    &bound,
                    &expanded,
                    &declared,
                    &elaborated,
                    &checked,
                    &materialized,
                    path,
                ),
            );
        }

        // lower the module against the repository string pool
        let strings = self.repository.string_pool();
        let path = self.module_symbol_path(context, module)?;
        modules.insert(
            module,
            LowerModuleState::new(
                parsed,
                &bound,
                &expanded,
                &declared,
                &elaborated,
                &checked,
                &materialized,
                path,
            ),
        );
        let mut lowerer = ModuleLowerer::new(module, strings, modules);
        let (lowered, mut errors) = lowerer.lower(target_layout)?;

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

        // qualify the module path under its package name
        let name = package
            .name
            .as_deref()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("package {:?} has no name for lowering", package.id),
            })?;

        if path.is_empty() {
            Ok(name.to_string())
        } else {
            Ok(format!("{name}.{path}"))
        }
    }
}
