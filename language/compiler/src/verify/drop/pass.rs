use std::collections::HashMap;

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::verify::VerifyState;

use super::plan::{DropMarker, DropPlan, DropRelease};

declare_mir_pass! {
    /// Insert explicit last use drops for verified owned values.
    #[pass(id = "drop-insert")]
    pub(crate) DropInsert,
    "Insert last use drops"
}

impl DropInsert {
    /// Insert drops in one MIR tree.
    pub(crate) fn run(&self, tree: &mut mir::Tree, _state: &mut VerifyState<'_>) {
        // collect ids before mutating the tree
        let functions = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        // build drop plan for each function
        for function_id in functions {
            let function = tree.get(function_id).clone();
            if function.entry.is_none() {
                continue;
            }

            let drops = DropPlan::build(&function, tree);
            self.insert_function_drops(tree, drops);
        }
    }

    /// Insert drops into one function body.
    fn insert_function_drops(
        &self,
        tree: &mut mir::Tree,
        drops_by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<DropMarker>>,
    ) {
        for (block_id, drops) in drops_by_block {
            let mut inserted = 0;
            for drop in drops {
                // handle storage release before the marker
                match drop.release {
                    DropRelease::None => {}
                    DropRelease::Free(value) => {
                        let instruction_id = tree.insert(mir::Instruction::Free {
                            value: value.into(),
                        });
                        let block = tree.get_mut(block_id);

                        block
                            .instructions
                            .insert(drop.index + inserted, instruction_id);
                        inserted += 1;
                    }
                }

                // record the lifetime end for later MIR passes
                let instruction_id = tree.insert(mir::Instruction::Drop { place: drop.place });
                let block = tree.get_mut(block_id);

                block
                    .instructions
                    .insert(drop.index + inserted, instruction_id);
                inserted += 1;
            }
        }
    }
}
