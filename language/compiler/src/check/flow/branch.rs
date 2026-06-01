use crate::check::{FlowBranch, FlowCheckpoint, WalkState};

impl WalkState<'_, '_> {
    /// Mark the current flow position for branch rollback.
    pub(in crate::check) fn checkpoint_flow(&self) -> FlowCheckpoint {
        self.flow().checkpoint()
    }

    /// Collect the flow changes since one checkpoint.
    pub(in crate::check) fn collect_flow_branch(&self, checkpoint: FlowCheckpoint) -> FlowBranch {
        self.flow().branch(checkpoint)
    }

    /// Restore current flow state to one checkpoint.
    pub(in crate::check) fn restore_flow(&mut self, checkpoint: FlowCheckpoint) {
        self.flow_mut().restore(checkpoint);
    }

    /// Restore one completed branch.
    pub(in crate::check) fn restore_flow_branch(
        &mut self,
        checkpoint: FlowCheckpoint,
        branch: &FlowBranch,
    ) {
        self.flow_mut().restore_branch(checkpoint, branch);
    }

    /// Merge two completed flow branches.
    pub(in crate::check) fn merge_flow_branches(
        &mut self,
        checkpoint: FlowCheckpoint,
        left: &FlowBranch,
        right: &FlowBranch,
    ) {
        self.flow_mut().merge_branches(checkpoint, left, right);
    }

    /// Merge the state common to all completed flow branches.
    pub(in crate::check) fn merge_flow_branches_from(
        &mut self,
        checkpoint: FlowCheckpoint,
        branches: &[FlowBranch],
    ) {
        // restore to base state when there are no branches
        let Some(first) = branches.first() else {
            self.restore_flow(checkpoint);

            return;
        };

        // seed merge with the first branch
        self.restore_flow_branch(checkpoint, first);

        // intersect each remaining branch into the current flow
        for branch in &branches[1..] {
            let current = self.collect_flow_branch(checkpoint);

            self.merge_flow_branches(checkpoint, &current, branch);
        }
    }
}
