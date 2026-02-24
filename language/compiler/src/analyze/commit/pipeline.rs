use crate::analyze::common::{CommitContext, ModuleContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Build one infer-table-missing internal error for commit stage paths.
    fn missing_commit_infer_table_error(
        &self,
        module_id: ModuleId,
        profile: ProfileId,
        context: &'static str,
    ) -> AnalyzeError {
        AnalyzeError::Internal {
            message: format!(
                "missing infer table for commit stage ({context}): module={module_id:?}, profile={profile:?}"
            ),
        }
    }

    /// Commit solved infer table outputs for one module after solve convergence.
    pub(super) fn commit_module_solved_infer_table(
        &self,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        let module_id = module.id;
        let dir = module.dir(profile);

        // commit solved infer state in one deterministic pass
        self.with_infer_table_for_module_mut(module_id, profile, |infer| {
            let tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let mut types = dir.types.write();
            self.commit_provisional_resolutions_and_instances(module, infer, &mut types);
            self.commit_infer_expression_overlays(infer, &mut types);
            self.discharge_instance_commit_obligations(infer, &mut types)?;

            // commit direct type writes that depend on solved type substitutions
            let options = self.analyze_context_options_for_module(module_id);
            let module_ctx = ModuleContext::new(module, profile, &tree, &symbols, &options);
            let mut commit_ctx = CommitContext::new(module_ctx, &mut types);
            let actions = self.collect_binding_value_commit_actions(&mut commit_ctx, infer);
            self.apply_binding_value_commit_actions(actions, &mut commit_ctx);
            Ok::<(), AnalyzeError>(())
        })
        .ok_or_else(|| self.missing_commit_infer_table_error(module_id, profile, "commit"))??;

        // clear infer table only after successful commit
        self.clear_infer_table_for_module(module_id, profile);
        Ok(())
    }
}
