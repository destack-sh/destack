use crate::verify::VerifyState;
use destack_mir as mir;

use super::function::{FunctionBorrowContract, FunctionVerifyState};

/// Ownership contract solver for one MIR tree.
pub(super) struct OwnershipSolver<'a, 'b> {
    /// The MIR tree being verified.
    tree: &'a mut mir::Tree,
    /// Verification context.
    context: &'a mut VerifyState<'b>,
    /// Functions participating in ownership solving.
    functions: Vec<mir::LocalNodeId<mir::Function>>,
}

impl<'a, 'b> OwnershipSolver<'a, 'b> {
    /// Create an ownership solver.
    pub(super) fn new(tree: &'a mut mir::Tree, context: &'a mut VerifyState<'b>) -> Self {
        let functions = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        Self {
            tree,
            context,
            functions,
        }
    }

    /// Verify ownership and write solved contracts.
    pub(super) fn run(mut self) {
        self.rebuild_function_metadata();
        self.solve_contracts();
        self.replay_functions();
    }

    /// Rebuild MIR metadata used by ownership checking.
    fn rebuild_function_metadata(&mut self) {
        for function_id in &self.functions {
            self.tree
                .infer_and_set_function_return_lifetime(*function_id);
            self.tree.rebuild_function_places(*function_id);
        }
    }

    /// Solve transitive borrow contracts to a fixed point.
    fn solve_contracts(&mut self) {
        loop {
            let mut is_changed = false;

            for index in 0..self.functions.len() {
                let function_id = self.functions[index];
                let Some(contract) = self.infer_contract(function_id) else {
                    continue;
                };

                if self.write_contract(function_id, contract) {
                    is_changed = true;
                }
            }

            if !is_changed {
                break;
            }
        }
    }

    /// Replay functions once solved contracts are stable.
    fn replay_functions(&mut self) {
        for index in 0..self.functions.len() {
            let function_id = self.functions[index];
            let Some(contract) = self.check_function(function_id) else {
                continue;
            };

            self.write_contract(function_id, contract);
        }
    }

    /// Check one function without emitting diagnostics.
    fn infer_contract(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<FunctionBorrowContract> {
        if self.tree.get(function_id).entry.is_none() {
            return None;
        }

        let function = self.tree.get(function_id);

        Some(FunctionVerifyState::for_contract(function, self.tree, self.context).check())
    }

    /// Check one function and emit diagnostics.
    fn check_function(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<FunctionBorrowContract> {
        if self.tree.get(function_id).entry.is_none() {
            return None;
        }

        let function = self.tree.get(function_id);

        Some(FunctionVerifyState::new(function, self.tree, self.context).check())
    }

    /// Write one function borrow contract and return whether it changed.
    fn write_contract(
        &mut self,
        function_id: mir::LocalNodeId<mir::Function>,
        contract: FunctionBorrowContract,
    ) -> bool {
        let function = self.tree.get_mut(function_id);
        if function.borrow_obligations == contract.borrow_obligations
            && function.return_lifetime == contract.return_lifetime
        {
            return false;
        }

        function.borrow_obligations = contract.borrow_obligations;
        function.return_lifetime = contract.return_lifetime;

        true
    }
}
