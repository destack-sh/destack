use crate::declare_mir_pass;
use destack_mir as mir;

use crate::verify::VerifyState;

use super::solve::OwnershipSolver;

declare_mir_pass! {
    /// Verify MIR ownership rules.
    #[pass(id = "ownership-check")]
    pub(crate) OwnershipCheck,
    "Verify ownership rules"
}

impl OwnershipCheck {
    /// Verify ownership rules for one MIR tree.
    pub(crate) fn run(&self, tree: &mut mir::Tree, context: &mut VerifyState<'_>) {
        OwnershipSolver::new(tree, context).run();
    }
}
