use std::sync::Arc;

use tspp_artifact::{ArtifactDependencySet, ArtifactKey, ArtifactPayload, EnvironmentBound};
use tspp_repository::{ProfileId, ProviderContext, ProviderError, ProviderResult, Revision};
use tspp_source::{ModuleId, TargetId};

use super::Linter;

impl Linter {
    /// Collect dependencies for one lint artifact.
    pub fn collect(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactDependencySet> {
        let artifact_key = context.artifact_key();
        let dependencies = match artifact_key {
            ArtifactKey::ModuleLinted {
                module,
                profile,
                target,
            } => self.collect_module(context, module, profile, target),
            ArtifactKey::ProgramLinted { profile, target } => {
                self.collect_program(context, profile, target)
            }
            _ => Err(ProviderError::internal(format!(
                "non-linter artifact reached linter collection: {artifact_key:?}"
            ))),
        };

        dependencies.map_err(Box::new)
    }

    /// Provide one lint artifact.
    pub fn provide(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactPayload> {
        let artifact_key = context.artifact_key();
        let artifact = match artifact_key {
            ArtifactKey::ModuleLinted {
                module,
                profile,
                target,
            } => self.provide_module(context, module, profile, target),
            ArtifactKey::ProgramLinted { profile, target } => {
                self.provide_program(context, profile, target)
            }
            _ => Err(ProviderError::internal(format!(
                "non-linter artifact reached linter execution: {artifact_key:?}"
            ))),
        };

        artifact.map_err(Box::new)
    }

    /// Collect the bound environment required by DIR lints.
    pub(super) fn collect_environment_bound(
        &self,
        context: &dyn ProviderContext,
        profile: ProfileId,
        dependencies: &mut ArtifactDependencySet,
    ) -> Result<Option<Arc<EnvironmentBound>>, ProviderError> {
        dependencies.require(ArtifactKey::environment_bound(profile));
        let artifacts = self.artifact_reader(context);
        let environment = match artifacts.read::<EnvironmentBound>(profile) {
            Ok(environment) => environment,
            Err(ProviderError::Blocked { .. }) => {
                dependencies.mark_partial();

                return Ok(None);
            }
            Err(error) => return Err(error),
        };

        Ok(Some(environment))
    }

    /// Require the artifacts read by DIR lints.
    pub(super) fn require_dir_modules(
        &self,
        revision: Revision,
        modules: &[ModuleId],
        profile: ProfileId,
        dependencies: &mut ArtifactDependencySet,
    ) -> Result<(), ProviderError> {
        for module in modules.iter().copied() {
            if !self.module(revision, module)?.is_code() {
                continue;
            }

            dependencies.require(ArtifactKey::dir_parsed(module));
            dependencies.require(ArtifactKey::dir_bound(module, profile));
            dependencies.require(ArtifactKey::dir_imported(module, profile));
            dependencies.require(ArtifactKey::dir_resolved(module, profile));
            dependencies.require(ArtifactKey::dir_expanded(module, profile));
            dependencies.require(ArtifactKey::dir_exported(module, profile));
            dependencies.require(ArtifactKey::dir_declared(module, profile));
            dependencies.require(ArtifactKey::dir_checked(module, profile));
            dependencies.require(ArtifactKey::dir_materialized(module, profile));
        }

        Ok(())
    }

    /// Require verified MIR for every reachable code module.
    pub(super) fn require_mir_modules(
        &self,
        revision: Revision,
        modules: &[ModuleId],
        profile: ProfileId,
        target: TargetId,
        dependencies: &mut ArtifactDependencySet,
    ) -> Result<(), ProviderError> {
        for module in modules.iter().copied() {
            if self.module(revision, module)?.is_code() {
                dependencies.require(ArtifactKey::dir_declared(module, profile));
                dependencies.require(ArtifactKey::dir_checked(module, profile));
                dependencies.require(ArtifactKey::dir_materialized(module, profile));
                dependencies.require(ArtifactKey::mir_lowered(module, profile, target));
                dependencies.require(ArtifactKey::mir_verified(module, profile, target));
            }
        }

        Ok(())
    }
}
