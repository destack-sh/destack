use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskDependencyError};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Ensure a module's solved infer table outputs have been committed.
    pub fn require_analyze_module_commit(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleCommit { module, profile })
    }

    /// Phase 5: Commit solved infer table outputs and discharge obligations.
    pub(crate) fn analyze_module_commit(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_COMMIT);

        // ensure solve preconditions are complete
        self.require_analyze_module_solve(module_id, profile)?;

        // skip non-code modules
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // declaration modules have no infer-table commit stage
        let module = self.program.modules.get(module_id);
        let module = module.read();
        if module.language_type.is_declaration() {
            return Ok(());
        }

        self.commit_module_solved_infer_table(&module, profile)
    }
}
