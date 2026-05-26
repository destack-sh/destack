use crate::check::{CheckModuleState, FlowBranch, FlowCheckpoint};

impl CheckModuleState {
    /// Mark the current flow position for branch rollback.
    pub(in crate::check) fn checkpoint_flow(&self) -> FlowCheckpoint {
        self.work.flow.checkpoint()
    }

    /// Collect the flow changes since one checkpoint.
    pub(in crate::check) fn collect_flow_branch(&self, checkpoint: FlowCheckpoint) -> FlowBranch {
        self.work.flow.branch(checkpoint)
    }

    /// Restore current flow facts to one checkpoint.
    pub(in crate::check) fn restore_flow(&mut self, checkpoint: FlowCheckpoint) {
        self.work.flow.restore(checkpoint);
    }

    /// Apply one completed branch.
    pub(in crate::check) fn apply_flow_branch(
        &mut self,
        checkpoint: FlowCheckpoint,
        branch: &FlowBranch,
    ) {
        self.work.flow.apply_branch(checkpoint, branch);
    }

    /// Merge two completed flow branches.
    pub(in crate::check) fn merge_flow_branches(
        &mut self,
        checkpoint: FlowCheckpoint,
        left: &FlowBranch,
        right: &FlowBranch,
    ) {
        self.work.flow.merge_branches(checkpoint, left, right);
    }

    /// Merge the facts common to all completed flow branches.
    pub(in crate::check) fn merge_flow_branches_from(
        &mut self,
        checkpoint: FlowCheckpoint,
        branches: &[FlowBranch],
    ) {
        let Some(first) = branches.first() else {
            self.restore_flow(checkpoint);

            return;
        };

        self.apply_flow_branch(checkpoint, first);

        for branch in &branches[1..] {
            let current = self.collect_flow_branch(checkpoint);

            self.merge_flow_branches(checkpoint, &current, branch);
        }
    }
}
