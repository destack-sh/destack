use crate::analyze::common::{CommitContext, ModuleContext};
use crate::{AnalyzeResult, Compiler};
use destack_dir::InferTable;
use destack_workspace::{Module, ModuleDir, ProfileId};

impl Compiler {
    /// Commit solved infer table outputs for one module after solve convergence.
    pub(super) fn commit_module_solved_infer_table(
        &self,
        dir: &ModuleDir,
        infer: &mut InferTable,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // commit solved infer state in one deterministic pass
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut types = dir.types.write();
        let options = self.analyze_context_options_for_module(module.id);
        let module = ModuleContext::new(module, profile, &tree, &symbols, &options);
        let mut ctx = CommitContext::with_dir(module, dir, &mut types);

        // commit direct type writes that depend on solved type substitutions
        self.commit_provisional_resolutions_and_instances(&mut ctx, infer);
        self.commit_infer_expression_overlays(infer, &mut *ctx.types);
        self.discharge_instance_commit_obligations(infer, &mut *ctx.types)?;
        let actions = self.collect_binding_value_commit_actions(&mut ctx, infer);
        self.apply_binding_value_commit_actions(actions, &mut ctx);
        self.materialize_committed_symbol_type_surfaces(&mut ctx)?;

        Ok(())
    }
}
