use destack_source::ModuleId;

use crate::check::{CheckState, FlowBranch, FlowCheckpoint};

impl CheckState<'_> {
    /// Mark the current flow position for branch rollback.
    pub(in crate::check) fn checkpoint_flow(&self, module: ModuleId) -> FlowCheckpoint {
        self.flow(module).checkpoint()
    }

    /// Collect the flow changes since one checkpoint.
    pub(in crate::check) fn collect_flow_branch(
        &self,
        module: ModuleId,
        checkpoint: FlowCheckpoint,
    ) -> FlowBranch {
        self.flow(module).branch(checkpoint)
    }

    /// Restore current flow facts to one checkpoint.
    pub(in crate::check) fn restore_flow(&mut self, module: ModuleId, checkpoint: FlowCheckpoint) {
        self.flow_mut(module).restore(checkpoint);
    }

    /// Apply one completed branch.
    pub(in crate::check) fn apply_flow_branch(
        &mut self,
        module: ModuleId,
        checkpoint: FlowCheckpoint,
        branch: &FlowBranch,
    ) {
        self.flow_mut(module).apply_branch(checkpoint, branch);
    }

    /// Merge two completed flow branches.
    pub(in crate::check) fn merge_flow_branches(
        &mut self,
        module: ModuleId,
        checkpoint: FlowCheckpoint,
        left: &FlowBranch,
        right: &FlowBranch,
    ) {
        self.flow_mut(module)
            .merge_branches(checkpoint, left, right);
    }

    /// Merge the facts common to all completed flow branches.
    pub(in crate::check) fn merge_flow_branches_from(
        &mut self,
        module: ModuleId,
        checkpoint: FlowCheckpoint,
        branches: &[FlowBranch],
    ) {
        let Some(first) = branches.first() else {
            self.restore_flow(module, checkpoint);

            return;
        };

        self.apply_flow_branch(module, checkpoint, first);

        for branch in &branches[1..] {
            let current = self.collect_flow_branch(module, checkpoint);

            self.merge_flow_branches(module, checkpoint, &current, branch);
        }
    }
}
