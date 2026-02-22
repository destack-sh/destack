use crate::{AnalyzeOptions, AnalyzeResult, Compiler};
use destack_dir::{InferTable, NodeTree, SymbolTable, TypeTable};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Replay post solve obligations in deterministic ownership order.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn replay_post_solve_obligations(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        options: &AnalyzeOptions,
    ) -> AnalyzeResult<()> {
        // resolve associated projection obligations first
        self.discharge_associated_comptime_projection_obligations(module, profile, types, infer)?;

        // commit infer owned instance obligations
        self.discharge_instance_commit_obligations(infer, types)?;
        self.commit_instance_instantiated_inferred_types(
            module, profile, tree, symbols, infer, types,
        )?;

        // report deferred member diagnostics after commitments
        self.discharge_missing_member_obligations(module, profile, types, infer, options)?;

        // report deferred relation diagnostics last
        self.discharge_type_relation_obligations(module, profile, symbols, types, infer, options)?;

        Ok(())
    }
}
