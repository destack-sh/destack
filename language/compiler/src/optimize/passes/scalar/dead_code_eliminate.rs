use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::AliasAnalysis;
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, instruction_has_side_effects,
};

declare_pass! {
    /// Aggressive Dead Code Elimination (ADCE).
    ///
    /// Uses LLVM-style reverse dataflow analysis to efficiently identify and remove dead
    /// instructions. An instruction is live if:
    /// - It has side effects (calls, stores, etc.)
    /// - Its result is used by a live instruction or terminator
    ///
    /// Also removes local or memory stores that are overwritten before any read.
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

impl FunctionPass for DeadCodeEliminate {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // build alias analysis for local dead store elimination
        let analyses = ctx.function_analyses(function, tree);
        let alias = analyses.get::<AliasAnalysis>();

        // run dead code elimination
        let changed = run_dead_code_elimination(function, tree, &alias);

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "DeadCodeEliminate"
    }

    fn id(&self) -> &'static str {
        "dead-code-eliminate"
    }
}

/// Core dead code elimination logic (shared by both pass implementations).
fn run_dead_code_elimination(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    alias: &AliasAnalysis,
) -> bool {
    // drop dead stores before liveness
    let mut changed = remove_dead_stores(function, tree, alias);

    // build value to defining instruction map
    let mut value_to_instruction: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> =
        HashMap::new();
    for &block_id in &function.blocks {
        // scan block instructions for definitions
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            // record the defining instruction
            let instruction = tree.get(instruction_id);
            if let Some(dest) = instruction.destination() {
                value_to_instruction.insert(dest, instruction_id);
            }
        }
    }

    // seed live roots and worklist
    let mut live: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    let mut worklist: VecDeque<mir::LocalNodeId<mir::Instruction>> = VecDeque::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // record side effecting instructions as live
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if instruction_has_side_effects(instruction) && live.insert(instruction_id) {
                worklist.push_back(instruction_id);
            }
        }

        // record terminator uses as live
        for value in block.terminator.uses() {
            if let Some(&instruction_id) = value_to_instruction.get(&value)
                && live.insert(instruction_id)
            {
                worklist.push_back(instruction_id);
            }
        }
    }

    // propagate liveness through dependencies
    while let Some(instruction_id) = worklist.pop_front() {
        let instruction = tree.get(instruction_id);

        // mark operands as live
        for value in instruction.uses() {
            if let Some(&def_instruction_id) = value_to_instruction.get(&value)
                && live.insert(def_instruction_id)
            {
                worklist.push_back(def_instruction_id);
            }
        }

        // mark external argument uses as live
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

    // remove dead instructions from blocks
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let original_len = block.instructions.len();
        let live_instructions: Vec<_> = block
            .instructions
            .iter()
            .copied()
            .filter(|id| live.contains(id))
            .collect();

        // rewrite the block when instructions are removed
        if live_instructions.len() != original_len {
            let mut new_block = block.clone();
            new_block.instructions = live_instructions;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Remove dead stores and return true when changes are made.
fn remove_dead_stores(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    alias: &AliasAnalysis,
) -> bool {
    // collect locals that are read anywhere
    let mut locals_read = HashSet::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            if let mir::Instruction::LocalGet { local, .. } = tree.get(instruction_id) {
                locals_read.insert(*local);
            }
        }
    }

    // find dead store instructions
    let mut dead_stores = HashSet::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let instruction_ids = block.instructions.clone();

        for (index, instruction_id) in instruction_ids.iter().copied().enumerate() {
            let instruction = tree.get(instruction_id);

            // classify stores and check for overwrites
            match instruction {
                mir::Instruction::LocalSet { local, .. } => {
                    if !locals_read.contains(local)
                        || local_set_overwritten(&instruction_ids, index, *local, tree)
                    {
                        dead_stores.insert(instruction_id);
                    }
                }
                mir::Instruction::Store { pointer, .. } => {
                    if store_overwritten_in_block(&instruction_ids, index, *pointer, tree, alias) {
                        dead_stores.insert(instruction_id);
                    }
                }
                _ => {}
            }
        }
    }

    // exit early when nothing is removed
    if dead_stores.is_empty() {
        return false;
    }

    // remove dead stores from blocks
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        if block.instructions.iter().any(|id| dead_stores.contains(id)) {
            let mut new_block = block.clone();
            new_block
                .instructions
                .retain(|id| !dead_stores.contains(id));
            tree.replace(block_id, new_block);
        }
    }

    true
}

/// Check whether a local.set is overwritten in the same block.
fn local_set_overwritten(
    instruction_ids: &[mir::LocalNodeId<mir::Instruction>],
    start: usize,
    local: mir::LocalNodeId<mir::Local>,
    tree: &mir::NodeTree,
) -> bool {
    // scan later instructions in the block
    for instruction_id in instruction_ids.iter().skip(start + 1).copied() {
        match tree.get(instruction_id) {
            // stop when a read observes the local
            mir::Instruction::LocalGet {
                local: get_local, ..
            } if *get_local == local => {
                return false;
            }
            // stop when a later write overwrites the local
            mir::Instruction::LocalSet {
                local: set_local, ..
            } if *set_local == local => {
                return true;
            }
            _ => {}
        }
    }

    false
}

/// Check whether a store is overwritten before any aliasing memory access.
fn store_overwritten_in_block(
    instruction_ids: &[mir::LocalNodeId<mir::Instruction>],
    start: usize,
    pointer: mir::Value,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
) -> bool {
    // build a memory location for the stored pointer
    let location = crate::optimize::common::MemoryLocation::from_ptr(pointer);

    // scan later instructions in the block
    for instruction_id in instruction_ids.iter().skip(start + 1).copied() {
        let instruction = tree.get(instruction_id);

        // stop when a later store overwrites this location
        if let mir::Instruction::Store {
            pointer: other_ptr, ..
        } = instruction
        {
            let other_loc = crate::optimize::common::MemoryLocation::from_ptr(*other_ptr);
            let alias_result = alias.alias(&location, &other_loc);

            if alias_result.is_must_alias() || location.ptr == other_loc.ptr {
                return true;
            }
            if alias_result.may_alias() {
                return false;
            }
            continue;
        }

        // stop when any instruction may read or write the location
        let mod_ref = alias.get_mod_ref_info(instruction_id, &location);
        if mod_ref.is_ref() || mod_ref.is_mod() {
            return false;
        }
    }

    false
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

        // expected output
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Instructions used in return chain are preserved.
    #[test]
    fn test_preserve_used_chain() {
        // source program
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return v2
}"#;

        // run the pass and verify output
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

        // expected output
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Calls have side effects and are preserved even when result is unused.
    #[test]
    fn test_preserve_side_effect_call() {
        // source program
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

        // run the pass and verify output
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

        // expected output
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v1
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Values used in branch terminators are preserved.
    #[test]
    fn test_preserve_terminator_uses() {
        // source program
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

        // run the pass and verify output
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

        // expected output
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
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

        // expected output
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block1(v2)
block1(v4: i32):
    return v4
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// All instructions are eliminated when none are used by terminator.
    #[test]
    fn test_eliminate_all_instructions() {
        // source program
        let input = r#"function @test() -> void {
block0:
    v0 = iconst 1i32
    v1 = iconst 2i32
    v2 = iadd v0, v1
    return
}"#;

        // expected output
        let expected = r#"function @test() -> void {
block0:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
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

        // expected output
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

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
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

        // expected output
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

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Multiple side-effect calls are all preserved.
    #[test]
    fn test_preserve_multiple_side_effect_calls() {
        // source program
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

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_unchanged(input);
    }

    /// Overwritten local sets are removed when not observed.
    #[test]
    fn test_remove_overwritten_local_set() {
        // source program
        let input = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    local.set local0, v0
    v1 = iconst 3i32
    local.set local0, v1
    v2 = local.get local0
    return v2
}"#;

        // expected output
        let expected = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    v1 = iconst 3i32
    local.set local0, v1
    v2 = local.get local0
    return v2
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Local stores without any reads are removed.
    #[test]
    fn test_remove_unread_local_set() {
        // source program
        let input = r#"function @test(v0: i32) -> void {
    local0: i32 ; owned, mut
block0(v0: i32):
    local.set local0, v0
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: i32) -> void {
    local0: i32 ; owned, mut
block0(v0: i32):
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Stores overwritten before any read are eliminated.
    #[test]
    fn test_remove_overwritten_store() {
        // source program
        let input = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 1i32
    v2 = iconst 2i32
    store v0, v1
    store v0, v2
    return
}"#;

        // expected output
        let expected = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 2i32
    store v0, v2
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_output(expected);
    }

    /// Stores read by a load are preserved.
    #[test]
    fn test_preserve_store_used_by_load() {
        // source program
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 1i32
    store v0, v1
    v2 = load v0 -> i32
    v3 = iconst 2i32
    store v0, v3
    return v2
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadCodeEliminate);
        program.assert_unchanged(input);
    }
}
