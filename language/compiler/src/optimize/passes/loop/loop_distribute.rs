use std::collections::{HashMap, HashSet, VecDeque};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{
    AliasAnalysis, ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, MemoryAccess,
    MemoryAccessEffect, MemoryAccessLocation, MemorySSA, Mutation, block_is_speculatable_no_reads,
    build_instruction_block_map, build_value_definition_map, clone_loop_blocks_with_instructions,
    control_instructions_for_latch, effects_may_alias, instruction_has_atomic_ordering,
    instruction_is_speculatable, loop_guard_branch, loop_preheader, terminator_remap,
};

declare_pass! {
    /// Split independent store groups into separate loops.
    ///
    /// This transformation separates disjoint memory writes into multiple loops
    /// when the loop body is a single latch block and the groups do not alias.
    ///
    /// ```mir
    /// function before(v0: uint32): void {
    /// b0(v0: uint32):
    ///     v1 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     v2 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b2(v5), b3
    /// b2(v7: uint32):
    ///     v8 = element.address v1, v7 -> ref<int32, raw, space(frame)>
    ///     v9 = 1int32
    ///     store v8, v9
    ///     v10 = element.address v2, v7 -> ref<int32, raw, space(frame)>
    ///     v11 = 2int32
    ///     store v10, v11
    ///     v12 = int.add v7, v4
    ///     jump b1(v12)
    /// b3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: uint32): void {
    /// b0(v0: uint32):
    ///     v1 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     v2 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b2(v5), b4(v3)
    /// b2(v7: uint32):
    ///     v8 = element.address v1, v7 -> ref<int32, raw, space(frame)>
    ///     v9 = 1int32
    ///     store v8, v9
    ///     v12 = int.add v7, v4
    ///     jump b1(v12)
    /// b3:
    ///     return
    /// b4(v13: uint32):
    ///     v14 = int.lt.u v13, v0
    ///     branch v14, b5(v13), b3
    /// b5(v15: uint32):
    ///     v10 = element.address v2, v15 -> ref<int32, raw, space(frame)>
    ///     v11 = 2int32
    ///     store v10, v11
    ///     v16 = int.add v15, v4
    ///     jump b4(v16)
    /// }
    /// ```
    #[pass(id = "loop-distribute")]
    pub LoopDistribute,
    "Distribute independent memory operations into separate loops"
}

impl FunctionPass for LoopDistribute {
    /// Run loop distribution on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // gather analyses
        let loops = analyses.get::<LoopAnalysis>(function, tree).clone();
        let cfg = analyses.get::<ControlFlowGraph>(function, tree).clone();
        let domtree = analyses.get::<DominatorTree>(function, tree).clone();
        let memory_ssa = analyses.get::<MemorySSA>(function, tree);
        let alias = analyses.get::<AliasAnalysis>(function, tree).clone();

        // run loop distribution
        let changed = run_loop_distribute(
            function,
            tree,
            &loops,
            &cfg,
            &domtree,
            memory_ssa.as_ref(),
            &alias,
        );

        // report what this pass changed
        if changed {
            Mutation::CONTROL_FLOW | Mutation::VALUES
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "LoopDistribute"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "loop-distribute"
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
    control_instructions: HashSet<mir::LocalNodeId<mir::Instruction>>,
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
fn run_loop_distribute(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    loops: &LoopAnalysis,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
) -> bool {
    // build value definition info
    let definitions = build_value_definition_map(function, tree);
    let instruction_blocks = build_instruction_block_map(function, tree);

    // select a candidate loop
    let candidate = loops.loops().iter().find_map(|lp| {
        build_candidate(
            lp,
            tree,
            cfg,
            domtree,
            memory_ssa,
            alias,
            &definitions,
            &instruction_blocks,
        )
    });
    let Some(candidate) = candidate else {
        return false;
    };

    // apply the distribution
    apply_distribution(function, tree, &candidate)
}

/// Build a distribution candidate for a loop.
#[allow(clippy::too_many_arguments)]
fn build_candidate(
    lp: &Loop,
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
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
            if target.block.block()? != lp.header {
                return None;
            }
            if target.arguments.len() != tree.get(lp.header).parameters.len() {
                return None;
            }
        }
        _ => return None,
    }

    // require a safe header for distribution
    if !block_is_speculatable_no_reads(lp.header, tree, memory_ssa) {
        return None;
    }

    // collect control instructions in the latch
    let control_instructions =
        control_instructions_for_latch(lp.header, latch, tree, definitions, instruction_blocks);

    // collect store groups in the latch
    let groups = collect_store_groups(
        latch,
        tree,
        memory_ssa,
        alias,
        definitions,
        instruction_blocks,
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
#[allow(clippy::too_many_arguments)]
fn collect_store_groups(
    latch: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
    control_instructions: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) -> Option<Vec<StoreGroup>> {
    // set up latch scanning state
    let latch_block = tree.get(latch);
    let mut groups = Vec::new();
    let mut assigned: HashMap<mir::LocalNodeId<mir::Instruction>, usize> = HashMap::new();

    // reject side effects in the latch body
    for &instruction_id in &latch_block.instructions {
        let instruction = tree.get(instruction_id);

        // skip control instructions
        if control_instructions.contains(&instruction_id) {
            continue;
        }

        // accept speculatable instructions
        if instruction_is_speculatable(instruction, tree) {
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
            mir::Instruction::Store { pointer, value } => (Some(pointer.value()?), value.value()?),
            mir::Instruction::LocalSet { value, .. } => (None, value.value()?),
            _ => continue,
        };

        // skip already assigned anchors
        if assigned.contains_key(&instruction_id) {
            continue;
        }

        // collect dependent instructions
        let group_instructions = collect_group_instructions(
            instruction_id,
            pointer,
            value,
            latch,
            tree,
            definitions,
            instruction_blocks,
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
        let effects = collect_group_effects(&group_instructions, tree, memory_ssa)?;

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
#[allow(clippy::too_many_arguments)]
fn collect_group_instructions(
    anchor: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
    value: mir::Value,
    latch: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    instruction_blocks: &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Block>>,
    control_instructions: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) -> Option<HashSet<mir::LocalNodeId<mir::Instruction>>> {
    // seed the worklist with store operands
    let mut worklist = VecDeque::new();
    worklist.push_back(value);
    if let Some(pointer) = pointer {
        worklist.push_back(pointer);
    }

    // walk operand definitions within the latch
    let mut instructions = HashSet::new();
    instructions.insert(anchor);

    while let Some(next_value) = worklist.pop_front() {
        // resolve the defining instruction
        let Some(definition) = definitions.get(&next_value) else {
            continue;
        };

        // skip values defined outside the latch
        if instruction_blocks.get(definition) != Some(&latch) {
            continue;
        }

        // skip control instructions
        if control_instructions.contains(definition) {
            continue;
        }

        // skip already recorded instructions
        if !instructions.insert(*definition) {
            continue;
        }

        // reject unsupported instruction kinds
        let instruction = tree.get(*definition);
        if !matches!(
            instruction,
            mir::Instruction::Store { .. }
                | mir::Instruction::LocalSet { .. }
                | mir::Instruction::Load { .. }
        ) && !instruction_is_speculatable(instruction, tree)
        {
            return None;
        }

        // enqueue operand uses
        worklist.extend(
            instruction
                .uses()
                .into_iter()
                .filter_map(|value| value.value()),
        );
    }

    Some(instructions)
}

/// Collect memory effects for a group.
#[allow(clippy::too_many_arguments)]
fn collect_group_effects(
    instructions: &HashSet<mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> Option<Vec<MemoryAccessEffect>> {
    // collect memory effects
    let mut effects = Vec::new();
    for instruction_id in instructions {
        // reject ordered memory accesses
        if instruction_has_atomic_ordering(tree, *instruction_id) {
            return None;
        }

        // read memory accesses for this instruction
        let Some(accesses) = memory_ssa.accesses_for_instruction(*instruction_id) else {
            continue;
        };

        // validate effects for each access
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

            // require a known location
            if effect.location == MemoryAccessLocation::Unknown {
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
    group: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) -> Vec<mir::LocalNodeId<mir::Instruction>> {
    // preserve the original order
    latch_block
        .instructions
        .iter()
        .filter(|id| group.contains(id))
        .copied()
        .collect()
}

/// Check whether store groups are independent.
fn groups_are_independent(groups: &[StoreGroup], alias: &AliasAnalysis) -> bool {
    // compare each group pair
    for (index, group) in groups.iter().enumerate() {
        for other in groups.iter().skip(index + 1) {
            for effect in &group.effects {
                for other_effect in &other.effects {
                    if !(effect.writes || other_effect.writes) {
                        continue;
                    }
                    if effects_may_alias(alias, effect, other_effect) {
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
        let loop_blocks: HashSet<_> = [candidate.header, candidate.latch].into_iter().collect();
        let (block_map, value_map, instruction_map) =
            clone_loop_blocks_with_instructions(&loop_blocks, function, tree);

        // remap cloned terminators
        for &cloned_id in block_map.values() {
            let block = tree.get(cloned_id).clone();
            let terminator_id = block.terminator;
            let mut terminator = tree.get(terminator_id).clone();
            terminator_remap(&mut terminator, &block_map, &value_map);
            tree.set(cloned_id, block);
            tree.set(terminator_id, terminator);
        }

        // insert cloned blocks into the function
        let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
        cloned_blocks.sort();
        for block_id in cloned_blocks {
            function.blocks.push(block_id);
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

        prune_latch_instructions(tree, instance.latch, &keep_set);
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
            loop_instances[0].header.into(),
            candidate
                .preheader_args
                .iter()
                .copied()
                .map(Into::into)
                .collect(),
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
        Option<HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Instruction>>>,
}

/// Map a keep set through an optional instruction mapping.
fn map_keep_set(
    keep: &HashSet<mir::LocalNodeId<mir::Instruction>>,
    instruction_map: Option<
        &HashMap<mir::LocalNodeId<mir::Instruction>, mir::LocalNodeId<mir::Instruction>>,
    >,
) -> Option<HashSet<mir::LocalNodeId<mir::Instruction>>> {
    // return original set for the original loop
    let Some(instruction_map) = instruction_map else {
        return Some(keep.clone());
    };

    // map instruction ids for cloned loops
    let mut mapped = HashSet::new();
    for instruction_id in keep {
        let mapped_id = instruction_map.get(instruction_id)?;
        mapped.insert(*mapped_id);
    }

    Some(mapped)
}

/// Remove instructions not in the keep set.
fn prune_latch_instructions(
    tree: &mut mir::Tree,
    latch: mir::LocalNodeId<mir::Block>,
    keep: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // filter instruction ids
    let mut block = tree.get(latch).clone();
    let mut filtered = Vec::new();
    for instruction_id in &block.instructions {
        if keep.contains(instruction_id) {
            filtered.push(*instruction_id);
        } else {
            tree.metadata.memory.remove_memory_accesses(*instruction_id);
        }
    }

    // commit the filtered latch
    block.instructions = filtered;
    tree.set(latch, block);
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
        preheader_args.iter().copied().map(Into::into).collect()
    } else {
        Vec::new()
    };
    let exit_target = mir::BlockTarget::new(exit_target.into(), exit_arguments);

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
    fn test_loop_distribute_splits_stores() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: uint32 = 0uint32
    value4: uint32 = 1uint32
    jump block1(value3)

block1(value5: uint32):
    value6: boolean = int.lt.u value5, value0
    branch value6, block2(value5), block3()

block2(value7: uint32):
    value8: ref<int32, raw, space(frame)> = element.address value1, value7
    value9: int32 = 1int32
    store value8, value9
    value10: ref<int32, raw, space(frame)> = element.address value2, value7
    value11: int32 = 2int32
    store value10, value11
    value12: uint32 = int.add value7, value4
    jump block1(value12)

block3:
    return
}
"#;

        let expected = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: uint32 = 0uint32
    value4: uint32 = 1uint32
    jump block1(value3)

block1(value5: uint32):
    value6: boolean = int.lt.u value5, value0
    branch value6, block2(value5), block4(value3)

block2(value7: uint32):
    value8: ref<int32, raw, space(frame)> = element.address value1, value7
    value9: int32 = 1int32
    store value8, value9
    value12: uint32 = int.add value7, value4
    jump block1(value12)

block3:
    return

block4(value13: uint32):
    value14: boolean = int.lt.u value13, value0
    branch value14, block5(value13), block3()

block5(value15: uint32):
    value18: ref<int32, raw, space(frame)> = element.address value2, value15
    value19: int32 = 2int32
    store value18, value19
    value20: uint32 = int.add value15, value4
    jump block4(value20)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(expected);
    }

    /// Aliasable stores prevent distribution.
    #[test]
    fn test_loop_distribute_skips_aliasing() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: uint32 = 0uint32
    value3: uint32 = 1uint32
    jump block1(value2)

block1(value4: uint32):
    value5: boolean = int.lt.u value4, value0
    branch value5, block2(value4), block3()

block2(value6: uint32):
    value7: ref<int32, raw, space(frame)> = element.address value1, value6
    value8: int32 = 1int32
    store value7, value8
    value9: ref<int32, raw, space(frame)> = element.address value1, value6
    value10: int32 = 2int32
    store value9, value10
    value11: uint32 = int.add value6, value3
    jump block1(value11)

block3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Non speculatable latch instructions prevent distribution.
    #[test]
    fn test_loop_distribute_skips_side_effects() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: uint32 = 0uint32
    value3: uint32 = 1uint32
    jump block1(value2)

block1(value4: uint32):
    value5: boolean = int.lt.u value4, value0
    branch value5, block2(value4), block3()

block2(value6: uint32):
    call touch(value6): (uint32) -> void
    value7: ref<int32, raw, space(frame)> = element.address value1, value6
    value8: int32 = 1int32
    store value7, value8
    value9: uint32 = int.add value6, value3
    jump block1(value9)

block3:
    return
}

function touch(value0: uint32): void {
entry0(value0: uint32):
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Store groups that include a load are distributed.
    #[test]
    fn test_loop_distribute_splits_load_store_groups() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value4: uint32 = 0uint32
    value5: uint32 = 1uint32
    jump block1(value4)

block1(value6: uint32):
    value7: boolean = int.lt.u value6, value0
    branch value7, block2(value6), block3()

block2(value8: uint32):
    value9: ref<int32, raw, space(frame)> = element.address value1, value8
    value10: int32 = load value9
    value11: ref<int32, raw, space(frame)> = element.address value2, value8
    store value11, value10
    value12: ref<int32, raw, space(frame)> = element.address value3, value8
    value13: int32 = 1int32
    store value12, value13
    value14: uint32 = int.add value8, value5
    jump block1(value14)

block3:
    return
}
"#;

        let expected = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value4: uint32 = 0uint32
    value5: uint32 = 1uint32
    jump block1(value4)

block1(value6: uint32):
    value7: boolean = int.lt.u value6, value0
    branch value7, block2(value6), block4(value4)

block2(value8: uint32):
    value9: ref<int32, raw, space(frame)> = element.address value1, value8
    value10: int32 = load value9
    value11: ref<int32, raw, space(frame)> = element.address value2, value8
    store value11, value10
    value14: uint32 = int.add value8, value5
    jump block1(value14)

block3:
    return

block4(value15: uint32):
    value16: boolean = int.lt.u value15, value0
    branch value16, block5(value15), block3()

block5(value17: uint32):
    value21: ref<int32, raw, space(frame)> = element.address value3, value17
    value22: int32 = 1int32
    store value21, value22
    value23: uint32 = int.add value17, value5
    jump block4(value23)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(expected);
    }

    /// Local set groups are distributed into separate loops.
    #[test]
    fn test_loop_distribute_splits_local_sets() {
        let input = r#"
function test(value0: uint32): void {
    local local0: int32, owned
    local local1: int32, owned

entry0(value0: uint32):
    value1: uint32 = 0uint32
    value2: uint32 = 1uint32
    jump block1(value1)

block1(value3: uint32):
    value4: boolean = int.lt.u value3, value0
    branch value4, block2(value3), block3()

block2(value5: uint32):
    value6: int32 = 10int32
    local.set local0, value6
    value7: int32 = 20int32
    local.set local1, value7
    value8: uint32 = int.add value5, value2
    jump block1(value8)

block3:
    return
}
"#;

        let expected = r#"
function test(value0: uint32): void {
    local local0: int32, owned
    local local1: int32, owned

entry0(value0: uint32):
    value1: uint32 = 0uint32
    value2: uint32 = 1uint32
    jump block1(value1)

block1(value3: uint32):
    value4: boolean = int.lt.u value3, value0
    branch value4, block2(value3), block4(value1)

block2(value5: uint32):
    value6: int32 = 10int32
    local.set local0, value6
    value8: uint32 = int.add value5, value2
    jump block1(value8)

block3:
    return

block4(value9: uint32):
    value10: boolean = int.lt.u value9, value0
    branch value10, block5(value9), block3()

block5(value11: uint32):
    value13: int32 = 20int32
    local.set local1, value13
    value14: uint32 = int.add value11, value2
    jump block4(value14)
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(expected);
    }

    /// Header loads prevent distribution.
    #[test]
    fn test_loop_distribute_skips_header_load() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: uint32 = 0uint32
    value3: uint32 = 1uint32
    jump block1(value2)

block1(value4: uint32):
    value5: int32 = load value1
    value6: boolean = int.lt.u value4, value0
    branch value6, block2(value4), block3()

block2(value7: uint32):
    value8: ref<int32, raw, space(frame)> = element.address value1, value7
    value9: int32 = 1int32
    store value8, value9
    value10: uint32 = int.add value7, value3
    jump block1(value10)

block3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Single store groups are not distributed.
    #[test]
    fn test_loop_distribute_skips_single_group() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: uint32 = 0uint32
    value3: uint32 = 1uint32
    jump block1(value2)

block1(value4: uint32):
    value5: boolean = int.lt.u value4, value0
    branch value5, block2(value4), block3()

block2(value6: uint32):
    value7: ref<int32, raw, space(frame)> = element.address value1, value6
    value8: int32 = 1int32
    store value7, value8
    value9: uint32 = int.add value6, value3
    jump block1(value9)

block3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Missing preheaders prevent distribution.
    #[test]
    fn test_loop_distribute_skips_missing_preheader() {
        let input = r#"
function test(value0: boolean, value1: uint32): void {
entry0(value0: boolean, value1: uint32):
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: uint32 = 0uint32
    value4: uint32 = 1uint32
    branch value0, block2(value3), block1(value3)

block1(value5: uint32):
    jump block2(value5)

block2(value6: uint32):
    value7: boolean = int.lt.u value6, value1
    branch value7, block3(value6), block4()

block3(value8: uint32):
    value9: ref<int32, raw, space(frame)> = element.address value2, value8
    value10: int32 = 1int32
    store value9, value10
    value11: uint32 = int.add value8, value4
    jump block2(value11)

block4:
    return
}
"#;

        let expected = r#"
function test(value0: boolean, value1: uint32): void {
entry0(value0: boolean, value1: uint32):
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: uint32 = 0uint32
    value4: uint32 = 1uint32
    branch value0, block2(value3), block1(value3)

block1(value5: uint32):
    jump block2(value5)

block2(value6: uint32):
    value7: boolean = int.lt.u value6, value1
    branch value7, block3(value6), block4()

block3(value8: uint32):
    value9: ref<int32, raw, space(frame)> = element.address value2, value8
    value10: int32 = 1int32
    store value9, value10
    value11: uint32 = int.add value8, value4
    jump block2(value11)

block4:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(expected);
    }

    /// Unused latch instructions prevent distribution.
    #[test]
    fn test_loop_distribute_skips_unassigned_instruction() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: uint32 = 0uint32
    value4: uint32 = 1uint32
    jump block1(value3)

block1(value5: uint32):
    value6: boolean = int.lt.u value5, value0
    branch value6, block2(value5), block3()

block2(value7: uint32):
    value8: ref<int32, raw, space(frame)> = element.address value1, value7
    value9: int32 = 1int32
    store value8, value9
    value10: ref<int32, raw, space(frame)> = element.address value2, value7
    value11: int32 = 2int32
    store value10, value11
    value12: uint32 = int.add value7, value4
    value13: uint32 = int.add value12, value4
    jump block1(value12)

block3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Shared group instructions prevent distribution.
    #[test]
    fn test_loop_distribute_skips_shared_group_instructions() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value3: uint32 = 0uint32
    value4: uint32 = 1uint32
    jump block1(value3)

block1(value5: uint32):
    value6: boolean = int.lt.u value5, value0
    branch value6, block2(value5), block3()

block2(value7: uint32):
    value8: ref<int32, raw, space(frame)> = element.address value1, value7
    value9: uint32 = int.add value7, value4
    store value8, value9
    value10: ref<int32, raw, space(frame)> = element.address value2, value7
    store value10, value9
    value11: uint32 = int.add value7, value4
    jump block1(value11)

block3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Exit arguments prevent distribution.
    #[test]
    fn test_loop_distribute_skips_exit_arguments() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: uint32 = 0uint32
    value3: uint32 = 1uint32
    jump block1(value2)

block1(value4: uint32):
    value5: boolean = int.lt.u value4, value0
    branch value5, block2(value4), block3(value4)

block2(value6: uint32):
    value7: ref<int32, raw, space(frame)> = element.address value1, value6
    value8: int32 = 1int32
    store value7, value8
    value9: uint32 = int.add value6, value3
    jump block1(value9)

block3(value10: uint32):
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Multi block loops are not distributed.
    #[test]
    fn test_loop_distribute_skips_multi_block_loop() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: uint32 = 0uint32
    value3: uint32 = 1uint32
    jump block1(value2)

block1(value4: uint32):
    value5: boolean = int.lt.u value4, value0
    branch value5, block2(value4), block4()

block2(value6: uint32):
    value7: ref<int32, raw, space(frame)> = element.address value1, value6
    value8: int32 = 1int32
    store value7, value8
    jump block3(value6)

block3(value9: uint32):
    value10: uint32 = int.add value9, value3
    jump block1(value10)

block4:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }

    /// Non jump latches prevent distribution.
    #[test]
    fn test_loop_distribute_skips_non_jump_latch() {
        let input = r#"
function test(value0: uint32): void {
entry0(value0: uint32):
    value1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: uint32 = 0uint32
    value3: uint32 = 1uint32
    jump block1(value2)

block1(value4: uint32):
    value5: boolean = int.lt.u value4, value0
    branch value5, block2(value4), block3()

block2(value6: uint32):
    value7: ref<int32, raw, space(frame)> = element.address value1, value6
    value8: int32 = 1int32
    store value7, value8
    value9: boolean = int.lt.u value6, value0
    branch value9, block1(value6), block3()

block3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopDistribute);
        test.assert_output(input);
    }
}
