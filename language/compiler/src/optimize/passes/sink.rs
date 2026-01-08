use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree, LoopAnalysis};
use crate::optimize::common::{build_use_def_maps, instruction_is_pure};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};
use mir::Instruction;

/// Check if an instruction is a memory read (load, local get).
fn instruction_is_memory_read(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::Load { .. } | Instruction::LocalGet { .. }
    )
}

/// Check if an instruction may write memory or have other side effects
/// that could affect a subsequent load.
fn instruction_may_affect_memory(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::Store { .. }
            | Instruction::LocalSet { .. }
            | Instruction::Call { .. }
            | Instruction::CallIndirect { .. }
            | Instruction::Intrinsic { .. }
            | Instruction::Drop { .. }
            | Instruction::ManagedAlloc { .. }
            | Instruction::ManagedAllocArray { .. }
            | Instruction::RawAlloc { .. }
            | Instruction::RawFree { .. }
            | Instruction::StackAlloc { .. }
    )
}

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

impl Pass for Sink {
    fn metadata(&self) -> &'static PassMetadata {
        Sink::metadata()
    }
}

impl FunctionPass for Sink {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        let cfg = context.analyses.get::<ControlFlowGraph>(function, tree);
        let domtree = context.analyses.get::<DominatorTree>(function, tree);
        let loops = context.analyses.get::<LoopAnalysis>(function, tree);

        // collect all blocks that are in any loop
        let loop_blocks: HashSet<mir::LocalNodeId<mir::Block>> = loops
            .loops()
            .iter()
            .flat_map(|lp| lp.blocks.iter().copied())
            .collect();

        // build value->uses map and value->defining-block map
        let use_def = build_use_def_maps(function, tree);

        // collect sinking work
        let mut work: Vec<SinkWork> = Vec::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let successors = block.terminator.successors();

            if successors.is_empty() {
                continue;
            }

            // check each instruction for sinking
            for (idx, &instruction_id) in block.instructions.iter().enumerate() {
                let instruction = tree.get(instruction_id);

                // determine if the instruction can be sunk
                let can_sink = if instruction_is_pure(instruction) {
                    // pure instructions can always be sunk
                    true
                } else if instruction_is_memory_read(instruction) {
                    // memory reads (loads) can be sunk if there are no intervening
                    // memory-affecting operations between this instruction and the terminator
                    let has_intervening_memory_op = block.instructions[idx + 1..]
                        .iter()
                        .any(|&instr_id| instruction_may_affect_memory(tree.get(instr_id)));
                    !has_intervening_memory_op
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
                    // no uses - DCE will remove this
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
                        // used in a non-successor block (perhaps a later block)
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

                // don't sink from outside a loop to inside a loop
                // (would increase execution frequency)
                let source_in_loop = loop_blocks.contains(&block_id);
                let target_in_loop = loop_blocks.contains(&successor);
                if !source_in_loop && target_in_loop {
                    continue;
                }

                // verify the instruction's operands will still be available in the successor
                // (they must dominate the successor)
                let operands_ok = instruction.uses().iter().all(|&operand| {
                    // check if operand is defined in a block that dominates successor
                    match use_def.def_block.get(&operand) {
                        Some(&operand_block) => {
                            domtree.dominates(operand_block, successor)
                                || domtree.dominates(operand_block, block_id)
                        }
                        // function parameter - always available
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

        drop(cfg);
        drop(domtree);
        drop(loops);

        if work.is_empty() {
            return AnalysisPreservation::all();
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

        AnalysisPreservation::none()
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
    v2 = iconst 1i32
    v3 = iadd v0, v2
    branch v1, block1, block2
block1:
    return v3
block2:
    return v0
}"#;
        let expected = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 1i32
    branch v1, block1, block2
block1:
    v3 = iadd v0, v2
    return v3
block2:
    return v0
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Sink);
        program.assert_output(expected);
    }

    /// Instruction used in terminator is not sunk.
    #[test]
    fn test_preserve_terminator_use() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = iadd v0, v1
    v3 = iconst 10i32
    v4 = icmp_slt v2, v3
    branch v4, block1, block2
block1:
    return v2
block2:
    return v0
}"#;
        let mut program = TestProgram::new(input);
        let before = program.format();
        program.run_pass(&Sink);

        // v4 is used in the terminator, can't sink
        // v2 is used in both terminator AND block1, can't sink
        program.assert_output(&before);
    }

    /// Instruction with side effects is preserved.
    #[test]
    fn test_preserve_side_effects() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 1i32
    drop v0
    branch v1, block1, block2
block1:
    return v2
block2:
    v3 = iconst 0i32
    return v3
}"#;
        // v2 is used only in block1, so it could sink if not for drop ordering
        // however, drop has side effects and cannot be reordered
        // v2 is computed before drop, so sinking v2 past drop would reorder them
        // actually, v2 has no dependency on drop, so v2 CAN sink to block1
        let expected = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    drop v0
    branch v1, block1, block2
block1:
    v2 = iconst 1i32
    return v2
block2:
    v3 = iconst 0i32
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Sink);
        program.assert_output(expected);
    }

    /// Instruction used in multiple successors is preserved.
    #[test]
    fn test_preserve_multiple_users() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 1i32
    v3 = iadd v0, v2
    branch v1, block1, block2
block1:
    return v3
block2:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        let before = program.format();
        program.run_pass(&Sink);
        program.assert_output(&before);
    }

    /// Only the final instruction sinks when intermediate values have same-block uses.
    #[test]
    fn test_sink_chain_unconditional() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = iadd v0, v1
    v3 = iconst 2i32
    v4 = iadd v2, v3
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
    v1 = iconst 1i32
    v2 = iadd v0, v1
    v3 = iconst 2i32
    jump block1
block1:
    v4 = iadd v2, v3
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Sink);
        program.assert_output(expected);
    }

    /// Instruction is not sunk into block with multiple predecessors.
    #[test]
    fn test_preserve_multiple_predecessors() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 1i32
    v3 = iadd v0, v2
    branch v1, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        let before = program.format();
        program.run_pass(&Sink);
        program.assert_output(&before);
    }

    /// Instruction is not sunk from outside a loop to inside a loop.
    #[test]
    fn test_preserve_no_sink_into_loop() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 1i32
    v3 = iadd v0, v2
    jump block1
block1:
    branch v1, block2, block3
block2:
    v4 = iadd v3, v3
    jump block1
block3:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&Sink);

        // v3 is used in block2 (inside loop) but defined in block0 (outside loop)
        // sinking would increase execution frequency
        program.assert_output(&before);
    }

    /// Empty function is unchanged.
    #[test]
    fn test_preserve_empty_function() {
        let input = r#"function @test() -> void {
block0:
    return
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Sink);
        program.assert_unchanged(input);
    }

    /// Instruction used in same block is not sunk.
    #[test]
    fn test_preserve_same_block_use() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    v2 = iconst 1i32
    v3 = iadd v0, v2
    v4 = iadd v3, v2
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
    v2 = iconst 1i32
    v3 = iadd v0, v2
    branch v1, block1, block2
block1:
    v4 = iadd v3, v2
    return v4
block2:
    return v0
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Sink);
        program.assert_output(expected);
    }

    /// Function with no sinkable instructions is unchanged.
    #[test]
    fn test_preserve_nothing_to_sink() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 1i32
    v2 = iadd v0, v1
    return v2
}"#;
        let mut program = TestProgram::new(input);
        let before = program.format();
        program.run_pass(&Sink);
        program.assert_output(&before);
    }

    /// Sinking within a loop is allowed (same execution frequency).
    #[test]
    fn test_sink_within_loop() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    jump block1
block1:
    v2 = iconst 1i32
    v3 = iadd v0, v2
    branch v1, block2, block3
block2:
    v4 = iadd v3, v2
    jump block1
block3:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&Sink);

        // v3 is used in both block2 and block3, so can't sink
        // v2 is used in block1 (same block) and block2, so can't sink
        // no sinking should occur here
        program.assert_output(&before);
    }

    /// Sinking from loop block to single-predecessor successor within loop.
    #[test]
    fn test_sink_loop_internal() {
        let input = r#"function @test(v0: i32, v1: bool) -> i32 {
block0(v0: i32, v1: bool):
    jump block1
block1:
    v2 = iconst 1i32
    v3 = iadd v0, v2
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
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&Sink);
        // no sinking should occur: sinking into the exit block would require
        // passing through the loop, and v2 is used by v3 in the same block
        program.assert_output(&before);
    }

    /// Loads are not sunk when there's an intervening store.
    #[test]
    fn test_preserve_load_with_intervening_store() {
        let input = r#"function @test(v0: ref<raw i32>, v1: bool, v2: i32) -> i32 {
block0(v0: ref<raw i32>, v1: bool, v2: i32):
    v3 = load v0
    store v0, v2
    branch v1, block1, block2
block1:
    return v3
block2:
    return v2
}"#;
        // v3 is only used in block1, but there's a store after the load
        // sinking past the store could change the loaded value
        let mut program = TestProgram::new(input);
        let before = program.format();
        program.run_pass(&Sink);
        program.assert_output(&before);
    }

    /// Loads CAN be sunk when there are no intervening memory operations.
    #[test]
    fn test_sink_load_no_intervening_ops() {
        let input = r#"function @test(v0: ref<raw i32>, v1: bool) -> i32 {
block0(v0: ref<raw i32>, v1: bool):
    v2 = load v0
    v3 = iconst 0i32
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
    v2 = load v0
    return v2
block2:
    v3 = iconst 0i32
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Sink);
        program.assert_output(expected);
    }

    /// Pure instructions can still be sunk past side-effectful instructions.
    #[test]
    fn test_sink_pure_past_store() {
        let input = r#"function @test(v0: i32, v1: ref<raw i32>, v2: bool) -> i32 {
block0(v0: i32, v1: ref<raw i32>, v2: bool):
    v3 = iconst 1i32
    v4 = iadd v0, v3
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
    v3 = iconst 1i32
    store v1, v0
    branch v2, block1, block2
block1:
    v4 = iadd v0, v3
    return v4
block2:
    return v0
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Sink);
        program.assert_output(expected);
    }
}
