use crate::analyze::common::CommitContext;
use crate::{AnalyzeResult, Compiler};
use destack_dir::{InferTable, NodeTree, SymbolTable, TypeTable};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Commit solved infer table outputs for one module after solve convergence.
    pub(super) fn commit_solved_infer_table(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        module: &Module,
        profile: ProfileId,
    ) -> AnalyzeResult<()> {
        // commit solved infer state in one deterministic pass
        let options = self.analyze_context_options_for_module(module.id);
        let mut ctx = CommitContext::new(module, profile, &options, tree, symbols, types);

        // commit direct type writes that depend on solved type substitutions
        self.commit_provisional_resolutions_and_instances(&mut ctx, infer);
        self.commit_infer_expression_overlays(infer, &mut *ctx.types);
        self.discharge_instance_commit_obligations(infer, &mut *ctx.types)?;
        self.commit_binding_value_types(&mut ctx, infer)?;
        self.materialize_committed_symbol_type_surfaces(&mut ctx)?;

        Ok(())
    }
}
