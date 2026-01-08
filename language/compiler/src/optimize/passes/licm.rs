use std::collections::HashSet;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{DominatorTree, Loop, LoopAnalysis};
use crate::optimize::common::instruction_is_pure;
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
};

declare_pass! {
    /// Move loop-invariant computations outside of loops.
    ///
    /// An instruction is loop-invariant if all its operands are defined outside
    /// the loop or by other loop-invariant instructions. Loop-invariant instructions
    /// can be hoisted to the loop preheader, reducing redundant computation.
    ///
    /// This pass hoists:
    /// - Pure arithmetic (Binary, Unary, Cast)
    /// - Constants
    /// - Pure aggregate operations (FieldGet, FieldSet, FieldAddr, ElementGet, ElementSet, ElementAddr)
    /// - Immutable global references (GlobalConst, GlobalAddr)
    ///
    /// Memory operations (Load, Store, LocalGet, LocalSet) require alias analysis
    /// and are not hoisted in this basic implementation.
    ///
    /// Requires canonical loop form (preheader, single latch) from LoopSimplify.
    #[pass(id = "licm")]
    pub Licm,
    "Loop invariant code motion"
}

impl Pass for Licm {
    fn metadata(&self) -> &'static PassMetadata {
        Licm::metadata()
    }
}

impl FunctionPass for Licm {
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

        let loops = context.analyses.get::<LoopAnalysis>(function, tree);
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }
        let domtree = context.analyses.get::<DominatorTree>(function, tree);

        // collect hoisting work for each loop, outermost first
        let mut all_work: Vec<HoistWork> = Vec::new();
        let mut already_queued: HashSet<(mir::LocalNodeId<mir::Block>, usize)> = HashSet::new();

        for lp in loops.loops().iter() {
            // find preheader (immediate dominator outside the loop)
            let preheader = match find_preheader(lp, &domtree, entry) {
                Some(p) => p,
                None => continue,
            };

            // collect values defined outside the loop
            let mut invariant_values: HashSet<mir::Value> = HashSet::new();

            // function parameters
            for param in &function.parameters {
                invariant_values.insert(param.value);
            }

            // values from blocks outside the loop
            for &block_id in &function.blocks {
                if lp.blocks.contains(&block_id) {
                    continue;
                }
                let block = tree.get(block_id);

                for param in &block.parameters {
                    invariant_values.insert(param.value);
                }

                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    if let Some(destination) = instruction.destination() {
                        invariant_values.insert(destination);
                    }
                }
            }

            // iteratively find loop-invariant instructions
            let mut changed = true;
            while changed {
                changed = false;
                for &block_id in &function.blocks {
                    if !lp.blocks.contains(&block_id) {
                        continue;
                    }
                    let block = tree.get(block_id);

                    for &instruction_id in &block.instructions {
                        let instruction = tree.get(instruction_id);
                        if let Some(destination) = instruction.destination() {
                            if invariant_values.contains(&destination) {
                                continue;
                            }
                            let is_pure = instruction_is_pure(instruction);
                            let operands_invariant = instruction
                                .uses()
                                .iter()
                                .all(|v| invariant_values.contains(v));
                            if is_pure && operands_invariant {
                                invariant_values.insert(destination);
                                changed = true;
                            }
                        }
                    }
                }
            }

            // collect instructions to hoist
            for &block_id in &function.blocks {
                if !lp.blocks.contains(&block_id) {
                    continue;
                }
                let block = tree.get(block_id);

                for (index, &instruction_id) in block.instructions.iter().enumerate() {
                    if already_queued.contains(&(block_id, index)) {
                        continue;
                    }

                    let instruction = tree.get(instruction_id);
                    if let Some(destination) = instruction.destination() {
                        let is_pure = instruction_is_pure(instruction);
                        let operands_invariant = instruction
                            .uses()
                            .iter()
                            .all(|v| invariant_values.contains(v));

                        // hoist if pure and all operands are invariant
                        if is_pure && invariant_values.contains(&destination) && operands_invariant
                        {
                            already_queued.insert((block_id, index));
                            all_work.push(HoistWork {
                                source_block: block_id,
                                instruction_index: index,
                                target_preheader: preheader,
                            });
                        }
                    }
                }
            }
        }

        drop(domtree);
        drop(loops);

        if all_work.is_empty() {
            return AnalysisPreservation::all();
        }

        // sort descending by (block, index) so removal doesn't invalidate indices
        all_work.sort_by(|a, b| {
            b.source_block
                .cmp(&a.source_block)
                .then(b.instruction_index.cmp(&a.instruction_index))
        });

        let mut hoisted_count = 0;
        let mut current_block: Option<mir::LocalNodeId<mir::Block>> = None;
        let mut block_data: Option<mir::Block> = None;
        let mut preheader_insertions: Vec<(
            mir::LocalNodeId<mir::Block>,
            mir::LocalNodeId<mir::Instruction>,
        )> = Vec::new();

        for work in all_work {
            // flush previous block if switching
            if current_block != Some(work.source_block) {
                if let (Some(block_id), Some(data)) = (current_block, block_data.take()) {
                    tree.replace(block_id, data);
                }
                current_block = Some(work.source_block);
                block_data = Some(tree.get(work.source_block).clone());
            }

            let data = block_data.as_mut().unwrap();
            let instr_id = data.instructions.remove(work.instruction_index);
            preheader_insertions.push((work.target_preheader, instr_id));
            hoisted_count += 1;
        }

        // flush last block
        if let (Some(block_id), Some(data)) = (current_block, block_data.take()) {
            tree.replace(block_id, data);
        }

        // insert into preheaders
        preheader_insertions.reverse();
        let mut preheaders_to_update: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();
        for (preheader, _) in &preheader_insertions {
            preheaders_to_update.insert(*preheader);
        }

        for preheader_id in preheaders_to_update {
            let mut preheader = tree.get(preheader_id).clone();
            for (target, instr_id) in &preheader_insertions {
                if *target == preheader_id {
                    preheader.instructions.push(*instr_id);
                }
            }
            tree.replace(preheader_id, preheader);
        }

        if hoisted_count > 0 {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }
}

/// Work item for hoisting an instruction.
struct HoistWork {
    source_block: mir::LocalNodeId<mir::Block>,
    instruction_index: usize,
    target_preheader: mir::LocalNodeId<mir::Block>,
}

/// Find the preheader of a loop.
///
/// The preheader is the immediate dominator of the header that is outside the loop.
fn find_preheader(
    lp: &Loop,
    domtree: &DominatorTree,
    _entry: mir::LocalNodeId<mir::Block>,
) -> Option<mir::LocalNodeId<mir::Block>> {
    let idom = domtree.immediate_dominator(lp.header)?;
    if lp.blocks.contains(&idom) {
        // idom is inside the loop, no proper preheader
        None
    } else {
        Some(idom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::LoopSimplify;

    /// Constant in loop is hoisted to preheader.
    #[test]
    fn test_hoist_constant() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    jump block1
block1:
    v1 = iconst 42i32
    branch v0, block1, block2
block2:
    return v1
}"#;
        // v1 = iconst 42 should be hoisted to block0
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 42i32
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Binary operation on invariant operands is hoisted.
    #[test]
    fn test_hoist_binary_invariant() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    jump block1
block1:
    v3 = iadd v1, v2
    branch v0, block1, block2
block2:
    return v3
}"#;
        // v3 = iadd v1, v2 is invariant (v1, v2 are function params)
        let expected = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    v3 = iadd v1, v2
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Chain of invariant operations is hoisted.
    #[test]
    fn test_hoist_chain() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    jump block1
block1:
    v2 = iconst 10i32
    v3 = iadd v1, v2
    v4 = imul v3, v2
    branch v0, block1, block2
block2:
    return v4
}"#;
        // all three instructions are invariant
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 10i32
    v3 = iadd v1, v2
    v4 = imul v3, v2
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Operation using loop-variant value is not hoisted.
    #[test]
    fn test_no_hoist_variant() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    jump block1(v1)
block1(v2: i32):
    v3 = iconst 1i32
    v4 = iadd v2, v3
    branch v0, block1(v4), block2
block2:
    return v4
}"#;
        // v3 is invariant and can be hoisted
        // v4 depends on v2 which is a loop phi, so v4 cannot be hoisted
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v3 = iconst 1i32
    jump block1(v1)
block1(v2: i32):
    v4 = iadd v2, v3
    branch v0, block1(v4), block2
block2:
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Function without loops is unchanged.
    #[test]
    fn test_no_loops() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&Licm);
        program.assert_unchanged(input);
    }

    /// Already hoisted code is unchanged.
    #[test]
    fn test_already_hoisted() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    v3 = iadd v1, v2
    jump block1
block1:
    branch v0, block1, block2
block2:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_unchanged(input);
    }

    /// Invariant in inner loop is hoisted to inner preheader.
    #[test]
    fn test_hoist_nested_inner() {
        let input = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    jump block1
block1:
    jump block2
block2:
    v3 = iconst 5i32
    v4 = iadd v2, v3
    branch v1, block2, block3
block3:
    branch v0, block1, block4
block4:
    return v4
}"#;
        // v3 and v4 are invariant to the inner loop, hoist to block1 (inner preheader)
        // actually v4 uses v2 which is a function param, so both are invariant to outer too
        // they should be hoisted to block0
        let expected = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    v3 = iconst 5i32
    v4 = iadd v2, v3
    jump block1
block1:
    jump block2
block2:
    branch v1, block2, block3
block3:
    branch v0, block1, block4
block4:
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }

    /// Calls are not hoisted (side effects).
    #[test]
    fn test_no_hoist_call() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    jump block1
block1:
    v1 = call @get_value()
    branch v0, block1, block2
block2:
    return v1
}

function @get_value() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;
        // call should not be hoisted
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&Licm);
        program.assert_output(&before);
    }

    /// Allocations are not hoisted (each iteration needs fresh allocation).
    #[test]
    fn test_no_hoist_alloc() {
        let input = r#"function @test(v0: bool) -> ref<managed i32> {
block0(v0: bool):
    jump block1
block1:
    v1 = managed.alloc i32
    branch v0, block1, block2
block2:
    return v1
}"#;
        // managed.alloc should stay in loop: each iteration allocates a new object
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        let before = program.format();
        program.run_pass(&Licm);
        program.assert_output(&before);
    }

    /// Multiple independent loops each get their invariants hoisted.
    #[test]
    fn test_multiple_loops() {
        let input = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    jump block1
block1:
    v3 = iconst 10i32
    branch v0, block1, block2
block2:
    jump block3
block3:
    v4 = iconst 20i32
    v5 = iadd v2, v4
    branch v1, block3, block4
block4:
    v6 = iadd v3, v5
    return v6
}"#;
        // v3 hoisted from loop1 to block0
        // v4, v5 hoisted from loop3 to block2
        let expected = r#"function @test(v0: bool, v1: bool, v2: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32):
    v3 = iconst 10i32
    jump block1
block1:
    branch v0, block1, block2
block2:
    v4 = iconst 20i32
    v5 = iadd v2, v4
    jump block3
block3:
    branch v1, block3, block4
block4:
    v6 = iadd v3, v5
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&Licm);
        program.assert_output(expected);
    }
}
