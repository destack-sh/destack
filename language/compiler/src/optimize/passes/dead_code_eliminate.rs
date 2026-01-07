use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
    instruction_collect_used_values, instruction_has_side_effects,
};

declare_pass! {
    /// Eliminate dead code from functions.
    ///
    /// Removes instructions whose results are never used. An instruction is
    /// considered dead if its result value is not used by any other instruction
    /// or terminator, and the instruction has no side effects.
    #[pass(id = "dead-code-eliminate")]
    pub DeadCodeEliminate,
    "Eliminate dead code"
}

impl Pass for DeadCodeEliminate {
    fn metadata(&self) -> &'static PassMetadata {
        DeadCodeEliminate::metadata()
    }
}

impl FunctionPass for DeadCodeEliminate {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        // iterative DCE: keep removing dead code until fixpoint
        // (this is necessary because removing one dead instruction may leave another dead)
        loop {
            let used_values = instruction_collect_used_values(function, tree);
            let mut changed_this_iteration = false;

            // remove dead instructions from each block
            for &block_id in &function.blocks {
                let block = tree.get(block_id);
                let original_len = block.instructions.len();

                // collect live instructions
                let live_instructions: Vec<_> = block
                    .instructions
                    .iter()
                    .copied()
                    .filter(|&instruction_id| {
                        let instruction = tree.get(instruction_id);

                        // keep if instruction has a result that's used
                        if let Some(dest) = instruction.destination()
                            && used_values.contains(&dest)
                        {
                            return true;
                        }

                        // keep if instruction has side effects
                        instruction_has_side_effects(instruction)
                    })
                    .collect();

                // update block with live instructions only
                if live_instructions.len() != original_len {
                    let mut new_block = tree.get(block_id).clone();
                    new_block.instructions = live_instructions;
                    tree.replace(block_id, new_block);
                    changed_this_iteration = true;
                }
            }

            if !changed_this_iteration {
                break;
            }
        }

        // DCE doesn't change the CFG structure, only removes instructions
        // dominator tree and CFG are still valid
        AnalysisPreservation::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    #[test]
    fn test_removes_unused_instruction() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v0
}"#;
        // v1 and v2 are unused, should be removed
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    #[test]
    fn test_keeps_used_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#;
        // all instructions are used in the chain to v2
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_removes_multiple_dead_instructions() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iconst 3i32
    v3 = iadd v1, v2
    v4 = iconst 4i32
    return v0
}"#;
        // only v0 is used, everything else is dead
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    #[test]
    fn test_keeps_side_effect_instructions() {
        // calls have side effects and should be kept even if result unused
        let input = r#"function @test() -> void {
block0:
    v0 = iconst 1i32
    v1 = call @side_effect(v0)
    return
}
function @side_effect(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_unchanged(input);
    }

    #[test]
    fn test_removes_dead_in_multiple_blocks() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    v3 = iconst 3i32
    v4 = iconst 4i32
    return v1
block2:
    v5 = iconst 5i32
    return v1
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    #[test]
    fn test_keeps_value_used_in_terminator() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = icmp_sgt v0, v1
    branch v2, block1, block2
block1:
    return v0
block2:
    return v1
}"#;
        // v0, v1, v2 all used
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_unchanged(input);
    }
}
