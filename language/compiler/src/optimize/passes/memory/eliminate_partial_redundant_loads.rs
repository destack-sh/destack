use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasAnalysis, ControlFlowGraph, DominatorTree, EdgeSplitPolicy, MemoryAccessId, MemoryNode,
    MemorySSA, Mutation, append_edge_arguments, apply_substitutions_in_function,
    build_use_def_maps, ensure_edge_block, instruction_allows_read_only_motion,
    instruction_has_side_effects, instruction_is_read_only_access, instruction_is_speculatable,
    resolve_edge_value, value_available_in_block,
};

declare_pass! {
    /// Eliminate partially redundant loads using MemorySSA.
    ///
    /// Loads whose memory state flows through a MemorySSA phi can be
    /// replaced by per predecessor loads and a block parameter.
    ///
    /// ```mir
    /// function before(v0: boolean): int32 {
    ///     local l0: int32
    /// b0(v0: boolean):
    ///     v1 = local.address l0 -> ref<int32, borrowed, mutable, frame>
    ///     branch v0 => b1 | b2
    /// b1:
    ///     jump b3
    /// b2:
    ///     jump b3
    /// b3:
    ///     v2 = load v1 -> int32
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: boolean): int32 {
    ///     local l0: int32
    /// b0(v0: boolean):
    ///     v1 = local.address l0 -> ref<int32, borrowed, mutable, frame>
    ///     branch v0 => b1 | b2
    /// b1:
    ///     v4 = load v1 -> int32
    ///     jump b3(v4)
    /// b2:
    ///     v5 = load v1 -> int32
    ///     jump b3(v5)
    /// b3(v3: int32):
    ///     return v3
    /// }
    /// ```
    #[pass(id = "eliminate-partial-redundant-loads")]
    pub EliminatePartialRedundantLoads,
    "Eliminate partially redundant loads"
}

impl FunctionPass for EliminatePartialRedundantLoads {
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let memory = &mut optimized.memory;
        let effects = &mut optimized.effects;

        // skip imported functions
        if function.entry().is_none() {
            return Mutation::NONE;
        }

        // run load PRE
        let changed =
            run_eliminate_partial_redundant_loads(function, tree, memory, effects, ctx, analyses);

        // report what this pass changed
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    fn name(&self) -> &'static str {
        "EliminatePartialRedundantLoads"
    }

    fn id(&self) -> &'static str {
        "eliminate-partial-redundant-loads"
    }
}

/// Candidate load to PRE.
#[derive(Clone, Copy)]
struct LoadCandidate {
    /// Block containing the load.
    block: mir::LocalNodeId<mir::Block>,
    /// Load instruction id.
    load_id: mir::LocalNodeId<mir::Instruction>,
    /// Load destination value.
    destination: mir::Value,
    /// Load reference value.
    pointer: mir::Value,
    /// Load result type.
    result_type: mir::LocalNodeId<mir::Type>,
}

/// MemorySSA data for a candidate load.
#[derive(Clone, Copy)]
struct LoadAccessInfo {
    /// MemorySSA phi access for the load block.
    phi_access: MemoryAccessId,
}

/// Load insertion plan for a predecessor edge.
#[derive(Clone, Copy)]
struct EdgeInsertion {
    /// Predecessor block id.
    predecessor: mir::LocalNodeId<mir::Block>,
    /// Pointer value to load from on this edge.
    pointer: mir::Value,
    /// Existing value to reuse instead of inserting a load.
    existing_value: Option<mir::Value>,
}

/// Run load PRE and return true when changes are made.
fn run_eliminate_partial_redundant_loads(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    effects: &mir::EffectTable,
    _ctx: &PipelineContext<'_>,
    analyses: &mir::FunctionAnalysisCache,
) -> bool {
    // gather analyses
    let cfg = analyses.get::<ControlFlowGraph>(function, tree).clone();
    let domtree = analyses.get::<DominatorTree>(function, tree).clone();
    let memory_ssa = analyses.get::<MemorySSA>(function, tree);
    let alias = analyses.get::<AliasAnalysis>(function, tree);

    // build value definition info
    let use_def = build_use_def_maps(function, tree);
    let function_params: HashSet<_> = function
        .parameters
        .iter()
        .map(|param| param.value)
        .collect();

    // ensure fresh value allocation
    function.recompute_next_value_id(tree);

    // track modifications
    let mut substitutions = HashMap::new();
    let mut to_remove = HashSet::new();
    let mut edge_blocks: HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    > = HashMap::new();
    let mut changed = false;

    // scan each block for eligible loads
    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
        // collect block parameters for edge resolution
        let block = tree.get(block_id).clone();
        let param_indices = block
            .parameters
            .iter()
            .enumerate()
            .map(|(index, param)| (param.value, index))
            .collect::<HashMap<_, _>>();

        // scan block instructions for load candidates
        for &instruction_id in &block.instructions {
            // filter load instructions
            let load = match tree.get(instruction_id) {
                mir::Instruction::Load {
                    destination,
                    pointer,
                    result_type,
                } => {
                    let destination = *destination;
                    let pointer = *pointer;
                    let result_type = *result_type;

                    LoadCandidate {
                        block: block_id,
                        load_id: instruction_id,
                        destination,
                        pointer,
                        result_type,
                    }
                }
                _ => continue,
            };

            // validate load eligibility
            let Some(access_info) =
                load_access_info(&load, &block, tree, effects, memory_ssa.as_ref())
            else {
                continue;
            };

            // build edge insertions for each predecessor
            let Some(edge_insertions) = collect_edge_insertions(
                &load,
                &access_info,
                &cfg,
                &domtree,
                tree,
                &use_def.def_block,
                &function_params,
                &param_indices,
                memory_ssa.as_ref(),
                &alias,
            ) else {
                continue;
            };

            // allocate a new block parameter for the load value
            let param_value = function.next_typed_value(load.result_type);
            let param = mir::BlockParameter {
                value: param_value,
                ty: load.result_type,
            };
            let mut updated_block = tree.get(block_id).clone();
            updated_block.parameters.push(param);
            tree.set(block_id, updated_block);
            changed = true;

            // insert loads on each edge and append arguments
            for insertion in edge_insertions {
                // select or create the insertion block
                let insertion_block = match insertion.existing_value {
                    Some(_) => insertion.predecessor,
                    None => ensure_edge_block(
                        insertion.predecessor,
                        load.block,
                        function,
                        tree,
                        &cfg,
                        &mut edge_blocks,
                        EdgeSplitPolicy::PredecessorMultiSuccessor,
                        &mut changed,
                    ),
                };

                // reuse an existing load or insert a new one
                let load_value = if let Some(existing) = insertion.existing_value {
                    existing
                } else {
                    // insert a new load at the edge block
                    let load_value = function.next_typed_value(load.result_type);
                    let load_instruction = mir::Instruction::Load {
                        destination: load_value,
                        pointer: insertion.pointer,
                        result_type: load.result_type,
                    };
                    let load_id = tree.insert(load_instruction);

                    // append the new load before the terminator
                    let mut insertion_instructions = tree.get(insertion_block).instructions.clone();
                    insertion_instructions.push(load_id);
                    function.replace_block_instructions(
                        insertion_block,
                        insertion_instructions,
                        tree,
                    );

                    // clone memory access entries when present
                    clone_load_metadata(memory, load.load_id, load_id, insertion.pointer);
                    load_value
                };

                // append the load argument to the successor edge
                append_edge_arguments(tree, insertion_block, load.block, &[load_value]);
            }

            // record substitution and remove the original load
            substitutions.insert(load.destination, param_value);
            to_remove.insert(load.load_id);
        }
    }

    // return early when no substitutions were recorded
    if substitutions.is_empty() && to_remove.is_empty() {
        return changed;
    }

    // apply substitutions and removals
    let updated =
        apply_substitutions_in_function(function, tree, memory, &substitutions, Some(&to_remove));

    // drop memory tables for removed loads
    for load_id in &to_remove {
        memory.remove_memory_accesses(*load_id);
    }

    changed || updated
}

/// Return MemorySSA data when a load is eligible for load PRE.
fn load_access_info(
    load: &LoadCandidate,
    block: &mir::Block,
    tree: &mir::Tree,
    effects: &mir::EffectTable,
    memory_ssa: &MemorySSA,
) -> Option<LoadAccessInfo> {
    // resolve the memory ssa use access
    let use_access_id = memory_ssa.first_use_access(load.load_id)?;

    // require a memory phi at the block entry
    let phi_access = memory_ssa.block_phi(load.block)?;
    if memory_ssa.defining_access(use_access_id) != Some(phi_access) {
        return None;
    }

    // require a known reference location
    let MemoryNode::Use(use_access) = memory_ssa.access(use_access_id) else {
        return None;
    };
    // require a trackable effect
    if !use_access.effect.is_trackable() {
        return None;
    }

    // ensure the load can move to block entry
    if !load_can_move_to_entry(load.load_id, block, tree, effects, memory_ssa) {
        return None;
    }

    Some(LoadAccessInfo { phi_access })
}

/// Return true when a load can be moved to the block entry.
fn load_can_move_to_entry(
    load_id: mir::LocalNodeId<mir::Instruction>,
    block: &mir::Block,
    tree: &mir::Tree,
    effects: &mir::EffectTable,
    memory_ssa: &MemorySSA,
) -> bool {
    // inspect instructions before the load
    for &instruction_id in &block.instructions {
        // stop once the load is reached
        if instruction_id == load_id {
            break;
        }

        // read the instruction data
        let instruction = tree.get(instruction_id);

        // allow read only accesses with safe tables
        let read_only_access = instruction_is_read_only_access(instruction_id, memory_ssa);

        // reject side effecting instructions
        if instruction_has_side_effects(instruction) {
            if read_only_access
                && instruction_allows_read_only_motion(instruction_id, instruction, effects)
            {
                continue;
            }
            return false;
        }

        // accept speculatable instructions
        if instruction_is_speculatable(instruction, tree) {
            continue;
        }

        // accept read only memory accesses
        if read_only_access {
            continue;
        }

        // accept simple reads and reject everything else
        match instruction {
            mir::Instruction::Load { .. } | mir::Instruction::LocalGet { .. } => {}
            _ => return false,
        }
    }

    true
}

/// Collect edge insertions for each predecessor of the load block.
// allow many arguments to keep the edge selection explicit
fn collect_edge_insertions(
    load: &LoadCandidate,
    access_info: &LoadAccessInfo,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::Tree,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    function_params: &HashSet<mir::Value>,
    param_indices: &HashMap<mir::Value, usize>,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
) -> Option<Vec<EdgeInsertion>> {
    // collect predecessor edge insertions
    let mut insertions = Vec::new();
    let predecessors = cfg.predecessors(load.block);

    // require at least one predecessor
    if predecessors.is_empty() {
        return None;
    }

    // resolve incoming memory accesses for the load block
    let MemoryNode::Phi(phi) = memory_ssa.access(access_info.phi_access) else {
        return None;
    };
    let incoming_by_pred: HashMap<_, _> = phi
        .incoming
        .iter()
        .map(|(block, access)| (*block, *access))
        .collect();

    // reject pointers defined in the load block
    if !param_indices.contains_key(&load.pointer)
        && def_blocks
            .get(&load.pointer)
            .is_some_and(|def_block| *def_block == load.block)
    {
        return None;
    }

    // scan predecessors for insertion opportunities
    for &predecessor in predecessors {
        // resolve the edge pointer
        let predecessor_block = tree.get(predecessor);
        let pointer = resolve_edge_value(
            load.pointer,
            load.block,
            predecessor_block,
            tree,
            param_indices,
        )?;

        // ensure the reference value is available on this edge
        if !value_available_in_block(pointer, predecessor, def_blocks, function_params, domtree) {
            return None;
        }

        // read the incoming memory access for this predecessor
        let incoming_access = incoming_by_pred.get(&predecessor).copied()?;

        // reuse an existing load when possible
        let existing_value = reusable_predecessor_load(
            predecessor,
            pointer,
            load.result_type,
            incoming_access,
            tree,
            memory_ssa,
            alias,
        );

        // record the insertion decision
        insertions.push(EdgeInsertion {
            predecessor,
            pointer,
            existing_value,
        });
    }

    Some(insertions)
}

/// Find a reusable load value in a predecessor block.
fn reusable_predecessor_load(
    predecessor: mir::LocalNodeId<mir::Block>,
    pointer: mir::Value,
    result_type: mir::LocalNodeId<mir::Type>,
    incoming_access: MemoryAccessId,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
) -> Option<mir::Value> {
    // scan loads in order and reuse the latest matching load
    let block = tree.get(predecessor);
    let mut reusable = None;

    // inspect each instruction for a reusable load
    for &instruction_id in &block.instructions {
        let mir::Instruction::Load {
            destination,
            pointer: load_pointer,
            result_type: load_type,
        } = tree.get(instruction_id)
        else {
            continue;
        };

        // require the pointer and type to match
        if *load_pointer != pointer || *load_type != result_type {
            continue;
        }

        // require a memory ssa use for this load
        let Some(use_access_id) = memory_ssa.first_use_access(instruction_id) else {
            continue;
        };
        let MemoryNode::Use(use_access) = memory_ssa.access(use_access_id) else {
            continue;
        };

        // skip untrackable effects
        if !use_access.effect.is_trackable() {
            continue;
        }

        // require the same incoming memory state
        let load_clobber = memory_ssa.clobbering_use(use_access_id, alias);
        if load_clobber == incoming_access {
            reusable = Some(*destination);
        }
    }

    reusable
}

/// Clone load tables to a new instruction.
fn clone_load_metadata(
    memory: &mut mir::MemoryTable,
    source: mir::LocalNodeId<mir::Instruction>,
    destination: mir::LocalNodeId<mir::Instruction>,
    pointer: mir::Value,
) {
    // skip when there is no tables to clone
    let Some(accesses) = memory.memory_accesses(source) else {
        return;
    };

    // update reference targets for cloned tables
    let mut cloned = Vec::with_capacity(accesses.len());
    for access in accesses {
        let mut updated = access.clone();
        if matches!(updated.target, mir::MemoryTarget::Address(_)) {
            updated.target = mir::MemoryTarget::Address(pointer);
        }
        cloned.push(updated);
    }

    memory.insert_memory_accesses(destination, cloned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Load PRE inserts per edge loads for a join.
    #[test]
    fn test_eliminate_partial_redundant_loads_inserts_edge_loads() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v2: int32 = 1
    store v1, v2
    jump b3

b2:
    v3: int32 = 2
    store v1, v3
    jump b3

b3:
    v4: int32 = load v1
    return v4
}
"#;

        let expected = r#"
function test(v0: boolean): int32 {
    local l0: int32

entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v2: int32 = 1
    store v1, v2
    v6: int32 = load v1
    jump b3(v6)

b2:
    v3: int32 = 2
    store v1, v3
    v7: int32 = load v1
    jump b3(v7)

b3(v5: int32):
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(expected);
    }

    /// Loads are not moved when the pointer is defined in the join block.
    #[test]
    fn test_eliminate_partial_redundant_loads_skips_unavailable_pointer() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    v2: int32 = load v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(input);
    }

    /// Loads are not moved past side effecting instructions.
    #[test]
    fn test_eliminate_partial_redundant_loads_skips_side_effects() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    v2: int32 = 0
    store v1, v2
    v3: int32 = load v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(input);
    }

    /// Read only calls do not block load PRE.
    #[test]
    fn test_eliminate_partial_redundant_loads_allows_read_only_call() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v2: int32 = 1
    store v1, v2
    jump b3

b2:
    v3: int32 = 2
    store v1, v3
    jump b3

b3:
    call readOnly(): () => void
    v4: int32 = load v1
    return v4
}

external function readOnly(): void
"#;

        let expected = r#"
function test(v0: boolean): int32 {
    local l0: int32

entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v2: int32 = 1
    store v1, v2
    v6: int32 = load v1
    jump b3(v6)

b2:
    v3: int32 = 2
    store v1, v3
    v7: int32 = load v1
    jump b3(v7)

b3(v5: int32):
    call readOnly(): () => void
    return v5
}

external function readOnly(): void
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.function_id_by_name("test");
        let function = test.optimized.tree.get(function_id);
        let join_block = function.block(3);
        let call_inst = test.instructions_in_block(join_block)[0];
        let callsite = mir::CallSite::Instruction(call_inst);
        let tables = test.optimized.effects.call_mut(callsite);
        tables.memory = mir::MemoryEffect::read_only(mir::StorageSet::ANY);
        tables.behavior = mir::FunctionBehavior::none();

        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(expected);
    }

    /// Volatile loads are not moved.
    #[test]
    fn test_eliminate_partial_redundant_loads_skips_volatile_load() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    v2: int32 = load v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.optimized.tree.get(function_id);
        let join_block = function.block(3);
        let load_inst = test.instructions_in_block(join_block)[0];

        test.insert_pointer_access_with_options(
            load_inst,
            mir::MemoryOperation::Read,
            mir::Value::new(1),
            Some(4),
            true,
            None,
        );

        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(input);
    }

    /// Loads with non phi defining access are not moved.
    #[test]
    fn test_eliminate_partial_redundant_loads_skips_non_phi_defining_access() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    v2: int32 = 1
    store v1, v2
    v3: int32 = load v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(input);
    }

    /// Reuse predecessor loads that already match the incoming memory state.
    #[test]
    fn test_eliminate_partial_redundant_loads_reuses_predecessor_load() {
        let input = r#"
function test(v0: boolean, v1: boolean): int32 {
    local l0: int32
entry(v0: boolean, v1: boolean):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v3: int32 = 1
    store v2, v3
    v4: int32 = load v2
    branch v1 => b3 | b4

b2:
    jump b3

b3:
    v5: int32 = load v2
    return v5

b4:
    v6: int32 = 0
    return v6
}
"#;

        let expected = r#"
function test(v0: boolean, v1: boolean): int32 {
    local l0: int32

entry(v0: boolean, v1: boolean):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v3: int32 = 1
    store v2, v3
    v4: int32 = load v2
    branch v1 => b3(v4) | b4

b2:
    v8: int32 = load v2
    jump b3(v8)

b3(v7: int32):
    return v7

b4:
    v6: int32 = 0
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(expected);
    }

    /// Read only intrinsics do not block load PRE.
    #[test]
    fn test_eliminate_partial_redundant_loads_allows_read_only_intrinsic() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v2: int32 = 1
    store v1, v2
    jump b3

b2:
    v3: int32 = 2
    store v1, v3
    jump b3

b3:
    v4: int64 = 4
    v5: int32 = intrinsic.memory.raw.compareBytes(v1, v1, v4)
    v6: int32 = load v1
    return v6
}
"#;

        let expected = r#"
function test(v0: boolean): int32 {
    local l0: int32

entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v2: int32 = 1
    store v1, v2
    v8: int32 = load v1
    jump b3(v8)

b2:
    v3: int32 = 2
    store v1, v3
    v9: int32 = load v1
    jump b3(v9)

b3(v7: int32):
    v4: int64 = 4
    v5: int32 = intrinsic.memory.raw.compareBytes(v1, v1, v4)
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(expected);
    }

    /// Loads on edges with multiple successors use edge blocks.
    #[test]
    fn test_eliminate_partial_redundant_loads_splits_edge_blocks() {
        let input = r#"
function test(v0: boolean, v1: boolean): int32 {
    local l0: int32
entry(v0: boolean, v1: boolean):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    v3: int32 = 1
    store v2, v3
    branch v1 => b3 | b4

b2:
    v4: int32 = 2
    store v2, v4
    jump b3

b3:
    v5: int32 = load v2
    return v5

b4:
    v6: int32 = 0
    return v6
}
"#;

        let expected = r#"
function test(v0: boolean, v1: boolean): int32 {
    local l0: int32

entry(v0: boolean, v1: boolean):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b3

b1:
    v3: int32 = 1
    store v2, v3
    branch v1 => b2 | b5

b2:
    v8: int32 = load v2
    jump b4(v8)

b3:
    v4: int32 = 2
    store v2, v4
    v9: int32 = load v2
    jump b4(v9)

b4(v7: int32):
    return v7

b5:
    v6: int32 = 0
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantLoads);
        test.assert_output(expected);
    }
}
