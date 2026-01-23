use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{AliasAnalysis, ControlFlowGraph, DominatorTree, LoopAnalysis};
use crate::optimize::common::{
    MemoryLocation, build_instruction_block_map, build_use_def_maps, build_value_definition_map,
    instruction_is_memory_read, instruction_is_speculatable, instruction_may_affect_memory,
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
    /// function @before(v0: i32, v1: bool) -> i32 {
    /// block0(v0: i32, v1: bool):
    ///     v2 = iconst 1i32
    ///     v3 = iadd v0, v2
    ///     branch v1, block1, block2
    /// block1:
    ///     return v3
    /// block2:
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32, v1: bool) -> i32 {
    /// block0(v0: i32, v1: bool):
    ///     v2 = iconst 1i32
    ///     branch v1, block1, block2
    /// block1:
    ///     v3 = iadd v0, v2
    ///     return v3
    /// block2:
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
        let successors = block.terminator.successors();

        if successors.is_empty() {
            continue;
        }

        // check each instruction for sinking
        for (idx, &instruction_id) in block.instructions.iter().enumerate() {
            // read the instruction for analysis
            let instruction = tree.get(instruction_id);

            // determine if the instruction can be sunk
            let can_sink = if instruction_is_speculatable(instruction) {
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

            // check that the value is not used in the terminator
            let terminator_uses: Vec<_> = block.terminator.uses().into_iter().collect();
            if terminator_uses.contains(&destination) {
                continue;
            }

            // check where the value is used
            let uses = use_def
                .use_blocks
                .get(&destination)
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
                if !successors.contains(&use_block) {
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
                if let Some(def_id) = definition_map.get(&operand)
                    && instruction_blocks.get(def_id) == Some(&successor)
                {
                    return false;
                }

                // check if operand is defined in a block that dominates successor
                match use_def.def_block.get(&operand) {
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
            let location = MemoryLocation::from_ptr(*pointer);
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
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    branch v1, block1, block2
block1:
    return v3
block2:
    return v0
}"#;
        let expected = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    branch v1, block1, block2
block1:
    v3: i32 = iadd v0, v2
    return v3
block2:
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Instruction used in terminator is not sunk.
    #[test]
    fn test_preserve_terminator_use() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: i32 = iadd v0, v1
    v3: i32 = iconst 10i32
    v4: bool = icmp_slt v2, v3
    branch v4, block1, block2
block1:
    return v2
block2:
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
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    raw.drop v0
    branch v1, block1, block2
block1:
    return v2
block2:
    v3: i32 = iconst 0i32
    return v3
}"#;
        // v2 is used only in block1, so it could sink if not for drop ordering
        // however, drop has side effects and cannot be reordered
        // v2 is computed before drop, so sinking v2 past drop would reorder them
        // actually, v2 has no dependency on drop, so v2 CAN sink to block1
        let expected = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    raw.drop v0
    branch v1, block1, block2
block1:
    v2: i32 = iconst 1i32
    return v2
block2:
    v3: i32 = iconst 0i32
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Instruction used in multiple successors is preserved.
    #[test]
    fn test_preserve_multiple_users() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    branch v1, block1, block2
block1:
    return v3
block2:
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
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: i32 = iadd v0, v1
    v3: i32 = iconst 2i32
    v4: i32 = iadd v2, v3
    jump block1
block1:
    return v4
}"#;
        // v1 used by v2 (same block) → can't sink
        // v2 used by v4 (same block) → can't sink
        // v3 used by v4 (same block) → can't sink
        // v4 used only in block1 → sinks
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: i32 = iadd v0, v1
    v3: i32 = iconst 2i32
    jump block1
block1:
    v4: i32 = iadd v2, v3
    return v4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Instruction is not sunk into block with multiple predecessors.
    #[test]
    fn test_preserve_multiple_predecessors() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    branch v1, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
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
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    jump block1
block1:
    branch v1, block2, block3
block2:
    v4: i32 = iadd v3, v3
    jump block1
block3:
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let before = test.format();
        test.run_pass(&Sink);

        // v3 is used in block2 (inside loop) but defined in block0 (outside loop)
        // sinking would increase execution frequency
        test.assert_output(&before);
    }

    /// Empty function is unchanged.
    #[test]
    fn test_preserve_empty_function() {
        let input = r#"function @test() -> void {
block0:
    return
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_unchanged(input);
    }

    /// Instruction used in same block is not sunk.
    #[test]
    fn test_preserve_same_block_use() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    v4: i32 = iadd v3, v2
    branch v1, block1, block2
block1:
    return v4
block2:
    return v0
}"#;
        // v2 is used by v3 and v4 in the same block, so cannot sink
        // v3 is used by v4 in the same block, so cannot sink
        // v4 could sink to block1, but v3 and v2 cannot
        let expected = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    branch v1, block1, block2
block1:
    v4: i32 = iadd v3, v2
    return v4
block2:
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Function with no sinkable instructions is unchanged.
    #[test]
    fn test_preserve_nothing_to_sink() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: i32 = iadd v0, v1
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
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    jump block1
block1:
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    branch v1, block2, block3
block2:
    v4: i32 = iadd v3, v2
    jump block1
block3:
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
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    jump block1
block1:
    v2: i32 = iconst 1i32
    v3: i32 = iadd v0, v2
    jump block2
block2:
    branch v1, block1, block3
block3:
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
        let input = r#"function @test(v0: ref<raw i32>, v1: bool, v2: i32) -> i32 {
block0(v0: ref<raw i32>, v1: bool, v2: i32):
    v3: i32 = load v0
    store v0, v2
    branch v1, block1, block2
block1:
    return v3
block2:
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
        let input = r#"function @test(v0: ref<raw i32>, v1: bool) -> i32 {
block0(v0: ref<raw i32>, v1: bool):
    v2: i32 = load v0
    v3: i32 = iconst 0i32
    branch v1, block1, block2
block1:
    return v2
block2:
    return v3
}"#;
        // v2 (load) is only used in block1, no intervening memory ops
        // v3 (iconst) is only used in block2
        // both are sunk to their respective successors
        let expected = r#"function @test(v0: ref<raw i32>, v1: bool) -> i32 {
block0(v0: ref<raw i32>, v1: bool):
    branch v1, block1, block2
block1:
    v2: i32 = load v0
    return v2
block2:
    v3: i32 = iconst 0i32
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }

    /// Pure instructions can still be sunk past side-effectful instructions.
    #[test]
    fn test_sink_pure_past_store() {
        let input = r#"function @test(v0: i32, v1: ref<raw i32>, v2: bool) -> i32 {
block0(v0: i32, v1: ref<raw i32>, v2: bool):
    v3: i32 = iconst 1i32
    v4: i32 = iadd v0, v3
    store v1, v0
    branch v2, block1, block2
block1:
    return v4
block2:
    return v0
}"#;
        // v4 is a pure computation (iadd) used only in block1
        // it can be sunk past the store since it doesn't read memory
        let expected = r#"function @test(v0: i32, v1: ref<raw i32>, v2: bool) -> i32 {
block0(v0: i32, v1: ref<raw i32>, v2: bool):
    v3: i32 = iconst 1i32
    store v1, v0
    branch v2, block1, block2
block1:
    v4: i32 = iadd v0, v3
    return v4
block2:
    return v0
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&Sink);
        test.assert_output(expected);
    }
}
