use destack_source::ModuleId;
use destack_workspace::{ArtifactKey, ProfileId};

use crate::analyze::DirReadBoundary;
use crate::timing::tags;
use crate::{
    AnalyzeError, BuildKey, BuildProduct, BuildRequirementError, Compiler, ElaborateError,
    ElaborateResult, ResolveError,
};
use destack_dir::AnchoredGlobalNodeId;

impl Compiler {
    /// Map one build requirement failure into an elaborate error.
    pub(crate) fn elaborate_error_from_requirement(
        &self,
        error: BuildRequirementError,
    ) -> ElaborateError {
        match error {
            BuildRequirementError::NotReady { requirement } => {
                ElaborateError::Yield { requirement }
            }
            BuildRequirementError::Failed { requirement } => {
                ElaborateError::UnsatisfiedRequirement { requirement }
            }
        }
    }

    /// Map one resolve error into an elaborate error at one origin node.
    pub(crate) fn elaborate_error_from_resolve(
        &self,
        error: ResolveError,
        node: AnchoredGlobalNodeId,
    ) -> ElaborateError {
        match error {
            ResolveError::Yield { requirement } => ElaborateError::Yield { requirement },
            ResolveError::UnsatisfiedRequirement { requirement } => {
                ElaborateError::UnsatisfiedRequirement { requirement }
            }
            ResolveError::Skipped { reason } => ElaborateError::Skipped { reason },
            _ => ElaborateError::UnsupportedConstruct { node },
        }
    }

    /// Map one analyze error into an elaborate error at one origin node.
    pub(crate) fn elaborate_error_from_analyze(
        &self,
        error: AnalyzeError,
        node: AnchoredGlobalNodeId,
    ) -> ElaborateError {
        match error {
            AnalyzeError::Yield { requirement } => ElaborateError::Yield { requirement },
            AnalyzeError::UnsatisfiedRequirement { requirement } => {
                ElaborateError::UnsatisfiedRequirement { requirement }
            }
            AnalyzeError::Skipped { reason } => ElaborateError::Skipped { reason },
            _ => ElaborateError::UnsupportedConstruct { node },
        }
    }

    /// Build elaborated DIR for one module.
    pub fn process_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> ElaborateResult<BuildProduct> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<ElaborateError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;
        let dir_data = self
            .require_artifact_dir_for_boundary(module, profile, DirReadBoundary::Analyzed)
            .map_err(|error| self.elaborate_error_from_requirement(error))?;
        let (_, dir) = self.with_private_transient_artifact_dir(
            module,
            profile,
            DirReadBoundary::Analyzed,
            dir_data,
            |dir| -> ElaborateResult<()> {
                let module = self.program.modules.get(module);
                let module = module.read();

                let _timing = self.timing_scope(tags::ELABORATE_MODULE_TRANSFORM);
                self.elaborate_module_transform(&module, profile, &dir)?;

                let _timing = self.timing_scope(tags::ELABORATE_MODULE_REIFY);
                self.elaborate_module_reify(&module, profile, &dir)?;

                Ok(())
            },
        )?;

        let is_code_module = self.is_code_module(module);

        if is_code_module {
            self.stats.record_elaborate();
        }

        Ok(BuildProduct::Dir(dir.to_data()))
    }

    /// Ensure elaborated DIR exists for a module.
    pub fn require_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirElaborated {
            module,
            profile,
        }))
    }
}
