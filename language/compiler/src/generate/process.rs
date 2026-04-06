use crate::timing::tags;
use crate::{Compiler, CompilerContext, GenerateError, GenerateResult, RequirementError};

use destack_artifact::ArtifactKey;
use destack_source::{ModuleId, TargetId};
use destack_workspace::ProfileId;

impl Compiler {
    /// Build one module artifact.
    pub fn process_module_output(
        &self,
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
        context: &CompilerContext<'_>,
    ) -> GenerateResult<()> {
        let artifact_key = ArtifactKey::module_output(module, target);
        let _timing = self.timing_scope(tags::GENERATE_MODULE);
        self.generate_target_module_output(module, profile, &target, context)?;
        let artifact =
            self.module_output(module, &target)
                .ok_or_else(|| GenerateError::Internal {
                    module,
                    message: format!("missing module artifact for target '{target}'"),
                })?;
        context.store_artifact(&artifact_key, artifact.as_ref(), |_, _, _| Ok(()));
        self.stats.record_generate();

        Ok(())
    }
    /// Require one module artifact.
    pub fn require_module_output(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
        profile: ProfileId,
        target: &TargetId,
    ) -> Result<(), RequirementError> {
        let _ = profile;
        self.require_artifact(revision, ArtifactKey::module_output(module, *target))
    }
}
