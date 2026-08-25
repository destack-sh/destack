use std::sync::Arc;

use destack_artifact::{
    ArtifactDependencySet, ArtifactKey, ArtifactPayload, EnvironmentBound, EnvironmentDeclared,
};
use destack_repository::{ArtifactAttemptRecorder, ProfileId, ProviderContext};
use destack_source::ModuleId;

use crate::sema::{CheckModuleState, CheckState, Pass};
use crate::{Compiler, CompilerError, CompilerResult};

impl Compiler {
    /// Collect inputs for materialized DIR of one module.
    pub(crate) fn collect_dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        _context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactDependencySet> {
        // require this module's own stage artifacts
        let mut dependencies = ArtifactDependencySet::default();
        dependencies.require(ArtifactKey::dir_parsed(module));
        dependencies.require(ArtifactKey::dir_bound(module, profile));
        dependencies.require(ArtifactKey::dir_resolved(module, profile));
        dependencies.require(ArtifactKey::dir_expanded(module, profile));
        dependencies.require(ArtifactKey::dir_declared(module, profile));
        dependencies.require(ArtifactKey::dir_elaborated(module, profile));
        dependencies.require(ArtifactKey::dir_checked(module, profile));

        // require the aggregate implicit declarations
        dependencies.require_payload(ArtifactKey::environment_bound(profile));
        dependencies.require_payload(ArtifactKey::environment_declared(profile));

        Ok(dependencies)
    }

    /// Build materialized DIR for one module.
    pub(crate) fn provide_dir_materialized(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<ArtifactPayload> {
        // read the profile's environment
        let artifacts = self.artifact_reader(context);
        let global = artifacts
            .read::<EnvironmentBound>(profile)
            .map_err(CompilerError::from)?;
        let declared_environment = artifacts
            .read::<EnvironmentDeclared>(profile)
            .map_err(CompilerError::from)?;
        let environment = self.environment(context.revision())?;

        // load the evaluation state over the module's checked artifacts
        let mut check = ArtifactAttemptRecorder::breakdown_maybe(
            context.recorder(),
            "load",
            || -> CompilerResult<_> {
                let module = CheckModuleState::load(
                    self,
                    context,
                    &artifacts,
                    profile,
                    module,
                    Pass::Materialize,
                )?;

                Ok(CheckState::new(
                    self,
                    context,
                    &artifacts,
                    profile,
                    global,
                    Some(declared_environment),
                    environment,
                    module,
                    Pass::Materialize,
                    false,
                ))
            },
        )?;

        // run the pass
        ArtifactAttemptRecorder::breakdown_maybe(context.recorder(), "run", || {
            check.run_materialize()
        })?;

        let materialized = check.into_materialized()?;

        Ok(ArtifactPayload::DirMaterialized(Arc::new(materialized)))
    }
}
