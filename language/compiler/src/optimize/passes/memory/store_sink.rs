use std::collections::{HashMap, HashSet, VecDeque};

use crate::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, ControlFlowGraph, MemoryAccess, MemoryAccessId, MemorySSA,
};
use crate::optimize::common::{
    EdgeSplitPolicy, build_value_definition_map, collect_non_escaping_stack_allocs,
    effect_is_trackable, ensure_edge_block, instruction_has_atomic_ordering, stack_alloc_base,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Sink stores down to the successors that use them.
    ///
    /// When a store feeds memory uses only along a subset of outgoing edges,
    /// move the store to those edges and remove it from the predecessor block.
    ///
    /// ```mir
    /// function before(v0: boolean): int32 {
    /// b0(v0: boolean):
    ///     v1 = stack.alloc int32 -> ref<int32, raw, space(stack)>
    ///     v2 = 7int32
    ///     store v1, v2
    ///     branch v0, b1, b2
    /// b1:
    ///     v3 = load v1 -> int32
    ///     return v3
    /// b2:
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: boolean): int32 {
    /// b0(v0: boolean):
    ///     v1 = stack.alloc int32 -> ref<int32, raw, space(stack)>
    ///     v2 = 7int32
    ///     branch v0, b1, b2
    /// b1:
    ///     store v1, v2
    ///     v3 = load v1 -> int32
    ///     return v3
    /// b2:
    ///     return v2
    /// }
    /// ```
    #[pass(id = "store-sink", requires(call_effects, memory_access_metadata))]
    pub StoreSink,
    "Sink stores to the edges that require them"
}

impl FunctionPass for StoreSink {
    /// Run store sinking on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // run store sinking
        let changed = run_store_sink(function, tree, ctx);

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "StoreSink"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "store-sink"
    }
}

/// Kind of store candidate.
#[derive(Clone, Copy)]
enum StoreKind {
    /// Store to a pointer.
    Store,
    /// Store to a local slot.
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
fn run_store_sink(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // gather analyses
    let analyses = ctx.function_analyses(function, tree);
    let cfg = analyses.get::<ControlFlowGraph>().clone();
    let memory_ssa = analyses.get::<MemorySSA>();
    let alias = analyses.get::<AliasAnalysis>().clone();

    // build pointer definition info
    let definitions = build_value_definition_map(function, tree);
    let non_escaping_stack_allocs = collect_non_escaping_stack_allocs(function, tree, &definitions);

    // collect store candidates
    let candidates = collect_store_candidates(
        function,
        tree,
        memory_ssa.as_ref(),
        &non_escaping_stack_allocs,
        &definitions,
    );
    if candidates.is_empty() {
        return false;
    }

    // map clobbering defs to the blocks that read from them
    let use_blocks_by_def = collect_use_blocks_by_def(function, tree, memory_ssa.as_ref(), &alias);

    // track modifications
    let mut edge_blocks: HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    > = HashMap::new();
    let mut to_remove = HashSet::new();
    let mut changed = false;

    // evaluate candidates for sinking
    for candidate in candidates {
        let block = tree.get(candidate.block);
        let terminator = tree.get(block.terminator);
        let successors = terminator.successors();

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
            let Some(successor) = successor.block() else {
                continue;
            };

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

            let store_id = insert_store_for_candidate(tree, insertion_block, &candidate);
            clone_store_metadata(tree, candidate.instruction, store_id, candidate.pointer);
        }

        // remove the original store
        to_remove.insert(candidate.instruction);
    }

    // exit when no instructions were removed
    if to_remove.is_empty() {
        return false;
    }

    // remove sunk stores
    for &block_id in &function.blocks {
        let block = tree.get_mut(block_id);
        block.instructions.retain(|id| !to_remove.contains(id));
    }

    // drop memory metadata for removed stores
    for instruction_id in &to_remove {
        tree.metadata.memory.remove_memory_accesses(*instruction_id);
    }

    true
}

/// Collect store candidates for sinking.
fn collect_store_candidates(
    function: &mir::Function,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
    non_escaping_stack_allocs: &HashSet<mir::Value>,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Vec<StoreCandidate> {
    // scan blocks for store candidates
    let mut candidates = Vec::new();

    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            // select store instructions
            let (kind, pointer, local, value) = match tree.get(instruction_id) {
                mir::Instruction::Store { pointer, value } => {
                    let Some(pointer) = pointer.value() else {
                        continue;
                    };
                    let Some(value) = value.value() else {
                        continue;
                    };

                    (StoreKind::Store, Some(pointer), None, value)
                }
                mir::Instruction::LocalSet { local, value } => {
                    let Some(local) = local.local() else {
                        continue;
                    };
                    let Some(value) = value.value() else {
                        continue;
                    };

                    (StoreKind::LocalSet, None, Some(local), value)
                }
                _ => continue,
            };

            // read the MemorySSA access
            let access_id = match memory_ssa.access_for_instruction(instruction_id) {
                Some(access_id) => access_id,
                None => continue,
            };
            let MemoryAccess::Def(def_access) = memory_ssa.access(access_id) else {
                continue;
            };

            // require a write effect
            if !def_access.effect.writes {
                continue;
            }

            // skip ordered stores
            if instruction_has_atomic_ordering(tree, instruction_id) {
                continue;
            }

            // require a trackable effect
            if !effect_is_trackable(&def_access.effect) {
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

            // require a sinkable location
            if !store_is_sinkable_location(
                kind,
                pointer,
                non_escaping_stack_allocs,
                definitions,
                tree,
            ) {
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

/// Return true when a store targets a non escaping location.
fn store_is_sinkable_location(
    kind: StoreKind,
    pointer: Option<mir::Value>,
    non_escaping_stack_allocs: &HashSet<mir::Value>,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::Tree,
) -> bool {
    // allow local stores
    if matches!(kind, StoreKind::LocalSet) {
        return true;
    }

    // resolve the stack base for pointer stores
    let Some(pointer) = pointer else {
        return false;
    };
    let Some(base) = stack_alloc_base(pointer, definitions, tree) else {
        return false;
    };

    non_escaping_stack_allocs.contains(&base)
}

/// Collect blocks that read from each clobbering def.
fn collect_use_blocks_by_def(
    function: &mir::Function,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
) -> HashMap<MemoryAccessId, HashSet<mir::LocalNodeId<mir::Block>>> {
    // collect clobbering use blocks
    let mut blocks_by_def: HashMap<_, HashSet<_>> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        for &instruction_id in &block.instructions {
            let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
                continue;
            };

            for &access_id in accesses {
                let MemoryAccess::Use(_) = memory_ssa.access(access_id) else {
                    continue;
                };

                let clobber = memory_ssa.clobbering_access_for_use(access_id, alias, tree);
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
    use_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> bool {
    // use a queue for breadth first traversal
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(block) = queue.pop_front() {
        if use_blocks.contains(&block) {
            return true;
        }

        let block = tree.get(block);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };

            if visited.insert(successor) {
                queue.push_back(successor);
            }
        }
    }

    false
}

/// Insert a store instruction for a candidate.
fn insert_store_for_candidate(
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    candidate: &StoreCandidate,
) -> mir::LocalNodeId<mir::Instruction> {
    // build the new store instruction
    let instruction = match candidate.kind {
        StoreKind::Store => mir::Instruction::Store {
            pointer: candidate.pointer.expect("store pointer required").into(),
            value: candidate.value.into(),
        },
        StoreKind::LocalSet => mir::Instruction::LocalSet {
            local: candidate.local.expect("local target required").into(),
            value: candidate.value.into(),
        },
    };

    // insert the instruction in the edge block
    let instruction_id = tree.insert(instruction);
    let mut block = tree.get(block_id).clone();
    block.instructions.push(instruction_id);
    tree.replace(block_id, block);
    instruction_id
}

/// Clone store metadata to a new instruction.
fn clone_store_metadata(
    tree: &mut mir::Tree,
    source: mir::LocalNodeId<mir::Instruction>,
    destination: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
) {
    // skip when there is no metadata to clone
    let Some(accesses) = tree.metadata.memory.memory_accesses(source) else {
        return;
    };

    // update pointer targets for cloned metadata
    let mut cloned = Vec::with_capacity(accesses.len());
    for access in accesses {
        let mut updated = access.clone();
        if let (Some(pointer), mir::MemoryAccessTarget::Pointer(_)) = (pointer, updated.target) {
            updated.target = mir::MemoryAccessTarget::Pointer(pointer);
        }
        cloned.push(updated);
    }

    tree.metadata
        .memory
        .insert_memory_accesses(destination, cloned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Stores are sunk to the successor that reads them.
    #[test]
    fn test_store_sink_to_single_successor() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 7int32
    store v1, v2
    branch v0, b1, b2
b1:
    v3: int32 = load v1
    return v3
b2:
    return v2
}"#;

        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 7int32
    branch v0, b1, b3
b1:
    store v1, v2
    jump b2
b2:
    v3: int32 = load v1
    return v3
b3:
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StoreSink);
        test.assert_output(expected);
    }

    /// Stores needed on both edges are not sunk.
    #[test]
    fn test_store_sink_skips_all_successors() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 7int32
    store v1, v2
    branch v0, b1, b2
b1:
    v3: int32 = load v1
    return v3
b2:
    v4: int32 = load v1
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StoreSink);
        test.assert_output(input);
    }

    /// Stores to escaping memory are not sunk.
    #[test]
    fn test_store_sink_skips_escaping_store() {
        let input = r#"
function test(v0: boolean, v1: ref<int32, raw, space(static)>): void {
b0(v0: boolean, v1: ref<int32, raw, space(static)>):
    v2: int32 = 1int32
    store v1, v2
    branch v0, b1, b2
b1:
    return
b2:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StoreSink);
        test.assert_output(input);
    }
}
