use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, DominatorTree, MemoryAccess, MemoryAccessEffect, MemoryAccessLocation,
    MemorySSA,
};
use crate::optimize::common::{
    clone_instruction_metadata, instruction_has_atomic_ordering, instruction_is_borrow_address,
    instruction_is_read_only_access, instruction_is_speculatable, instruction_map,
    terminator_used_values,
};

/// Guard branch metadata for loop headers.
#[derive(Debug, Clone)]
pub struct LoopGuardBranch {
    /// Exit block outside the loop.
    pub exit_block: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to the exit block.
    pub exit_arguments: Vec<mir::Value>,
    /// Arguments passed to the in loop target.
    pub in_loop_arguments: Vec<mir::Value>,
    /// True when the in loop edge is the then branch.
    pub in_loop_is_then: bool,
}

/// Policy for collecting loop memory effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopEffectPolicy {
    /// Accept only read effects from load like instructions.
    ReadOnly,
    /// Accept both reads and writes.
    ReadWrite,
}

/// Resolve the guard branch for a loop header.
pub fn loop_guard_branch(
    header: mir::LocalNodeId<mir::Block>,
    in_loop: mir::LocalNodeId<mir::Block>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::Tree,
) -> Option<LoopGuardBranch> {
    // read the header terminator
    let header_block = tree.get(header);
    let header_terminator = tree.get(header_block.terminator);
    let mir::Terminator::Branch {
        then_target,
        else_target,
        ..
    } = header_terminator
    else {
        return None;
    };

    let then_block = then_target.block.block()?;
    let else_block = else_target.block.block()?;
    let then_arguments: Vec<_> = then_target
        .arguments
        .iter()
        .copied()
        .map(|argument| argument.value())
        .collect::<Option<_>>()?;
    let else_arguments: Vec<_> = else_target
        .arguments
        .iter()
        .copied()
        .map(|argument| argument.value())
        .collect::<Option<_>>()?;

    // reject non loop in loop target
    if !loop_blocks.contains(&in_loop) {
        return None;
    }

    // handle then edge targeting the loop
    if then_block == in_loop {
        // reject exits that stay inside the loop
        if loop_blocks.contains(&else_block) {
            return None;
        }

        // record the exit and in loop arguments
        return Some(LoopGuardBranch {
            exit_block: else_block,
            exit_arguments: else_arguments,
            in_loop_arguments: then_arguments,
            in_loop_is_then: true,
        });
    }

    // handle else edge targeting the loop
    if else_block == in_loop {
        // reject exits that stay inside the loop
        if loop_blocks.contains(&then_block) {
            return None;
        }

        // record the exit and in loop arguments
        return Some(LoopGuardBranch {
            exit_block: then_block,
            exit_arguments: then_arguments,
            in_loop_arguments: else_arguments,
            in_loop_is_then: false,
        });
    }

    None
}

/// Return true when all instructions in a block are speculatable and read free.
pub fn block_is_speculatable_no_reads(
    block: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> bool {
    // scan instructions in the block
    let block = tree.get(block);
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);

        // reject non speculatable instructions
        if !instruction_is_speculatable(instruction, tree) {
            return false;
        }

        // reject read only memory accesses
        if instruction_is_read_only_access(instruction_id, memory_ssa) {
            return false;
        }
    }

    true
}

/// Find a loop preheader and its header arguments.
pub fn loop_preheader(
    header: mir::LocalNodeId<mir::Block>,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::Tree,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    // collect outside predecessors
    let mut outside_preds: Vec<_> = cfg
        .predecessors(header)
        .iter()
        .copied()
        .filter(|pred| !loop_blocks.contains(pred))
        .collect();

    // require a unique preheader
    if outside_preds.len() != 1 {
        return None;
    }

    // resolve the preheader
    let preheader = outside_preds.pop()?;

    // ensure the preheader dominates the header
    if !domtree.dominates(preheader, header) {
        return None;
    }

    // read the preheader terminator
    let preheader_block = tree.get(preheader);
    let preheader_terminator = tree.get(preheader_block.terminator);
    let arguments = match preheader_terminator {
        mir::Terminator::Jump { target } if target.block.block()? == header => target
            .arguments
            .iter()
            .copied()
            .map(|argument| argument.value())
            .collect::<Option<_>>()?,
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Collect control instructions in a latch block.
pub fn control_instructions_for_latch(
    header: mir::LocalNodeId<mir::Block>,
    latch: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
) -> HashSet<mir::LocalNodeId<mir::Instruction>> {
    // collect control values from header and latch arguments
    let mut control_values = HashSet::new();
    let header_block = tree.get(header);
    for instruction_id in &header_block.instructions {
        let instruction = tree.get(*instruction_id);
        control_values.extend(
            instruction
                .uses()
                .into_iter()
                .filter_map(|value| value.value()),
        );
    }
    let header_terminator = tree.get(header_block.terminator);
    control_values.extend(terminator_used_values(header_terminator));

    let latch_block = tree.get(latch);
    let latch_terminator = tree.get(latch_block.terminator);
    if let mir::Terminator::Jump { target } = latch_terminator {
        control_values.extend(
            target
                .arguments
                .iter()
                .copied()
                .filter_map(|argument| argument.value()),
        );
    }

    // walk backward from control values to latch definitions
    let mut control_instructions = HashSet::new();
    let mut worklist: VecDeque<_> = control_values.into_iter().collect();
    while let Some(value) = worklist.pop_front() {
        let Some(definition) = definitions.get(&value) else {
            continue;
        };

        if instruction_blocks.get(definition) != Some(&latch) {
            continue;
        }

        if !control_instructions.insert(*definition) {
            continue;
        }

        let instruction = tree.get(*definition);
        worklist.extend(
            instruction
                .uses()
                .into_iter()
                .filter_map(|value| value.value()),
        );
    }

    control_instructions
}

/// Collect loop memory effects across all loop blocks.
pub fn collect_loop_effects(
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
    policy: LoopEffectPolicy,
) -> Option<Vec<MemoryAccessEffect>> {
    // gather memory effects
    let mut effects = Vec::new();
    for block_id in loop_blocks {
        // scan instructions in the loop block
        let block = tree.get(*block_id);
        for instruction_id in &block.instructions {
            // reject ordered accesses
            if instruction_has_atomic_ordering(tree, *instruction_id) {
                return None;
            }

            // enforce read only instruction requirements
            if matches!(policy, LoopEffectPolicy::ReadOnly) {
                let instruction = tree.get(*instruction_id);
                if instruction_is_speculatable(instruction, tree)
                    || instruction_is_borrow_address(instruction)
                {
                    continue;
                }

                if !matches!(instruction, mir::Instruction::Load { .. }) {
                    return None;
                }
            }

            // resolve memory accesses for the instruction
            let Some(accesses) = memory_ssa.accesses_for_instruction(*instruction_id) else {
                continue;
            };

            // record each valid memory effect
            for access_id in accesses {
                let effect = match memory_ssa.access(*access_id) {
                    MemoryAccess::Def(def_access) => def_access.effect.clone(),
                    MemoryAccess::Use(use_access) => use_access.effect.clone(),
                    _ => continue,
                };

                // reject volatile or barrier effects
                if effect.is_volatile || effect.is_barrier {
                    return None;
                }

                // reject unknown locations
                if effect.location == MemoryAccessLocation::Unknown {
                    return None;
                }

                // reject writes in read only mode
                if matches!(policy, LoopEffectPolicy::ReadOnly) && effect.writes {
                    return None;
                }

                // record the effect
                effects.push(effect);
            }
        }
    }

    Some(effects)
}

/// Clone all blocks in a loop, creating fresh block and value ids.
#[allow(clippy::type_complexity)]
pub fn clone_loop_blocks(
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
) -> (
    HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    HashMap<mir::Value, mir::Value>,
) {
    // clone loop blocks and values
    let (block_map, value_map, _) = clone_loop_blocks_internal(loop_blocks, function, tree);
    (block_map, value_map)
}

/// Clone all blocks in a loop, returning instruction id mappings.
#[allow(clippy::type_complexity)]
pub fn clone_loop_blocks_with_instructions(
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
) -> (
    HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    HashMap<mir::Value, mir::Value>,
    HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Instruction>>,
) {
    // clone loop blocks and values
    clone_loop_blocks_internal(loop_blocks, function, tree)
}

/// Clone loop blocks and return block, value, and instruction maps.
#[allow(clippy::type_complexity)]
fn clone_loop_blocks_internal(
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
) -> (
    HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
    HashMap<mir::Value, mir::Value>,
    HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Instruction>>,
) {
    // initialize clone maps
    let mut block_map = HashMap::new();
    let mut value_map = HashMap::new();
    let mut instruction_id_map = HashMap::new();

    // sort blocks for deterministic insertion
    let mut sorted_blocks: Vec<_> = loop_blocks.iter().copied().collect();
    sorted_blocks.sort();

    // allocate cloned blocks and values
    for block_id in &sorted_blocks {
        // read the original block
        let original = tree.get(*block_id);

        // build new block parameters and value mapping
        let new_params: Vec<mir::Parameter> = original
            .parameters
            .iter()
            .map(|param| match (param.value.value(), param.ty.ty()) {
                (Some(value), Some(ty)) => {
                    let new_value = function.next_typed_value(ty);
                    value_map.insert(value, new_value);

                    mir::Parameter {
                        value: new_value.into(),
                        ty: ty.into(),
                    }
                }
                _ => *param,
            })
            .collect();

        // allocate new values for instruction destinations
        for &instruction_id in &original.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination().and_then(|value| value.value()) {
                let new_value = function.next_typed_value_like(destination);
                value_map.insert(destination, new_value);
            }
        }

        let new_block = mir::Block {
            name: None,
            parameters: new_params,
            instructions: Vec::new(),
            terminator: original.terminator,
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
            clone_instruction_metadata(tree, instruction_id, new_instruction_id, &value_map);
            instruction_id_map.insert(instruction_id, new_instruction_id);
            new_instructions.push(new_instruction_id);
        }

        // attach cloned instructions to the new block
        let mut new_block = tree.get(new_block_id).clone();
        new_block.instructions = new_instructions;
        tree.replace(new_block_id, new_block);
    }

    (block_map, value_map, instruction_id_map)
}
