use crate::timing::tags;
use crate::{Compiler, CompilerContext, ExecuteError, ExecuteResult, RequirementError};

use destack_artifact::ArtifactKey;
use destack_source::ModuleId;
use destack_workspace::ProfileId;

impl Compiler {
    /// Build patched DIR for one module.
    pub fn process_dir_patched(
        &self,
        module: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> ExecuteResult<()> {
        let revision = context.revision();
        let artifact_key = ArtifactKey::dir_patched(module, profile);
        let artifact_stamp = context.artifact_stamp(&artifact_key);

        // reuse a persisted patched dir when it is still valid
        if context
            .restore_cached_artifact(
                artifact_key,
                |compiler| {
                    compiler.load_dir_patched_image(revision, module, artifact_stamp, profile)
                },
                |store, version, payload| store.publish_dir_patched(version, payload),
            )
            .is_some()
        {
            return Ok(());
        }

        let _timing = self.timing_scope(tags::EXECUTE_MODULE_PATCH);
        self.require_intrinsic_environment(context.revision(), profile)
            .map_err(ExecuteError::from)?;
        let payload = self.execute_module_patch(module, profile, context)?;
        if context.is_code_module(module) {
            self.stats.record_execute();
        }

        context.publish_artifact(artifact_key, payload.clone(), |store, version, payload| {
            store.publish_dir_patched(version, payload)
        });
        context.store_artifact(
            &artifact_key,
            &payload,
            |compiler, artifact_stamp, payload| {
                compiler.store_dir_patched_image(revision, module, profile, artifact_stamp, payload)
            },
        );

        Ok(())
    }

    /// Ensure patched DIR exists for a module.
    pub fn require_dir_patched(
        &self,
        revision: destack_workspace::Revision,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::dir_patched(module, profile))
    }
}
