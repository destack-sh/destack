use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use crate::optimize::common::instruction_map;

/// Clone all blocks in a loop, creating fresh block and value ids.
pub fn clone_loop_blocks(
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
) -> (
    HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    HashMap<mir::Value, mir::Value>,
) {
    // initialize clone maps
    let mut block_map = HashMap::new();
    let mut value_map = HashMap::new();

    // sort blocks for deterministic insertion
    let mut sorted_blocks: Vec<_> = loop_blocks.iter().copied().collect();
    sorted_blocks.sort();

    // allocate cloned blocks and values
    for block_id in &sorted_blocks {
        let original = tree.get(*block_id);

        // build new block parameters and value mapping
        let new_params: Vec<mir::TypedValue> = original
            .parameters
            .iter()
            .map(|param| {
                let new_value = function.next_value();
                value_map.insert(param.value, new_value);
                mir::TypedValue {
                    value: new_value,
                    ty: param.ty,
                }
            })
            .collect();

        // allocate new values for instruction destinations
        for &instruction_id in &original.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                let new_value = function.next_value();
                value_map.insert(destination, new_value);
            }
        }

        let new_block = mir::Block {
            parameters: new_params,
            instructions: Vec::new(),
            terminator: original.terminator.clone(),
        };
        let new_block_id = tree.insert(new_block);
        block_map.insert(*block_id, new_block_id);
    }

    // clone instructions into the new blocks
    for block_id in &sorted_blocks {
        let new_block_id = block_map[block_id];
        let instruction_ids: Vec<_> = tree.get(*block_id).instructions.clone();
        let mut new_instructions = Vec::new();
        for instruction_id in instruction_ids {
            let original_instruction = tree.get(instruction_id).clone();
            let new_instruction = instruction_map(&original_instruction, &value_map, tree);
            let new_instruction_id = tree.insert(new_instruction);
            new_instructions.push(new_instruction_id);
        }

        // attach cloned instructions to the new block
        let mut new_block = tree.get(new_block_id).clone();
        new_block.instructions = new_instructions;
        tree.replace(new_block_id, new_block);
    }

    (block_map, value_map)
}
