use std::sync::Arc;

use tspp_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, EnvironmentBound};
use tspp_mir as mir;
use tspp_repository::{ProfileId, ProviderContext};
use tspp_source::{ModuleId, TargetId};

use crate::lower::{LowerPhase, ModuleLowerer};
use crate::{Compiler, CompilerError, CompilerResult, LowerError};

impl Compiler {
    /// Collect the inputs declaring one module's MIR for one target.
    pub(crate) fn collect_mir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = self.dir_stage_dependencies(module, profile);
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;

        Ok(dependencies)
    }

    /// Collect the lowering inputs for one module and target.
    pub(crate) fn collect_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        let mut dependencies = self.dir_stage_dependencies(module, profile);
        dependencies.require(ArtifactKey::mir_declared(module, profile, target));
        self.observe_package_config(context, target.package_id(), &mut dependencies)?;
        Ok(dependencies)
    }

    /// Return the requirements on one module's own DIR stages.
    fn dir_stage_dependencies(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> ArtifactDependencySet {
        let mut dependencies = ArtifactDependencySet::default();
        for key in ArtifactKey::dir_stages(module, profile) {
            dependencies.require(key);
        }
        dependencies.require_payload(ArtifactKey::environment_bound(profile));

        dependencies
    }

    /// Return the ABI layout of one target.
    fn lower_target_layout(
        &self,
        context: &dyn ProviderContext,
        target: TargetId,
    ) -> CompilerResult<mir::TargetLayout> {
        let target_config =
            self.target_or_builtin(context, target)?
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("a missing configuration for the target '{target}'"),
                })?;

        Ok(self
            .target_layout(&target_config, target)
            .map_err(|message| LowerError::InvalidTarget {
                anchor: target.package_id().into(),
                package: target.package_id(),
                target,
                message,
            })?)
    }

    /// Declare one module's MIR for one target.
    pub(crate) fn provide_mir_declared(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let target_layout = self.lower_target_layout(context, target)?;
        let artifacts = self.artifact_reader(context);
        let environment = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let strings = self.repository.string_pool();
        let mut lower = ModuleLowerer::new(
            module,
            LowerPhase::Declare,
            strings,
            target,
            target_layout,
            self,
            context,
            artifacts,
            profile,
            environment,
        );

        // leave the failed declarations out, the module's own lowering reporting them
        let (declared, _) = lower.declare()?;

        Ok(ArtifactPayload::MirDeclared(Arc::new(declared)))
    }

    /// Lower one module for one target.
    pub(crate) fn provide_mir(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        let target_layout = self.lower_target_layout(context, target)?;

        // lower the module against the environment and the repository string pool
        let artifacts = self.artifact_reader(context);
        let environment = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let strings = self.repository.string_pool();
        let mut lower = ModuleLowerer::new(
            module,
            LowerPhase::Lower,
            strings,
            target,
            target_layout,
            self,
            context,
            artifacts,
            profile,
            environment,
        );
        let (lowered, mut errors) = lower.lower()?;

        // emit every lowering diagnostic and fail the artifact on the last
        let Some(last) = errors.pop() else {
            return Ok(ArtifactPayload::MirLowered(Arc::new(lowered)));
        };
        for error in errors {
            context.emit(error.as_ref())?;
        }

        Err(CompilerError::Diagnostic(last))
    }

    /// Return the canonical symbol path of one module.
    pub(in crate::lower) fn module_symbol_path(
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
        let path = path.strip_suffix(".tspp").unwrap_or(path);
        let path = path.trim_matches('/').replace('/', ".");

        // read the name the package namespaces its modules under
        let name = package
            .name
            .as_deref()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("a missing name for the package {:?}", package.id),
            })?;

        // qualify the module path under the package name
        if path.is_empty() {
            Ok(name.to_string())
        } else {
            Ok(format!("{name}.{path}"))
        }
    }
}
