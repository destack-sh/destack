use crate::verify::VerifyState;
use destack_mir as mir;

use super::function::FunctionVerifyState;

/// Ownership check for one MIR tree.
pub(crate) struct OwnershipCheck<'a, 'b> {
    /// The MIR tree being verified.
    tree: &'a mut mir::Tree,
    /// Verification state.
    state: &'a mut VerifyState<'b>,
    /// Functions participating in ownership checking.
    functions: Vec<mir::LocalNodeId<mir::Function>>,
}

impl<'a, 'b> OwnershipCheck<'a, 'b> {
    /// Create an ownership check.
    pub(crate) fn new(tree: &'a mut mir::Tree, state: &'a mut VerifyState<'b>) -> Self {
        let functions = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        Self {
            tree,
            state,
            functions,
        }
    }

    /// Verify ownership.
    pub(crate) fn run(mut self) {
        self.rebuild_function_metadata();
        self.check_functions();
    }

    /// Rebuild MIR metadata used by ownership checking.
    fn rebuild_function_metadata(&mut self) {
        for function_id in &self.functions {
            self.tree.rebuild_function_places(*function_id);
        }
    }

    /// Check every reachable function.
    fn check_functions(&mut self) {
        for index in 0..self.functions.len() {
            let function_id = self.functions[index];
            self.check_function(function_id);
        }
    }

    /// Check one function when it has a body.
    fn check_function(&mut self, function_id: mir::LocalNodeId<mir::Function>) {
        if self.tree.get(function_id).entry.is_none() {
            return;
        }

        let function = self.tree.get(function_id);
        FunctionVerifyState::new(function, self.tree, self.state).check();
    }
}
