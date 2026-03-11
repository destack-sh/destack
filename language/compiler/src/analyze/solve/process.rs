use crate::analyze::common::InferContext;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

impl Compiler {
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

        // skip non-code modules
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // declaration modules have no solve-time infer table
        let module = self.program.modules.get(module_id);
        let module = module.read();
        if module.language_type.is_declaration() {
            return Ok(());
        }

        // load module ctx and solve published infer constraints
        let options = self.analyze_context_options_for_module(module.id);
        self.with_infer_table_for_module_mut(module_id, profile, |infer| {
            let tree = module.dir(profile).tree.read();
            let symbols = module.dir(profile).symbols.read();
            let mut types = module.dir(profile).types.write();
            let mut ctx = InferContext::new(
                &module, profile, &options, &tree, &symbols, &mut types, infer,
            );
            {
                let (mut ctx, infer) = ctx.split_type_context_and_infer();
                self.solve_infer_table(&mut ctx, infer);
            }
            {
                let (mut ctx, infer) = ctx.split_type_context_and_infer();
                self.rewrite_inferred_type_overlays_for_instance_substitutions(&mut ctx, infer)?;
            }
            {
                let (mut ctx, infer) = ctx.split_type_context_and_infer();
                self.discharge_projection_obligations_in_solve(&mut ctx, infer)?;
            }
            self.discharge_missing_member_obligations_in_solve(&mut ctx.reborrow())?;
            {
                let (mut ctx, infer) = ctx.split_type_context_and_infer();
                self.discharge_relation_obligations_in_solve(&mut ctx, infer)?;
            }

            Ok::<(), AnalyzeError>(())
        })
        .ok_or_else(|| AnalyzeError::Internal {
            message: format!(
                "missing infer table for solve: module={module_id:?}, profile={profile:?}"
            ),
        })??;

        Ok(())
    }
}
