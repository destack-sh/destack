use std::collections::VecDeque;

use crate::optimize::declare_pass;
use destack_core::{FxIndexMap, FxIndexSet};
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasTable, EdgeSplitPolicy, MemoryAccessId, MemoryNode, MemoryTable, Mutation,
    ensure_edge_block,
};

declare_pass! {
    /// Sink stores down to the successors that use them.
    ///
    /// ```mir
    /// function before(v0: boolean): int32 {
    ///     local l0: int32
    /// b0(v0: boolean):
    ///     v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    ///     v2: int32 = 7
    ///     store v1, v2
    ///     branch v0 => b1 | b2
    /// b1:
    ///     v3: int32 = load v1
    ///     return v3
    /// b2:
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: boolean): int32 {
    ///     local l0: int32
    /// b0(v0: boolean):
    ///     v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    ///     v2: int32 = 7
    ///     branch v0 => b1 | b2
    /// b1:
    ///     store v1, v2
    ///     v3: int32 = load v1
    ///     return v3
    /// b2:
    ///     return v2
    /// }
    /// ```
    #[pass(id = "sink-stores")]
    pub SinkStores,
    "Sink stores to the edges that require them"
}

impl FunctionPass for SinkStores {
    /// Run store sinking on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        _ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &mut optimized.accesses;
        let effects = &optimized.effects;

        // skip imported functions
        if function.entry().is_none() {
            return Mutation::NONE;
        }

        // run store sinking
        let changed = run_sink_stores(function, tree, accesses, effects, analyses);

        // report what this pass changed
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Kind of store candidate.
#[derive(Clone, Copy)]
enum StoreKind {
    /// Store to a pointer.
    Store,
    /// Store to a local.
    LocalSet,
}

/// Store candidate for sinking.
#[derive(Clone, Copy)]
struct StoreCandidate {
    /// Block containing the store.
    block: mir::LocalNodeId<mir::Block>,
    /// Store instruction id.
    instruction: mir::LocalNodeId<mir::Instruction>,
    /// Store pointer when applicable.
    pointer: Option<mir::Value>,
    /// Local target when applicable.
    local: Option<mir::LocalNodeId<mir::Local>>,
    /// Stored value.
    value: mir::Value,
    /// Memory SSA access id.
    access: MemoryAccessId,
    /// Store kind.
    kind: StoreKind,
}

/// Run store sinking and return true when changes are made.
fn run_sink_stores(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    effects: &mir::EffectTable,
    analyses: &mut mir::FunctionCache,
) -> bool {
    // gather analyses
    let cfg = analyses.control(function, tree).clone();
    let memory = analyses.memory(function, tree, accesses, effects);
    let alias = analyses.alias(function, tree).clone();

    // collect store candidates
    let candidates = collect_store_candidates(function, tree, accesses, memory.as_ref(), &alias);
    if candidates.is_empty() {
        return false;
    }

    // map clobbering defs to the blocks that read from them
    let use_blocks_by_def = collect_use_blocks_by_def(function, tree, memory.as_ref(), &alias);

    // track modifications
    let mut edge_blocks: FxIndexMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    > = FxIndexMap::default();
    let mut to_remove = FxIndexSet::default();
    let mut changed = false;

    // evaluate candidates for sinking
    for candidate in candidates {
        let block = tree.get(candidate.block);
        let terminator = tree.get(block.terminator);
        let successors = terminator.successors(tree);

        // require multiple successors
        if successors.len() < 2 {
            continue;
        }

        // locate blocks that use this store
        let use_blocks = match use_blocks_by_def.get(&candidate.access) {
            Some(blocks) if !blocks.is_empty() => blocks,
            _ => continue,
        };

        // determine which successors need the store
        let mut needed = Vec::new();
        for successor in &successors {
            let successor = *successor;

            if successor_reaches_use(tree, successor, use_blocks) {
                needed.push(successor);
            }
        }

        // skip when the store is needed on every edge
        if needed.len() == successors.len() {
            continue;
        }

        // insert stores on needed edges
        for successor in &needed {
            let insertion_block = ensure_edge_block(
                candidate.block,
                *successor,
                function,
                tree,
                &cfg,
                &mut edge_blocks,
                EdgeSplitPolicy::PredecessorMultiSuccessor,
                &mut changed,
            );

            let store_id = insert_store_for_candidate(function, tree, insertion_block, &candidate);
            clone_store_accesses(accesses, candidate.instruction, store_id, candidate.pointer);
        }

        // remove the original store
        to_remove.insert(candidate.instruction);
    }

    // exit when no instructions were removed
    if to_remove.is_empty() {
        return false;
    }

    // remove sunk stores
    for block_id in function.blocks().to_vec() {
        let mut instructions = tree.get(block_id).instructions.clone();
        instructions.retain(|id| !to_remove.contains(id));
        function.replace_block_instructions(block_id, instructions, tree);
    }

    // drop memory accesses for removed stores
    for instruction_id in &to_remove {
        accesses.remove(*instruction_id);
    }

    true
}

/// Collect store candidates for sinking.
fn collect_store_candidates(
    function: &mir::Function,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    memory: &MemoryTable,
    alias: &AliasTable,
) -> Vec<StoreCandidate> {
    // scan blocks for store candidates
    let mut candidates = Vec::new();

    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);

        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            // select store instructions
            let (kind, pointer, local, value) = match tree.get(instruction_id) {
                mir::Instruction::Store { pointer, value } => {
                    let pointer = *pointer;
                    let value = *value;

                    (StoreKind::Store, Some(pointer), None, value)
                }
                mir::Instruction::LocalSet { local, value } => {
                    let value = *value;

                    (StoreKind::LocalSet, None, Some(*local), value)
                }
                _ => continue,
            };

            // read the MemoryTable access
            let access_id = match memory.instruction_access(instruction_id) {
                Some(access_id) => access_id,
                None => continue,
            };

            // require the access to define memory
            let MemoryNode::Def(def_access) = memory.access(access_id) else {
                continue;
            };

            // require a write effect
            if !def_access.effect.writes {
                continue;
            }

            // skip ordered stores
            if accesses.is_ordered(instruction_id, tree) {
                continue;
            }

            // require a trackable effect
            if !def_access.effect.is_trackable() {
                continue;
            }

            // require a sinkable terminator
            let terminator = tree.get(block.terminator);
            if !terminator_allows_sinking(terminator) {
                continue;
            }

            // require the store to be at the block end
            if !store_can_sink_from_block(index, block) {
                continue;
            }

            // require frame-owned storage
            let region = alias.resolve(&def_access.effect.region);
            if !region.is_frame_storage() {
                continue;
            }

            // record the candidate
            candidates.push(StoreCandidate {
                block: block_id,
                instruction: instruction_id,
                pointer,
                local,
                value,
                access: access_id,
                kind,
            });
        }
    }

    candidates
}

/// Return true when a store can be sunk past the block terminator.
fn store_can_sink_from_block(index: usize, block: &mir::Block) -> bool {
    // require the store to be the last instruction
    if index + 1 != block.instructions.len() {
        return false;
    }

    true
}

/// Return true when a terminator is safe for sinking.
fn terminator_allows_sinking(terminator: &mir::Terminator) -> bool {
    matches!(
        terminator,
        mir::Terminator::Branch { .. } | mir::Terminator::Switch { .. }
    )
}

/// Collect blocks that read from each clobbering def.
fn collect_use_blocks_by_def(
    function: &mir::Function,
    tree: &mir::Tree,
    memory: &MemoryTable,
    alias: &AliasTable,
) -> FxIndexMap<MemoryAccessId, FxIndexSet<mir::LocalNodeId<mir::Block>>> {
    // collect clobbering use blocks
    let mut blocks_by_def: FxIndexMap<_, FxIndexSet<_>> = FxIndexMap::default();

    for &block_id in function.blocks() {
        let block = tree.get(block_id);

        for &instruction_id in &block.instructions {
            // skip instructions without memory accesses
            let Some(accesses) = memory.instruction_accesses(instruction_id) else {
                continue;
            };

            for &access_id in accesses {
                // keep the accesses that read memory
                let MemoryNode::Use(_) = memory.access(access_id) else {
                    continue;
                };

                // file the block under the def this read clobbers
                let clobber = memory.clobbering_use(access_id, alias);
                blocks_by_def.entry(clobber).or_default().insert(block_id);
            }
        }
    }

    blocks_by_def
}

/// Return true when a successor reaches any of the use blocks.
fn successor_reaches_use(
    tree: &mir::Tree,
    start: mir::LocalNodeId<mir::Block>,
    use_blocks: &FxIndexSet<mir::LocalNodeId<mir::Block>>,
) -> bool {
    // seed a breadth first walk at the successor
    let mut queue = VecDeque::new();
    let mut visited = FxIndexSet::default();

    queue.push_back(start);
    visited.insert(start);

    while let Some(block) = queue.pop_front() {
        // stop as soon as the walk lands on a use
        if use_blocks.contains(&block) {
            return true;
        }

        // queue every block this one reaches
        let block = tree.get(block);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors(tree) {
            if visited.insert(successor) {
                queue.push_back(successor);
            }
        }
    }

    false
}

/// Insert a store instruction for a candidate.
fn insert_store_for_candidate(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    candidate: &StoreCandidate,
) -> mir::LocalNodeId<mir::Instruction> {
    // build the new store instruction
    let instruction = match candidate.kind {
        StoreKind::Store => mir::Instruction::Store {
            pointer: candidate.pointer.expect("store pointer required"),
            value: candidate.value,
        },
        StoreKind::LocalSet => mir::Instruction::LocalSet {
            local: candidate.local.expect("local target required"),
            value: candidate.value,
        },
    };

    // insert the instruction in the edge block
    let instruction_id = tree.insert(instruction);
    let mut instructions = tree.get(block_id).instructions.clone();
    instructions.push(instruction_id);
    function.replace_block_instructions(block_id, instructions, tree);

    instruction_id
}

/// Clone the store's memory accesses onto a new instruction.
fn clone_store_accesses(
    accesses: &mut mir::AccessTable,
    source: mir::LocalNodeId<mir::Instruction>,
    destination: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
) {
    // skip when there are no accesses to clone
    let Some(entries) = accesses.get(source) else {
        return;
    };

    // point the cloned accesses at the sunk store's pointer
    let mut cloned = Vec::with_capacity(entries.len());
    for access in entries {
        let mut updated = access.clone();
        if let (Some(pointer), mir::MemoryTarget::Address(_)) = (pointer, updated.target) {
            updated.target = mir::MemoryTarget::Address(pointer);
        }

        cloned.push(updated);
    }

    accesses.insert(destination, cloned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Stores are sunk to the successor that reads them.
    #[test]
    fn test_sink_stores_to_single_successor() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v2: int32 = 7
    store v1, v2
    branch v0 => b1 | b2

b1:
    v3: int32 = load v1
    return v3

b2:
    return v2
}
"#;

        let expected = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v2: int32 = 7
    branch v0 => b1 | b2

b1:
    store v1, v2
    jump b1_1

b1_1:
    v3: int32 = load v1
    return v3

b2:
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SinkStores);
        test.assert_output(expected);
    }

    /// Stores needed on both edges are not sunk.
    #[test]
    fn test_sink_stores_skips_all_successors() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    v2: int32 = 7
    store v1, v2
    branch v0 => b1 | b2

b1:
    v3: int32 = load v1
    return v3

b2:
    v4: int32 = load v1
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SinkStores);
        test.assert_output(input);
    }

    /// Stores to escaping memory are not sunk.
    #[test]
    fn test_sink_stores_skips_escaping_store() {
        let input = r#"
function test(v0: boolean, v1: ref<int32, borrowed, 'static, mutable, static>): void {
entry(v0: boolean, v1: ref<int32, borrowed, 'static, mutable, static>):
    v2: int32 = 1
    store v1, v2
    branch v0 => b1 | b2

b1:
    return

b2:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SinkStores);
        test.assert_output(input);
    }
}
