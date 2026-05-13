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
        let functions: Vec<_> = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        // rebuild MIR places before checking each function
        for function_id in functions {
            tree.infer_and_set_function_return_lifetime(function_id);
            tree.rebuild_function_places(function_id);

            let function = tree.get(function_id);
            if function.entry.is_none() {
                continue;
            }

            FunctionVerifyState::new(function, tree, context).check();
        }
    }
}
