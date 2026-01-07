use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::AnalysisKind;
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
    instruction_has_side_effects,
};

declare_pass! {
    /// Aggressive Dead Code Elimination (ADCE).
    ///
    /// Uses LLVM-style reverse dataflow analysis to efficiently identify and remove dead
    /// instructions. An instruction is live if:
    /// - It has side effects (calls, stores, etc.)
    /// - Its result is used by a live instruction or terminator
    ///
    /// ```mir
    /// function @before(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iconst 42i32
    ///     v2 = iadd v0, v1
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     return v0
    /// }
    /// ```
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
        // phase 1: build value -> defining instruction map
        let mut value_to_instruction: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> =
            HashMap::new();
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(dest) = instruction.destination() {
                    value_to_instruction.insert(dest, instruction_id);
                }
            }
        }

        // phase 2: seed worklist with live roots
        let mut live: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Instruction>> = VecDeque::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            // side-effecting instructions are live roots
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if instruction_has_side_effects(instruction) && live.insert(instruction_id) {
                    worklist.push_back(instruction_id);
                }
            }

            // values used by terminators are live
            for value in block.terminator.uses() {
                if let Some(&instruction_id) = value_to_instruction.get(&value)
                    && live.insert(instruction_id)
                {
                    worklist.push_back(instruction_id);
                }
            }
        }

        // phase 3: propagate liveness backward through use-def chains
        while let Some(instruction_id) = worklist.pop_front() {
            let instruction = tree.get(instruction_id);

            // mark all operands as live
            for value in instruction.uses() {
                if let Some(&def_instruction_id) = value_to_instruction.get(&value)
                    && live.insert(def_instruction_id)
                {
                    worklist.push_back(def_instruction_id);
                }
            }

            // handle externalized arguments (Call, CallIndirect, Intrinsic)
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_arguments(args_slice) {
                    if let Some(&def_instruction_id) = value_to_instruction.get(&arg)
                        && live.insert(def_instruction_id)
                    {
                        worklist.push_back(def_instruction_id);
                    }
                }
            }
        }

        // phase 4: remove dead instructions from all blocks
        let mut changed = false;
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let original_len = block.instructions.len();
            let live_instructions: Vec<_> = block
                .instructions
                .iter()
                .copied()
                .filter(|id| live.contains(id))
                .collect();
            if live_instructions.len() != original_len {
                let mut new_block = block.clone();
                new_block.instructions = live_instructions;
                tree.replace(block_id, new_block);
                changed = true;
            }
        }

        if changed {
            // values changed, but CFG intact
            AnalysisPreservation::Some(vec![AnalysisKind::ControlFlowGraph])
        } else {
            AnalysisPreservation::all()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Unused instructions are eliminated from the function.
    #[test]
    fn test_eliminate_unused_instruction() {
        // v1 and v2 are unused
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v0
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    /// Instructions used in return chain are preserved.
    #[test]
    fn test_preserve_used_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_unchanged(input);
    }

    /// Multiple unused instructions are all eliminated.
    #[test]
    fn test_eliminate_multiple_dead() {
        // only v0 is used
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iconst 3i32
    v3 = iadd v1, v2
    v4 = iconst 4i32
    return v0
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    /// Calls have side effects and are preserved even when result is unused.
    #[test]
    fn test_preserve_side_effect_call() {
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

    /// Dead code is eliminated from all blocks in the function.
    #[test]
    fn test_eliminate_dead_in_multiple_blocks() {
        // v2, v3, v4, v5 are all dead
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

    /// Values used in branch terminators are preserved.
    #[test]
    fn test_preserve_terminator_uses() {
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

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_unchanged(input);
    }

    /// Transitive chains of dead code are all eliminated.
    #[test]
    fn test_eliminate_transitive_dead() {
        // v2, v3, v4 depend on each other but none used in return
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    v3 = imul v2, v0
    v4 = isub v3, v1
    return v0
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    /// Values passed as block arguments are preserved.
    #[test]
    fn test_preserve_block_arguments() {
        // v3 is dead, but v1 and v2 are used as block arguments
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    v3 = iconst 3i32
    branch v0, block1(v1), block1(v2)
block1(v4: i32):
    return v4
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block1(v2)
block1(v4: i32):
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    /// All instructions are eliminated when none are used by terminator.
    #[test]
    fn test_eliminate_all_instructions() {
        let input = r#"function @test() -> void {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return
}"#;
        let expected = r#"function @test() -> void {
block0:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    /// Dead code inside loops is eliminated.
    #[test]
    fn test_eliminate_dead_in_loop() {
        // v3, v4, v5 are all dead (none of their results are used)
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 100i32
    jump block1
block1:
    v2 = icmp_slt v0, v1
    v3 = iconst 999i32
    branch v2, block2, block3
block2:
    v4 = iconst 1i32
    v5 = imul v3, v3
    jump block1
block3:
    return v0
}"#;
        // v3, v4, v5 are all eliminated since their results are never used
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 100i32
    jump block1
block1:
    v2 = icmp_slt v0, v1
    branch v2, block2, block3
block2:
    jump block1
block3:
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    /// Dead code in diamond CFG branches is eliminated.
    #[test]
    fn test_eliminate_dead_in_diamond() {
        // v2, v3, v5 are dead
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    branch v0, block1, block2
block1:
    v2 = iconst 2i32
    v3 = iconst 3i32
    jump block3(v1)
block2:
    v4 = iconst 4i32
    v5 = iconst 5i32
    jump block3(v4)
block3(v6: i32):
    return v6
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    branch v0, block1, block2
block1:
    jump block3(v1)
block2:
    v4 = iconst 4i32
    jump block3(v4)
block3(v6: i32):
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_eq(expected);
    }

    /// Multiple side-effect calls are all preserved.
    #[test]
    fn test_preserve_multiple_side_effect_calls() {
        let input = r#"function @test() -> void {
block0:
    v0 = iconst 1i32
    v1 = call @side_effect(v0)
    v2 = call @side_effect(v0)
    v3 = call @side_effect(v0)
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
}
