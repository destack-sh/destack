use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{AliasAnalysis, ControlFlowGraph, DominatorTree, LoopAnalysis};
use crate::optimize::common::{
    MemoryLocation, build_instruction_block_map, build_use_def_maps, build_value_definition_map,
    instruction_is_memory_read, instruction_is_speculatable, instruction_may_affect_memory,
    instruction_requires_exact_access,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Sink instructions closer to their uses.
    ///
    /// Code sinking moves instructions from a block into successors where
    /// their results are used. This reduces register pressure and avoids
    /// executing instructions on code paths that don't need their results.
    ///
    /// ```mir
    /// function before(v0: int32, v1: boolean): int32 {
    /// b0(v0: int32, v1: boolean):
    ///     v2 = 1int32
    ///     v3 = int.add v0, v2
    ///     branch v1, b1, b2
    /// b1:
    ///     return v3
    /// b2:
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32, v1: boolean): int32 {
    /// b0(v0: int32, v1: boolean):
    ///     v2 = 1int32
    ///     branch v1, b1, b2
    /// b1:
    ///     v3 = int.add v0, v2
    ///     return v3
    /// b2:
    ///     return v0
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only sinks pure instructions OR memory reads with no intervening memory ops
    /// - Only sinks when ALL uses are in a single successor
    /// - Does not sink from outside a loop to inside (would increase execution frequency)
    /// - Does not sink into blocks with multiple predecessors
    ///
    /// Loads can be sunk when there are no intervening stores, calls, or other
    /// memory-affecting operations between the load and the branch. This is safe
    /// without alias analysis because the memory state cannot change.
    #[pass(id = "sink")]
    pub Sink,
    "Code sinking"
}

impl FunctionPass for Sink {
    /// Run code sinking on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let analyses = ctx.function_analyses(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let loops = analyses.get::<LoopAnalysis>().clone();
        let alias = analyses.get::<AliasAnalysis>();

        // run sink
        let changed = run_sink(entry, function, tree, &cfg, &domtree, &loops, &alias);

        // select preservation based on sink changes
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "Sink"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "sink"
    }
}

/// Sink logic. Returns true if changes were made.
fn run_sink(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    loops: &LoopAnalysis,
    alias: &AliasAnalysis,
) -> bool {
    // collect all blocks that are in any loop
    let loop_blocks: HashSet<mir::LocalNodeId<mir::Block>> = loops
        .loops()
        .iter()
        .flat_map(|loop_info| loop_info.blocks.iter().copied())
        .collect();

    // build value->uses map and value->defining-block map
    let use_def = build_use_def_maps(function, tree);
    let definition_map = build_value_definition_map(function, tree);
    let instruction_blocks = build_instruction_block_map(function, tree);

    // collect sinking work
    let mut work: Vec<SinkWork> = Vec::new();

    for &block_id in &function.blocks {
        // load the block and its successors
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let successors = terminator.successors();

        if successors.is_empty() {
            continue;
        }

        // check each instruction for sinking
        for (idx, &instruction_id) in block.instructions.iter().enumerate() {
            // read the instruction for analysis
            let instruction = tree.get(instruction_id);

            // do not sink instructions that require exact access semantics
            if instruction_requires_exact_access(tree, instruction_id) {
                continue;
            }

            // determine if the instruction can be sunk
            let can_sink = if instruction_is_speculatable(instruction, tree) {
                // pure instructions can always be sunk
                true
            } else if instruction_is_memory_read(instruction) {
                // memory reads can be sunk if intervening operations do not clobber
                memory_read_can_sink(block, idx, tree, alias)
            } else {
                // other instructions (stores, calls, etc.) cannot be sunk
                false
            };

            if !can_sink {
                continue;
            }

            // get the destination value
            let destination = match instruction.destination() {
                Some(d) => d,
                None => continue,
            };
            let Some(destination_value) = destination.value() else {
                continue;
            };

            // check that the value is not used in the terminator
            let terminator_uses: Vec<_> = terminator.uses().into_iter().collect();
            if terminator_uses.contains(&destination) {
                continue;
            }

            // check where the value is used
            let uses = use_def
                .use_blocks
                .get(&destination_value)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            if uses.is_empty() {
                // no uses, DCE will remove this
                continue;
            }

            // find the unique successor that uses this value
            let mut target_successor: Option<mir::LocalNodeId<mir::Block>> = None;
            for &use_block in uses {
                // skip uses in the same block (instructions after this one)
                if use_block == block_id {
                    target_successor = None;
                    break;
                }

                // must be a successor
                if !successors
                    .iter()
                    .any(|successor| successor.block() == Some(use_block))
                {
                    // used in a non successor block
                    // this can happen if the value flows through block parameters
                    target_successor = None;
                    break;
                }

                match target_successor {
                    None => target_successor = Some(use_block),
                    Some(existing) if existing != use_block => {
                        // used in multiple successors
                        target_successor = None;
                        break;
                    }
                    Some(_) => {}
                }
            }

            let successor = match target_successor {
                Some(s) => s,
                None => continue,
            };

            // don't sink into the entry block
            if successor == entry {
                continue;
            }

            // don't sink into blocks with multiple predecessors
            // (the sunk instruction might not dominate all predecessors)
            if cfg.predecessors(successor).len() > 1 {
                continue;
            }

            // avoid sinking inside loops
            if loop_blocks.contains(&block_id) {
                continue;
            }

            // verify the instruction's operands will still be available in the successor
            // (they must dominate the successor)
            let operands_ok = instruction.uses().iter().all(|&operand| {
                let Some(operand_value) = operand.value() else {
                    return false;
                };

                if let Some(def_id) = definition_map.get(&operand_value)
                    && instruction_blocks.get(def_id) == Some(&successor)
                {
                    return false;
                }

                // check if operand is defined in a block that dominates successor
                match use_def.def_block.get(&operand_value) {
                    Some(&operand_block) => {
                        domtree.dominates(operand_block, successor)
                            || domtree.dominates(operand_block, block_id)
                    }
                    // function parameter, always available
                    None => true,
                }
            });

            if !operands_ok {
                continue;
            }

            work.push(SinkWork {
                from_block: block_id,
                instruction_idx: idx,
                to_block: successor,
            });
        }
    }

    if work.is_empty() {
        return false;
    }

    // sort by instruction index descending so we can remove without invalidating indices
    work.sort_by(|a, b| b.instruction_idx.cmp(&a.instruction_idx));

    // group by source block
    let mut by_block: HashMap<mir::LocalNodeId<mir::Block>, Vec<SinkWork>> = HashMap::new();
    for w in work {
        by_block.entry(w.from_block).or_default().push(w);
    }

    // apply sinking
    for (from_block, work_items) in by_block {
        // collect instructions to sink (indices are already sorted descending)
        let mut to_sink: Vec<(
            mir::LocalNodeId<mir::Instruction>,
            mir::LocalNodeId<mir::Block>,
        )> = Vec::new();

        let from = tree.get(from_block);
        for w in &work_items {
            let instruction_id = from.instructions[w.instruction_idx];
            to_sink.push((instruction_id, w.to_block));
        }

        // remove from source block
        let mut from = tree.get(from_block).clone();
        for w in &work_items {
            from.instructions.remove(w.instruction_idx);
        }
        tree.replace(from_block, from);

        // insert at beginning of target blocks
        for (instruction_id, to_block) in to_sink {
            let mut to = tree.get(to_block).clone();
            to.instructions.insert(0, instruction_id);
            tree.replace(to_block, to);
        }
    }

    true
}

/// Check if a memory read can be sunk past intervening instructions.
fn memory_read_can_sink(
    block: &mir::Block,
    index: usize,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
) -> bool {
    // load the instruction to sink
    let instruction_id = block.instructions[index];
    let instruction = tree.get(instruction_id);

    match instruction {
        mir::Instruction::Load { pointer, .. } => {
            // check for clobbering memory operations
            let Some(pointer) = pointer.value() else {
                return false;
            };

            let location = MemoryLocation::from_ptr(pointer);
            for &later_id in &block.instructions[index + 1..] {
                let later = tree.get(later_id);
                if instruction_may_affect_memory(later) && alias.may_clobber(later_id, &location) {
                    return false;
                }
            }
            true
        }
        mir::Instruction::LocalGet { local, .. } => {
            // check for clobbering local sets
            for &later_id in &block.instructions[index + 1..] {
                // stop when a later local set clobbers the value
                if let mir::Instruction::LocalSet { local: later, .. } = tree.get(later_id)
                    && later == local
                {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

/// Work item for sinking an instruction.
struct SinkWork {
    from_block: mir::LocalNodeId<mir::Block>,
    instruction_idx: usize,
    to_block: mir::LocalNodeId<mir::Block>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::LoopSimplify;

    /// Instruction used only in one successor is sunk.
    #[test]
    fn test_sink_to_single_user() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    branch v1, b1, b2
b1:
    return v3
b2:
    return v0
}"#;
        let expected = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    branch v1, b1, b2
b1:
    v3: int32 = int.add v0, v2
    return v3
b2:
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Instruction used in terminator is not sunk.
    #[test]
    fn test_preserve_terminator_use() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = int.add v0, v1
    v3: int32 = 10int32
    v4: boolean = int.lt.s v2, v3
    branch v4, b1, b2
b1:
    return v2
b2:
    return v0
}"#;
        let mut test = TestProgram::new(input);
        let before = test.format();
        test.run_pass(&Sink);

        // v4 is used in the terminator, can't sink
        // v2 is used in both terminator AND block1, can't sink
        test.assert_output(&before);
    }

    /// Instruction with side effects is preserved.
    #[test]
    fn test_preserve_side_effects() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    drop v0
    branch v1, b1, b2
b1:
    return v2
b2:
    v3: int32 = 0int32
    return v3
}"#;
        // v2 is used only in block1, so it could sink if not for drop ordering
        // however, drop has side effects and cannot be reordered
        // v2 is computed before drop, so sinking v2 past drop would reorder them
        // actually, v2 has no dependency on drop, so v2 CAN sink to block1
        let expected = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    drop v0
    branch v1, b1, b2
b1:
    v2: int32 = 1int32
    return v2
b2:
    v3: int32 = 0int32
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Instruction used in multiple successors is preserved.
    #[test]
    fn test_preserve_multiple_users() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    branch v1, b1, b2
b1:
    return v3
b2:
    return v3
}"#;
        let mut test = TestProgram::new(input);
        let before = test.format();
        test.run_pass(&Sink);
        test.assert_output(&before);
    }

    /// Only the final instruction sinks when intermediate values have same-block uses.
    #[test]
    fn test_sink_chain_unconditional() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = int.add v0, v1
    v3: int32 = 2int32
    v4: int32 = int.add v2, v3
    jump b1
b1:
    return v4
}"#;
        // v1 used by v2 (same block) → can't sink
        // v2 used by v4 (same block) → can't sink
        // v3 used by v4 (same block) → can't sink
        // v4 used only in block1 → sinks
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = int.add v0, v1
    v3: int32 = 2int32
    jump b1
b1:
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Volatile loads are not sunk across control flow.
    #[test]
    fn test_preserve_volatile_load() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = load v1
    branch v0, b1, b2
b1:
    return v2
b2:
    v3: int32 = 0int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let load_id = test
            .entry_instructions(function_id)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    test.tree.get(*instruction_id),
                    mir::Instruction::Load { .. }
                )
            })
            .expect("missing load");

        test.insert_pointer_access_with_options(
            load_id,
            mir::MemoryAccessKind::Read,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );

        test.run_pass(&Sink);
        test.assert_unchanged(input);
    }

    /// Instruction is not sunk into block with multiple predecessors.
    #[test]
    fn test_preserve_multiple_predecessors() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    branch v1, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    return v3
}"#;
        let mut test = TestProgram::new(input);
        let before = test.format();
        test.run_pass(&Sink);
        test.assert_output(&before);
    }

    /// Instruction is not sunk from outside a loop to inside a loop.
    #[test]
    fn test_preserve_no_sink_into_loop() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    jump b1
b1:
    branch v1, b2, b3
b2:
    v4: int32 = int.add v3, v3
    jump b1
b3:
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&Sink);

        // v3 is used in b2 (inside loop) but defined in b0 (outside loop)
        // sinking would increase execution frequency
        test.assert_output(&before);
    }

    /// Empty function is unchanged.
    #[test]
    fn test_preserve_empty_function() {
        let input = r#"
function test(): void {
b0:
    return
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_unchanged(input);
    }

    /// Instruction used in same block is not sunk.
    #[test]
    fn test_preserve_same_block_use() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    v4: int32 = int.add v3, v2
    branch v1, b1, b2
b1:
    return v4
b2:
    return v0
}"#;
        // v2 is used by v3 and v4 in the same block, so cannot sink
        // v3 is used by v4 in the same block, so cannot sink
        // v4 could sink to block1, but v3 and v2 cannot
        let expected = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    branch v1, b1, b2
b1:
    v4: int32 = int.add v3, v2
    return v4
b2:
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Function with no sinkable instructions is unchanged.
    #[test]
    fn test_preserve_nothing_to_sink() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = int.add v0, v1
    return v2
}"#;
        let mut test = TestProgram::new(input);
        let before = test.format();
        test.run_pass(&Sink);
        test.assert_output(&before);
    }

    /// Sinking within a loop is allowed (same execution frequency).
    #[test]
    fn test_sink_within_loop() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    jump b1
b1:
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    branch v1, b2, b3
b2:
    v4: int32 = int.add v3, v2
    jump b1
b3:
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&Sink);

        // v3 is used in both block2 and block3, so can't sink
        // v2 is used in block1 (same block) and block2, so can't sink
        // no sinking should occur here
        test.assert_output(&before);
    }

    /// Sinking from loop block to single-predecessor successor within loop.
    #[test]
    fn test_sink_loop_internal() {
        let input = r#"
function test(v0: int32, v1: boolean): int32 {
b0(v0: int32, v1: boolean):
    jump b1
b1:
    v2: int32 = 1int32
    v3: int32 = int.add v0, v2
    jump b2
b2:
    branch v1, b1, b3
b3:
    return v3
}"#;
        // v3 is computed in block1 (inside loop) but only used in block3 (exit)
        // sinking v3 from inside the loop to outside would be beneficial,
        // but block3 has predecessor block2 which is inside the loop
        // the pass checks loop depth mismatch and prevents this
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&Sink);
        // no sinking should occur: sinking into the exit block would require
        // passing through the loop, and v2 is used by v3 in the same block
        test.assert_output(&before);
    }

    /// Loads are not sunk when there's an intervening store.
    #[test]
    fn test_preserve_load_with_intervening_store() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: boolean, v2: int32): int32 {
b0(v0: ref<int32, raw>, v1: boolean, v2: int32):
    v3: int32 = load v0
    store v0, v2
    branch v1, b1, b2
b1:
    return v3
b2:
    return v2
}"#;
        // v3 is only used in block1, but there's a store after the load
        // sinking past the store could change the loaded value
        let mut test = TestProgram::new(input);
        let before = test.format();
        test.run_pass(&Sink);
        test.assert_output(&before);
    }

    /// Loads CAN be sunk when there are no intervening memory operations.
    #[test]
    fn test_sink_load_no_intervening_ops() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: boolean): int32 {
b0(v0: ref<int32, raw>, v1: boolean):
    v2: int32 = load v0
    v3: int32 = 0int32
    branch v1, b1, b2
b1:
    return v2
b2:
    return v3
}"#;
        // v2 (load) is only used in b1, no intervening memory ops
        // v3 (const) is only used in b2
        // both are sunk to their respective successors
        let expected = r#"
function test(v0: ref<int32, raw>, v1: boolean): int32 {
b0(v0: ref<int32, raw>, v1: boolean):
    branch v1, b1, b2
b1:
    v2: int32 = load v0
    return v2
b2:
    v3: int32 = 0int32
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Pure instructions can still be sunk past side-effectful instructions.
    #[test]
    fn test_sink_pure_past_store() {
        let input = r#"
function test(v0: int32, v1: ref<int32, raw>, v2: boolean): int32 {
b0(v0: int32, v1: ref<int32, raw>, v2: boolean):
    v3: int32 = 1int32
    v4: int32 = int.add v0, v3
    store v1, v0
    branch v2, b1, b2
b1:
    return v4
b2:
    return v0
}"#;
        // v4 is a pure computation (int.add) used only in b1
        // it can be sunk past the store since it doesn't read memory
        let expected = r#"
function test(v0: int32, v1: ref<int32, raw>, v2: boolean): int32 {
b0(v0: int32, v1: ref<int32, raw>, v2: boolean):
    v3: int32 = 1int32
    store v1, v0
    branch v2, b1, b2
b1:
    v4: int32 = int.add v0, v3
    return v4
b2:
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }
}
