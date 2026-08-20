use std::collections::VecDeque;

use crate::optimize::declare_pass;
use destack_core::{FxIndexMap, FxIndexSet};
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasTable, ControlTable, DefinitionTable, DominatorTable, Loop, LoopTable, MemoryAccessEffect,
    MemoryNode, MemoryRegion, MemoryTable, Mutation, block_is_speculatable_no_reads,
    clone_loop_blocks_with_instructions, control_instructions_for_latch,
    instruction_is_speculatable, loop_guard_branch, loop_preheader, terminator_remap,
};

declare_pass! {
    /// Split independent store groups into separate loops.
    ///
    /// ```mir
    /// function before(v0: uint32): void {
    ///     local l0: [int32; 16]
    ///     local l1: [int32; 16]
    /// b0(v0: uint32):
    ///     v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    ///     v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    ///     v3: uint32 = 0
    ///     v4: uint32 = 1
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6: boolean = lt v5, v0
    ///     branch v6 => b2(v5) | b3
    /// b2(v7: uint32):
    ///     v8: ref<int32, borrowed, mutable, frame> = element.address v1, v7
    ///     v9: int32 = 1
    ///     store v8, v9
    ///     v10: ref<int32, borrowed, mutable, frame> = element.address v2, v7
    ///     v11: int32 = 2
    ///     store v10, v11
    ///     v12: uint32 = add v7, v4
    ///     jump b1(v12)
    /// b3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: uint32): void {
    ///     local l0: [int32; 16]
    ///     local l1: [int32; 16]
    /// b0(v0: uint32):
    ///     v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    ///     v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    ///     v3: uint32 = 0
    ///     v4: uint32 = 1
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6: boolean = lt v5, v0
    ///     branch v6 => b2(v5) | b4(v3)
    /// b2(v7: uint32):
    ///     v8: ref<int32, borrowed, mutable, frame> = element.address v1, v7
    ///     v9: int32 = 1
    ///     store v8, v9
    ///     v12: uint32 = add v7, v4
    ///     jump b1(v12)
    /// b3:
    ///     return
    /// b4(v13: uint32):
    ///     v14: boolean = lt v13, v0
    ///     branch v14 => b5(v13) | b3
    /// b5(v15: uint32):
    ///     v10: ref<int32, borrowed, mutable, frame> = element.address v2, v15
    ///     v11: int32 = 2
    ///     store v10, v11
    ///     v16: uint32 = add v15, v4
    ///     jump b4(v16)
    /// }
    /// ```
    #[pass(id = "distribute-loops")]
    pub DistributeLoops,
    "Distribute independent memory operations into separate loops"
}

impl FunctionPass for DistributeLoops {
    /// Run loop distribution on the function.
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

        // gather analyses
        let loops = analyses.loops(function, tree).clone();
        let cfg = analyses.control(function, tree).clone();
        let domtree = analyses.dominator(function, tree).clone();
        let memory = analyses.memory(function, tree, accesses, effects);
        let alias = analyses.alias(function, tree).clone();

        // run loop distribution
        let changed = run_distribute_loops(
            function,
            tree,
            accesses,
            &loops,
            &cfg,
            &domtree,
            memory.as_ref(),
            &alias,
        );

        // report what this pass changed
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Candidate loop distribution data.
struct DistributeCandidate {
    /// Loop header block.
    header: mir::LocalNodeId<mir::Block>,
    /// Loop latch block.
    latch: mir::LocalNodeId<mir::Block>,
    /// Exit block outside the loop.
    exit_block: mir::LocalNodeId<mir::Block>,
    /// Loop preheader block.
    preheader: mir::LocalNodeId<mir::Block>,
    /// Arguments passed from preheader to header.
    preheader_args: Vec<mir::Value>,
    /// True when the in loop edge is the then branch.
    in_loop_is_then: bool,
    /// Instructions required for loop control.
    control_instructions: FxIndexSet<mir::LocalNodeId<mir::Instruction>>,
    /// Store groups to distribute.
    groups: Vec<StoreGroup>,
}

/// Store group within a latch block.
struct StoreGroup {
    /// Ordered instructions in the group.
    instructions: Vec<mir::LocalNodeId<mir::Instruction>>,
    /// Memory effects within the group.
    effects: Vec<MemoryAccessEffect>,
}

/// Run loop distribution and return true when changes are made.
fn run_distribute_loops(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    loops: &LoopTable,
    cfg: &ControlTable,
    domtree: &DominatorTable,
    memory: &MemoryTable,
    alias: &AliasTable,
) -> bool {
    // build value definition info
    let definitions = DefinitionTable::build(function, tree);

    // select a candidate loop
    let candidate = loops.loops().iter().find_map(|lp| {
        build_candidate(
            lp,
            function,
            tree,
            accesses,
            cfg,
            domtree,
            memory,
            alias,
            &definitions,
        )
    });
    let Some(candidate) = candidate else {
        return false;
    };

    // apply the distribution
    apply_distribution(function, tree, accesses, &candidate)
}

/// Build a distribution candidate for a loop.
fn build_candidate(
    lp: &Loop,
    function: &mir::Function,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    cfg: &ControlTable,
    domtree: &DominatorTable,
    memory: &MemoryTable,
    alias: &AliasTable,
    definitions: &DefinitionTable,
) -> Option<DistributeCandidate> {
    // require a single latch and exit
    if !lp.has_single_latch() || !lp.has_single_exit() {
        return None;
    }

    // require the loop to be reduced to header and latch
    if lp.blocks.len() != 2 {
        return None;
    }

    // resolve header and latch blocks
    let latch = *lp.latches.first()?;
    if latch == lp.header {
        return None;
    }

    // resolve the preheader and header arguments
    let (preheader, preheader_args) = loop_preheader(lp.header, &lp.blocks, cfg, domtree, tree)?;

    // resolve guard terminator information
    let guard = loop_guard_branch(lp.header, latch, &lp.blocks, tree)?;
    if !guard.exit_arguments.is_empty() {
        return None;
    }

    // require an empty exit block
    if !tree.get(guard.exit_block).parameters.is_empty() {
        return None;
    }

    // require the latch to jump to the header
    let latch_block = tree.get(latch);
    let latch_terminator = tree.get(latch_block.terminator);
    match latch_terminator {
        mir::Terminator::Jump { target } => {
            if target.block != lp.header {
                return None;
            }
            if target.arguments.len() != tree.get(lp.header).parameters.len() {
                return None;
            }
        }
        _ => return None,
    }

    // require a safe header for distribution
    if !block_is_speculatable_no_reads(lp.header, function, tree, memory) {
        return None;
    }

    // collect control instructions in the latch
    let control_instructions = control_instructions_for_latch(lp.header, latch, tree, definitions);

    // collect store groups in the latch
    let groups = collect_store_groups(
        function,
        latch,
        tree,
        accesses,
        memory,
        alias,
        definitions,
        &control_instructions,
    )?;

    // require at least two independent groups
    if groups.len() < 2 {
        return None;
    }

    Some(DistributeCandidate {
        header: lp.header,
        latch,
        exit_block: guard.exit_block,
        preheader,
        preheader_args,
        in_loop_is_then: guard.in_loop_is_then,
        control_instructions,
        groups,
    })
}

/// Collect store groups within a latch block.
fn collect_store_groups(
    function: &mir::Function,
    latch: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    memory: &MemoryTable,
    alias: &AliasTable,
    definitions: &DefinitionTable,
    control_instructions: &FxIndexSet<mir::LocalNodeId<mir::Instruction>>,
) -> Option<Vec<StoreGroup>> {
    // set up latch scanning state
    let latch_block = tree.get(latch);
    let mut groups = Vec::new();
    let mut assigned: FxIndexMap<mir::LocalNodeId<mir::Instruction>, usize> = FxIndexMap::default();

    // reject side effects in the latch body
    for &instruction_id in &latch_block.instructions {
        let instruction = tree.get(instruction_id);

        // skip control instructions
        if control_instructions.contains(&instruction_id) {
            continue;
        }

        // accept speculatable instructions
        if instruction_is_speculatable(instruction, function, tree) {
            continue;
        }

        // accept explicit memory operations
        if matches!(
            instruction,
            mir::Instruction::Store { .. }
                | mir::Instruction::LocalSet { .. }
                | mir::Instruction::Load { .. }
        ) {
            continue;
        }

        // reject unsupported effects
        return None;
    }

    // build groups anchored by stores
    for &instruction_id in &latch_block.instructions {
        // skip control instructions
        if control_instructions.contains(&instruction_id) {
            continue;
        }

        // collect store anchors
        let instruction = tree.get(instruction_id);
        let (pointer, value) = match instruction {
            mir::Instruction::Store { pointer, value } => (Some(pointer), value),
            mir::Instruction::LocalSet { value, .. } => (None, value),
            _ => continue,
        };

        // skip already assigned anchors
        if assigned.contains_key(&instruction_id) {
            continue;
        }

        // collect dependent instructions
        let group_instructions = collect_group_instructions(
            instruction_id,
            pointer.copied(),
            *value,
            latch,
            function,
            tree,
            definitions,
            control_instructions,
        )?;

        // reject overlaps with other groups
        for member in &group_instructions {
            if assigned
                .get(member)
                .is_some_and(|existing| *existing != groups.len())
            {
                return None;
            }
        }

        // record group membership
        for member in &group_instructions {
            assigned.insert(*member, groups.len());
        }

        // collect memory effects
        let effects = collect_group_effects(&group_instructions, tree, accesses, memory)?;

        // record the group
        groups.push(StoreGroup {
            instructions: ordered_group_instructions(latch_block, &group_instructions),
            effects,
        });
    }

    // ensure every instruction is accounted for
    for &instruction_id in &latch_block.instructions {
        // skip control instructions
        if control_instructions.contains(&instruction_id) {
            continue;
        }

        // accept instructions already assigned to a group
        if assigned.contains_key(&instruction_id) {
            continue;
        }

        // reject unassigned instructions
        return None;
    }

    // ensure groups are independent
    if !groups_are_independent(&groups, alias) {
        return None;
    }

    Some(groups)
}

/// Collect group instructions from a store anchor.
fn collect_group_instructions(
    anchor: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
    value: mir::Value,
    latch: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mir::Tree,
    definitions: &DefinitionTable,
    control_instructions: &FxIndexSet<mir::LocalNodeId<mir::Instruction>>,
) -> Option<FxIndexSet<mir::LocalNodeId<mir::Instruction>>> {
    // seed the worklist with store operands
    let mut worklist = VecDeque::new();
    worklist.push_back(value);
    if let Some(pointer) = pointer {
        worklist.push_back(pointer);
    }

    // walk operand definitions within the latch
    let mut instructions = FxIndexSet::default();
    instructions.insert(anchor);

    while let Some(next_value) = worklist.pop_front() {
        // resolve the defining instruction
        let Some(definition) = definitions.instruction(next_value) else {
            continue;
        };

        // skip values defined outside the latch
        if definitions.block(next_value) != Some(latch) {
            continue;
        }

        // skip control instructions
        if control_instructions.contains(&definition) {
            continue;
        }

        // skip already recorded instructions
        if !instructions.insert(definition) {
            continue;
        }

        // reject unsupported instruction kinds
        let instruction = tree.get(definition);
        if !matches!(
            instruction,
            mir::Instruction::Store { .. }
                | mir::Instruction::LocalSet { .. }
                | mir::Instruction::Load { .. }
        ) && !instruction_is_speculatable(instruction, function, tree)
        {
            return None;
        }

        // enqueue operand uses
        worklist.extend(instruction.uses());
    }

    Some(instructions)
}

/// Collect memory effects for a group.
fn collect_group_effects(
    instructions: &FxIndexSet<mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    memory: &MemoryTable,
) -> Option<Vec<MemoryAccessEffect>> {
    // collect memory effects
    let mut effects = Vec::new();
    for instruction_id in instructions {
        // reject ordered memory accesses
        if accesses.is_ordered(*instruction_id, tree) {
            return None;
        }

        // read memory accesses for this instruction
        let Some(accesses) = memory.instruction_accesses(*instruction_id) else {
            continue;
        };

        // validate effects for each access
        for access_id in accesses {
            let effect = match memory.access(*access_id) {
                MemoryNode::Def(def_access) => def_access.effect.clone(),
                MemoryNode::Use(use_access) => use_access.effect.clone(),
                _ => continue,
            };

            // reject volatile or barrier effects
            if effect.is_volatile || effect.is_barrier {
                return None;
            }

            // require a known region
            if matches!(effect.region, MemoryRegion::Any { .. }) {
                return None;
            }

            // record the effect
            effects.push(effect);
        }
    }

    // reject groups without memory effects
    if effects.is_empty() {
        return None;
    }

    Some(effects)
}

/// Order instructions based on their position in the latch block.
fn ordered_group_instructions(
    latch_block: &mir::Block,
    group: &FxIndexSet<mir::LocalNodeId<mir::Instruction>>,
) -> Vec<mir::LocalNodeId<mir::Instruction>> {
    // preserve the original order
    latch_block
        .instructions
        .iter()
        .filter(|id| group.contains(*id))
        .copied()
        .collect()
}

/// Check whether store groups are independent.
fn groups_are_independent(groups: &[StoreGroup], alias: &AliasTable) -> bool {
    // compare each group pair
    for (index, group) in groups.iter().enumerate() {
        for other in groups.iter().skip(index + 1) {
            for effect in &group.effects {
                for other_effect in &other.effects {
                    if !(effect.writes || other_effect.writes) {
                        continue;
                    }
                    if effect.may_alias(alias, other_effect) {
                        return false;
                    }
                }
            }
        }
    }

    true
}

/// Apply loop distribution for a candidate.
fn apply_distribution(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    candidate: &DistributeCandidate,
) -> bool {
    // prepare for cloning
    function.recompute_next_value_id(tree);

    // record loop instances
    let mut loop_instances = Vec::new();
    loop_instances.push(LoopInstance {
        header: candidate.header,
        latch: candidate.latch,
        instruction_map: None,
    });

    // clone loops for each extra group
    for _ in 1..candidate.groups.len() {
        let loop_blocks: FxIndexSet<_> = [candidate.header, candidate.latch].into_iter().collect();
        let (block_map, value_map, instruction_map) =
            clone_loop_blocks_with_instructions(&loop_blocks, function, tree, accesses);

        // remap cloned terminators
        for &cloned_id in block_map.values() {
            let block = tree.get(cloned_id).clone();
            let terminator_id = block.terminator;
            let mut terminator = tree.get(terminator_id).clone();
            terminator_remap(tree, &mut terminator, &block_map, &value_map);
            tree.set(cloned_id, block);
            tree.set(terminator_id, terminator);
        }

        // insert cloned blocks into the function
        let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
        cloned_blocks.sort();
        for block_id in cloned_blocks {
            function.add_block(block_id, tree);
        }

        loop_instances.push(LoopInstance {
            header: block_map[&candidate.header],
            latch: block_map[&candidate.latch],
            instruction_map: Some(instruction_map),
        });
    }

    // prune each loop body to the assigned group
    for (index, instance) in loop_instances.iter().enumerate() {
        let mut keep = candidate.control_instructions.clone();
        keep.extend(candidate.groups[index].instructions.iter().copied());

        let Some(keep_set) = map_keep_set(&keep, instance.instruction_map.as_ref()) else {
            return false;
        };

        prune_latch_instructions(function, tree, accesses, instance.latch, &keep_set);
    }

    // chain loop exits together
    for index in 0..loop_instances.len() {
        let next_header = loop_instances
            .get(index + 1)
            .map(|instance| instance.header);

        if !update_header_exit(
            tree,
            loop_instances[index].header,
            candidate.exit_block,
            next_header,
            &candidate.preheader_args,
            candidate.in_loop_is_then,
        ) {
            return false;
        }
    }

    // update the preheader to enter the first header
    let preheader_block = tree.get(candidate.preheader).clone();
    let new_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget::new(
            loop_instances[0].header,
            tree.add_values(&candidate.preheader_args),
        ),
    };
    tree.set(candidate.preheader, preheader_block);
    tree.set(tree.get(candidate.preheader).terminator, new_terminator);

    true
}

/// Loop instance data for distribution.
struct LoopInstance {
    /// Header block for this loop instance.
    header: mir::LocalNodeId<mir::Block>,
    /// Latch block for this loop instance.
    latch: mir::LocalNodeId<mir::Block>,
    /// Instruction id mapping for cloned loops.
    instruction_map:
        Option<FxIndexMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Instruction>>>,
}

/// Map a keep set through an optional instruction mapping.
fn map_keep_set(
    keep: &FxIndexSet<mir::LocalNodeId<mir::Instruction>>,
    instruction_map: Option<
        &FxIndexMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Instruction>>,
    >,
) -> Option<FxIndexSet<mir::LocalNodeId<mir::Instruction>>> {
    // return original set for the original loop
    let Some(instruction_map) = instruction_map else {
        return Some(keep.clone());
    };

    // map instruction ids for cloned loops
    let mut mapped = FxIndexSet::default();
    for instruction_id in keep {
        let mapped_id = instruction_map.get(instruction_id)?;
        mapped.insert(*mapped_id);
    }

    Some(mapped)
}

/// Remove instructions not in the keep set.
fn prune_latch_instructions(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    latch: mir::LocalNodeId<mir::Block>,
    keep: &FxIndexSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // filter instruction ids
    let instruction_ids = tree.get(latch).instructions.clone();
    let mut filtered = Vec::new();
    for instruction_id in instruction_ids {
        if keep.contains(&instruction_id) {
            filtered.push(instruction_id);
        } else {
            accesses.remove(instruction_id);
        }
    }

    // commit the filtered latch
    function.replace_block_instructions(latch, filtered, tree);
}

/// Update the header terminator to chain loop exits.
fn update_header_exit(
    tree: &mut mir::Tree,
    header: mir::LocalNodeId<mir::Block>,
    exit_block: mir::LocalNodeId<mir::Block>,
    next_header: Option<mir::LocalNodeId<mir::Block>>,
    preheader_args: &[mir::Value],
    in_loop_is_then: bool,
) -> bool {
    // read the header branch terminator
    let header_block = tree.get(header).clone();
    let header_terminator = tree.get(header_block.terminator).clone();
    let mir::Terminator::Branch {
        then_target,
        else_target,
        condition,
    } = &header_terminator
    else {
        return false;
    };

    // select the exit target and arguments
    let exit_target = next_header.unwrap_or(exit_block);
    let exit_arguments = if next_header.is_some() {
        tree.add_values(preheader_args)
    } else {
        mir::ValueSlice::default()
    };
    let exit_target = mir::BlockTarget::new(exit_target, exit_arguments);

    // rewrite the header terminator
    let new_terminator = if in_loop_is_then {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: then_target.clone(),
            else_target: exit_target,
        }
    } else {
        mir::Terminator::Branch {
            condition: *condition,
            then_target: exit_target,
            else_target: else_target.clone(),
        }
    };

    // store the rewritten terminator
    tree.set(header, header_block);
    tree.set(tree.get(header).terminator, new_terminator);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Store groups are distributed into separate loops.
    #[test]
    fn test_distribute_loops_splits_stores() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
    local l1: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v0
    branch v6 => b2(v5) | b3

b2(v7: uint32):
    v8: ref<int32, borrowed, mutable, frame> = element.address v1, v7
    v9: int32 = 1
    store v8, v9
    v10: ref<int32, borrowed, mutable, frame> = element.address v2, v7
    v11: int32 = 2
    store v10, v11
    v12: uint32 = add v7, v4
    jump b1(v12)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
    local l1: [int32; 16]

entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v0
    branch v6 => b2(v5) | b4(v3)

b2(v7: uint32):
    v8: ref<int32, borrowed, mutable, frame> = element.address v1, v7
    v9: int32 = 1
    store v8, v9
    v12: uint32 = add v7, v4
    jump b1(v12)

b3:
    return

b4(v13: uint32):
    v14: boolean = lt v13, v0
    branch v14 => b5(v13) | b3

b5(v15: uint32):
    v18: ref<int32, borrowed, mutable, frame> = element.address v2, v15
    v19: int32 = 2
    store v18, v19
    v20: uint32 = add v15, v4
    jump b4(v20)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(expected);
    }

    /// Aliasable stores prevent distribution.
    #[test]
    fn test_distribute_loops_skips_aliasing() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v0
    branch v5 => b2(v4) | b3

b2(v6: uint32):
    v7: ref<int32, borrowed, mutable, frame> = element.address v1, v6
    v8: int32 = 1
    store v7, v8
    v9: ref<int32, borrowed, mutable, frame> = element.address v1, v6
    v10: int32 = 2
    store v9, v10
    v11: uint32 = add v6, v3
    jump b1(v11)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Non speculatable latch instructions prevent distribution.
    #[test]
    fn test_distribute_loops_skips_side_effects() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v0
    branch v5 => b2(v4) | b3

b2(v6: uint32):
    call touch(v6): (uint32) => void
    v7: ref<int32, borrowed, mutable, frame> = element.address v1, v6
    v8: int32 = 1
    store v7, v8
    v9: uint32 = add v6, v3
    jump b1(v9)

b3:
    return
}

function touch(v0: uint32): void {
entry(v0: uint32):
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Store groups that include a load are distributed.
    #[test]
    fn test_distribute_loops_splits_load_store_groups() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
    local l1: [int32; 16]
    local l2: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    v3: ref<[int32; 16], borrowed, mutable, frame> = local.address l2
    v4: uint32 = 0
    v5: uint32 = 1
    jump b1(v4)

b1(v6: uint32):
    v7: boolean = lt v6, v0
    branch v7 => b2(v6) | b3

b2(v8: uint32):
    v9: ref<int32, borrowed, mutable, frame> = element.address v1, v8
    v10: int32 = load v9
    v11: ref<int32, borrowed, mutable, frame> = element.address v2, v8
    store v11, v10
    v12: ref<int32, borrowed, mutable, frame> = element.address v3, v8
    v13: int32 = 1
    store v12, v13
    v14: uint32 = add v8, v5
    jump b1(v14)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
    local l1: [int32; 16]
    local l2: [int32; 16]

entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    v3: ref<[int32; 16], borrowed, mutable, frame> = local.address l2
    v4: uint32 = 0
    v5: uint32 = 1
    jump b1(v4)

b1(v6: uint32):
    v7: boolean = lt v6, v0
    branch v7 => b2(v6) | b4(v4)

b2(v8: uint32):
    v9: ref<int32, borrowed, mutable, frame> = element.address v1, v8
    v10: int32 = load v9
    v11: ref<int32, borrowed, mutable, frame> = element.address v2, v8
    store v11, v10
    v14: uint32 = add v8, v5
    jump b1(v14)

b3:
    return

b4(v15: uint32):
    v16: boolean = lt v15, v0
    branch v16 => b5(v15) | b3

b5(v17: uint32):
    v21: ref<int32, borrowed, mutable, frame> = element.address v3, v17
    v22: int32 = 1
    store v21, v22
    v23: uint32 = add v17, v5
    jump b4(v23)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(expected);
    }

    /// Local set groups are distributed into separate loops.
    #[test]
    fn test_distribute_loops_splits_local_sets() {
        let input = r#"
function test(v0: uint32): void {
    local l0: int32
    local l1: int32

entry(v0: uint32):
    v1: uint32 = 0
    v2: uint32 = 1
    jump b1(v1)

b1(v3: uint32):
    v4: boolean = lt v3, v0
    branch v4 => b2(v3) | b3

b2(v5: uint32):
    v6: int32 = 10
    local.set l0, v6
    v7: int32 = 20
    local.set l1, v7
    v8: uint32 = add v5, v2
    jump b1(v8)

b3:
    return
}
"#;

        let expected = r#"
function test(v0: uint32): void {
    local l0: int32
    local l1: int32

entry(v0: uint32):
    v1: uint32 = 0
    v2: uint32 = 1
    jump b1(v1)

b1(v3: uint32):
    v4: boolean = lt v3, v0
    branch v4 => b2(v3) | b4(v1)

b2(v5: uint32):
    v6: int32 = 10
    local.set l0, v6
    v8: uint32 = add v5, v2
    jump b1(v8)

b3:
    return

b4(v9: uint32):
    v10: boolean = lt v9, v0
    branch v10 => b5(v9) | b3

b5(v11: uint32):
    v13: int32 = 20
    local.set l1, v13
    v14: uint32 = add v11, v2
    jump b4(v14)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(expected);
    }

    /// Header loads prevent distribution.
    #[test]
    fn test_distribute_loops_skips_header_load() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: int32 = load v1
    v6: boolean = lt v4, v0
    branch v6 => b2(v4) | b3

b2(v7: uint32):
    v8: ref<int32, borrowed, mutable, frame> = element.address v1, v7
    v9: int32 = 1
    store v8, v9
    v10: uint32 = add v7, v3
    jump b1(v10)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Single store groups are not distributed.
    #[test]
    fn test_distribute_loops_skips_single_group() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v0
    branch v5 => b2(v4) | b3

b2(v6: uint32):
    v7: ref<int32, borrowed, mutable, frame> = element.address v1, v6
    v8: int32 = 1
    store v7, v8
    v9: uint32 = add v6, v3
    jump b1(v9)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Missing preheaders prevent distribution.
    #[test]
    fn test_distribute_loops_skips_missing_preheader() {
        let input = r#"
function test(v0: boolean, v1: uint32): void {
    local l0: [int32; 16]
entry(v0: boolean, v1: uint32):
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v3: uint32 = 0
    v4: uint32 = 1
    branch v0 => b2(v3) | b1(v3)

b1(v5: uint32):
    jump b2(v5)

b2(v6: uint32):
    v7: boolean = lt v6, v1
    branch v7 => b3(v6) | b4

b3(v8: uint32):
    v9: ref<int32, borrowed, mutable, frame> = element.address v2, v8
    v10: int32 = 1
    store v9, v10
    v11: uint32 = add v8, v4
    jump b2(v11)

b4:
    return
}
"#;

        let expected = r#"
function test(v0: boolean, v1: uint32): void {
    local l0: [int32; 16]
entry(v0: boolean, v1: uint32):
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v3: uint32 = 0
    v4: uint32 = 1
    branch v0 => b2(v3) | b1(v3)

b1(v5: uint32):
    jump b2(v5)

b2(v6: uint32):
    v7: boolean = lt v6, v1
    branch v7 => b3(v6) | b4

b3(v8: uint32):
    v9: ref<int32, borrowed, mutable, frame> = element.address v2, v8
    v10: int32 = 1
    store v9, v10
    v11: uint32 = add v8, v4
    jump b2(v11)

b4:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(expected);
    }

    /// Unused latch instructions prevent distribution.
    #[test]
    fn test_distribute_loops_skips_unassigned_instruction() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
    local l1: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v0
    branch v6 => b2(v5) | b3

b2(v7: uint32):
    v8: ref<int32, borrowed, mutable, frame> = element.address v1, v7
    v9: int32 = 1
    store v8, v9
    v10: ref<int32, borrowed, mutable, frame> = element.address v2, v7
    v11: int32 = 2
    store v10, v11
    v12: uint32 = add v7, v4
    v13: uint32 = add v12, v4
    jump b1(v12)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Shared group instructions prevent distribution.
    #[test]
    fn test_distribute_loops_skips_shared_group_instructions() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
    local l1: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: ref<[int32; 16], borrowed, mutable, frame> = local.address l1
    v3: uint32 = 0
    v4: uint32 = 1
    jump b1(v3)

b1(v5: uint32):
    v6: boolean = lt v5, v0
    branch v6 => b2(v5) | b3

b2(v7: uint32):
    v8: ref<int32, borrowed, mutable, frame> = element.address v1, v7
    v9: uint32 = add v7, v4
    store v8, v9
    v10: ref<int32, borrowed, mutable, frame> = element.address v2, v7
    store v10, v9
    v11: uint32 = add v7, v4
    jump b1(v11)

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Exit arguments prevent distribution.
    #[test]
    fn test_distribute_loops_skips_exit_arguments() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v0
    branch v5 => b2(v4) | b3(v4)

b2(v6: uint32):
    v7: ref<int32, borrowed, mutable, frame> = element.address v1, v6
    v8: int32 = 1
    store v7, v8
    v9: uint32 = add v6, v3
    jump b1(v9)

b3(v10: uint32):
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Multi block loops are not distributed.
    #[test]
    fn test_distribute_loops_skips_multi_block_loop() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v0
    branch v5 => b2(v4) | b4

b2(v6: uint32):
    v7: ref<int32, borrowed, mutable, frame> = element.address v1, v6
    v8: int32 = 1
    store v7, v8
    jump b3(v6)

b3(v9: uint32):
    v10: uint32 = add v9, v3
    jump b1(v10)

b4:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }

    /// Non jump latches prevent distribution.
    #[test]
    fn test_distribute_loops_skips_non_jump_latch() {
        let input = r#"
function test(v0: uint32): void {
    local l0: [int32; 16]
entry(v0: uint32):
    v1: ref<[int32; 16], borrowed, mutable, frame> = local.address l0
    v2: uint32 = 0
    v3: uint32 = 1
    jump b1(v2)

b1(v4: uint32):
    v5: boolean = lt v4, v0
    branch v5 => b2(v4) | b3

b2(v6: uint32):
    v7: ref<int32, borrowed, mutable, frame> = element.address v1, v6
    v8: int32 = 1
    store v7, v8
    v9: boolean = lt v6, v0
    branch v9 => b1(v6) | b3

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DistributeLoops);
        test.assert_output(input);
    }
}
