use destack_artifact::{ArtifactKey, DirElaborated};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::timing::tags;
use crate::{
    AnalyzeError, ArtifactRequirementError, Compiler, ElaborateError, ElaborateResult, ResolveError,
};
use destack_dir::AnchoredGlobalNodeId;

impl Compiler {
    /// Map one build requirement failure into an elaborate error.
    pub(crate) fn elaborate_error_from_requirement(
        &self,
        error: ArtifactRequirementError,
    ) -> ElaborateError {
        match error {
            ArtifactRequirementError::NotReady { requirement } => {
                ElaborateError::Yield { requirement }
            }
            ArtifactRequirementError::Failed { requirement } => {
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
            ResolveError::Skipped => ElaborateError::Skipped,
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
            AnalyzeError::Skipped => ElaborateError::Skipped,
            _ => ElaborateError::UnsupportedConstruct { node },
        }
    }

    /// Build elaborated DIR for one module.
    pub fn process_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> ElaborateResult<()> {
        let module_version = self.module_version(module);
        let profile_version = self.profile_version(profile);
        self.ensure_module_profile_matches::<ElaborateError>(
            module,
            module_version,
            profile,
            profile_version,
        )?;

        // reuse a persisted elaborated dir when it is still valid
        let artifact_key = ArtifactKey::DirElaborated { module, profile };
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_dir_elaborated_image(module, module_version, profile)
            })
            .is_some()
        {
            return Ok(());
        }

        let analyzed = self
            .require_artifact_dir_analyzed(module, profile)
            .map_err(|error| self.elaborate_error_from_requirement(error))?;
        let module_ref = self.program.modules.get(module);
        let module_ref = module_ref.as_ref();
        let mut tree = analyzed.tree.as_ref().clone();
        let mut symbols = analyzed.symbols.as_ref().clone();
        let mut types = analyzed.types.as_ref().clone();

        let _timing = self.timing_scope(tags::ELABORATE_MODULE_TRANSFORM);
        self.elaborate_module_transform(module_ref, profile, &mut tree, &mut symbols, &mut types)?;

        let _timing = self.timing_scope(tags::ELABORATE_MODULE_REIFY);
        self.elaborate_module_reify(module_ref, profile, &mut tree, &mut symbols, &mut types)?;

        let is_code_module = self.is_code_module(module);

        if is_code_module {
            self.stats.record_elaborate();
        }

        // publish the elaborated artifact with transformed runtime tables
        let payload = DirElaborated::from_analyzed_with(analyzed.as_ref(), tree, symbols, types);

        self.artifacts
            .publish(artifact_key.clone(), payload.clone());
        self.store_artifact(&artifact_key, &payload, |compiler, payload| {
            compiler.store_dir_elaborated_image(module, profile, payload)
        });

        Ok(())
    }

    /// Ensure elaborated DIR exists for a module.
    pub fn require_dir_elaborated(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::dir_elaborated(module, profile))
    }
}
