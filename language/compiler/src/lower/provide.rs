use std::sync::Arc;

use destack_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, TargetArch};
use destack_repository::{ProfileId, ProviderContext};
use destack_source::{ModuleId, TargetId};

use crate::lower::ModuleLowerer;
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

        // lower the module against the repository string pool
        let strings = self.repository.string_pool();
        let mut lowerer = ModuleLowerer::new(
            module,
            &parsed,
            &bound,
            &expanded,
            &checked,
            &materialized,
            strings,
        );
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
}
