use std::collections::HashMap;

use destack_mir as mir;

use super::plan::{DropMarker, DropPlan, DropRelease};

/// Drop insertion for one MIR tree.
pub(crate) struct DropInsert<'a> {
    /// The MIR tree being rewritten.
    tree: &'a mut mir::Tree,
}

impl<'a> DropInsert<'a> {
    /// Create drop insertion for one MIR tree.
    pub(crate) fn new(tree: &'a mut mir::Tree) -> Self {
        Self { tree }
    }

    /// Insert explicit last use drops for verified owned values.
    pub(crate) fn run(&mut self) {
        let functions = self
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        // build and insert drops for each function
        for function_id in functions {
            let function = self.tree.get(function_id).clone();
            if function.entry.is_none() {
                continue;
            }

            let drops = DropPlan::build(&function, self.tree);
            self.insert_function_drops(drops);
        }
    }

    /// Insert drops into one function body.
    fn insert_function_drops(
        &mut self,
        drops_by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<DropMarker>>,
    ) {
        for (block_id, drops) in drops_by_block {
            let mut inserted = 0;
            for drop in drops {
                // release storage before the drop marker
                match drop.release {
                    DropRelease::None => {}
                    DropRelease::Free(value) => {
                        let instruction_id = self.tree.insert(mir::Instruction::Free {
                            value: value.into(),
                        });
                        let block = self.tree.get_mut(block_id);

                        block
                            .instructions
                            .insert(drop.index + inserted, instruction_id);
                        inserted += 1;
                    }
                }

                // record the lifetime end for later MIR passes
                let instruction_id = self
                    .tree
                    .insert(mir::Instruction::Drop { place: drop.place });
                let block = self.tree.get_mut(block_id);

                block
                    .instructions
                    .insert(drop.index + inserted, instruction_id);
                inserted += 1;
            }
        }
    }
}
