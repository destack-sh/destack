use crate::verify::VerifyState;
use destack_mir as mir;

use super::function::FunctionVerifyState;

impl VerifyState<'_> {
    /// Verify ownership for the current MIR tree.
    pub(in crate::verify) fn check_ownership(&mut self) {
        let tree = self.tree;

        // verify every body-backed function
        for (_, function) in tree.iter_nodes::<mir::Function>() {
            if function.entry().is_none() {
                continue;
            }

            FunctionVerifyState::new(function, tree, self).check();
        }
    }
}
