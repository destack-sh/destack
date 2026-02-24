use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskDependencyError};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
    /// Ensure a module's infer constraints have been solved.
    pub fn require_analyze_module_solve(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleSolve { module, profile })
    }

    /// Phase 4: Solve infer constraints.
    pub(crate) fn analyze_module_solve(
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
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_SOLVE);

        // ensure infer preconditions are complete
        self.require_analyze_module_infer(module_id, profile)?;

        // skip non-code modules
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // declaration modules have no infer-table solve stage
        let module = self.program.modules.get(module_id);
        let module = module.read();
        if module.language_type.is_declaration() {
            return Ok(());
        }

        // load module tables and solve published infer constraints
        let options = self.analyze_context_options_for_module(module.id);
        self.with_infer_table_for_module_mut(module_id, profile, |infer| {
            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();
            let mut types = module.dir(profile).types.write();
            self.solve_infer_table(&module, profile, &symbols, infer, &mut types, &options);

            self.rewrite_inferred_type_overlays_for_instance_substitutions(
                &module, profile, &tree, &symbols, infer, &mut types,
            )?;

            self.discharge_projection_obligations_in_solve(&module, profile, infer, &mut types)?;
            self.discharge_missing_member_obligations_in_solve(
                &module, profile, infer, &mut types, &options,
            )?;
            self.discharge_relation_obligations_in_solve(
                &module, profile, &symbols, infer, &mut types, &options,
            )?;

            Ok::<(), AnalyzeError>(())
        })
        .ok_or_else(|| AnalyzeError::Internal {
            message: format!(
                "missing infer table for solve stage: module={module_id:?}, profile={profile:?}"
            ),
        })??;

        Ok(())
    }
}
