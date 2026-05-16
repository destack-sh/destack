use crate::declare_mir_pass;
use destack_mir as mir;

use crate::verify::VerifyState;

use super::function::FunctionVerifyState;

declare_mir_pass! {
    /// Verify MIR ownership rules.
    #[pass(id = "ownership-check")]
    pub(crate) OwnershipCheck,
    "Verify ownership rules"
}

impl OwnershipCheck {
    /// Verify ownership rules for one MIR tree.
    pub(crate) fn run(&self, tree: &mut mir::Tree, context: &mut VerifyState<'_>) {
        // collect ids before rebuilding function metadata
        let functions = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        // rebuild MIR facts used by ownership checking
        for function_id in &functions {
            tree.infer_and_set_function_return_lifetime(*function_id);
            tree.rebuild_function_places(*function_id);
        }

        // solve transitive borrow obligations through the call graph
        loop {
            let mut is_changed = false;

            for function_id in &functions {
                if tree.get(*function_id).entry.is_none() {
                    continue;
                }

                let summary = {
                    let function = tree.get(*function_id);
                    FunctionVerifyState::new_silent(function, tree, context).check()
                };
                let function = tree.get_mut(*function_id);
                if function.borrow_obligations != summary.borrow_obligations
                    || function.return_lifetime != summary.return_lifetime
                {
                    function.borrow_obligations = summary.borrow_obligations;
                    function.return_lifetime = summary.return_lifetime;
                    is_changed = true;
                }
            }

            if !is_changed {
                break;
            }
        }

        // replay with fixed summaries and diagnostics enabled
        for function_id in functions {
            if tree.get(function_id).entry.is_none() {
                continue;
            }

            let summary = {
                let function = tree.get(function_id);
                FunctionVerifyState::new(function, tree, context).check()
            };
            let function = tree.get_mut(function_id);
            function.borrow_obligations = summary.borrow_obligations;
            function.return_lifetime = summary.return_lifetime;
        }
    }
}
