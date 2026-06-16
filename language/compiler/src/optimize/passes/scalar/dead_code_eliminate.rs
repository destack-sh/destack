use std::collections::{HashMap, HashSet, VecDeque};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{
    AliasAnalysis, MemoryLocation, Mutation, instruction_has_side_effects,
    instruction_requires_exact_access,
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
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 42int32
    ///     v2 = int.add v0, v1
    ///     return v0
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
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
        tree: &mut mir::Tree,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // build alias analysis for local dead store elimination
        let alias = analyses.get::<AliasAnalysis>(function, tree);

        // run dead code elimination
        let changed = run_dead_code_elimination(function, tree, &alias);

        // report what this pass changed
        if changed {
            Mutation::VALUES
        } else {
            Mutation::NONE
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
    tree: &mut mir::Tree,
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
            if let Some(dest) = instruction.destination()
                && let Some(dest) = dest.value()
            {
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
            if (instruction_has_side_effects(instruction)
                || instruction_requires_exact_access(tree, instruction_id))
                && live.insert(instruction_id)
            {
                worklist.push_back(instruction_id);
            }
        }

        // record terminator uses as live
        let terminator = tree.get(block.terminator);
        for value in terminator.uses() {
            let Some(value) = value.value() else {
                continue;
            };
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
            let Some(value) = value.value() else {
                continue;
            };
            if let Some(&def_instruction_id) = value_to_instruction.get(&value)
                && live.insert(def_instruction_id)
            {
                worklist.push_back(def_instruction_id);
            }
        }

        // mark external argument uses as live
        if let Some(args_slice) = instruction.argument_slice() {
            for &arg in tree.get_arguments(args_slice) {
                let Some(arg) = arg.value() else {
                    continue;
                };
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
            tree.set(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Remove dead stores and return true when changes are made.
fn remove_dead_stores(
    function: &mir::Function,
    tree: &mut mir::Tree,
    alias: &AliasAnalysis,
) -> bool {
    // collect locals that are read anywhere
    let mut locals_read = HashSet::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            if let mir::Instruction::LocalGet { local, .. } = tree.get(instruction_id)
                && let Some(local) = local.local()
            {
                locals_read.insert(local);
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
                    let Some(local) = local.local() else {
                        continue;
                    };

                    if !locals_read.contains(&local)
                        || local_set_overwritten(&instruction_ids, index, local, tree)
                    {
                        dead_stores.insert(instruction_id);
                    }
                }
                mir::Instruction::Store { pointer, .. } => {
                    let Some(pointer) = pointer.value() else {
                        continue;
                    };

                    if instruction_requires_exact_access(tree, instruction_id) {
                        continue;
                    }

                    if store_overwritten_in_block(&instruction_ids, index, pointer, tree, alias) {
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
            tree.set(block_id, new_block);
        }
    }

    true
}

/// Check whether a local.set is overwritten in the same block.
fn local_set_overwritten(
    instruction_ids: &[mir::LocalNodeId<mir::Instruction>],
    start: usize,
    local: mir::LocalNodeId<mir::Local>,
    tree: &mir::Tree,
) -> bool {
    // scan later instructions in the block
    for instruction_id in instruction_ids.iter().skip(start + 1).copied() {
        match tree.get(instruction_id) {
            // stop when a read observes the local
            mir::Instruction::LocalGet {
                local: get_local, ..
            } if get_local.local() == Some(local) => {
                return false;
            }
            // stop when a later write overwrites the local
            mir::Instruction::LocalSet {
                local: set_local, ..
            } if set_local.local() == Some(local) => {
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
    tree: &mir::Tree,
    alias: &AliasAnalysis,
) -> bool {
    // build a memory location for the stored pointer
    let location = MemoryLocation::from_ptr(pointer);

    // scan later instructions in the block
    for instruction_id in instruction_ids.iter().skip(start + 1).copied() {
        let instruction = tree.get(instruction_id);

        // stop when a later store overwrites this location
        if let mir::Instruction::Store {
            pointer: other_ptr, ..
        } = instruction
        {
            let Some(other_ptr) = other_ptr.value() else {
                return false;
            };
            let other_loc = MemoryLocation::from_ptr(other_ptr);
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
        let input = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: int32 = int.add value0, value1
    return value0
}
"#;

        // expected output
        let expected = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    return value0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Instructions used in return chain are preserved.
    #[test]
    fn test_preserve_used_chain() {
        // source test
        let input = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: int32 = int.add value0, value1
    return value2
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_unchanged(input);
    }

    /// Multiple unused instructions are all eliminated.
    #[test]
    fn test_eliminate_multiple_dead() {
        // only v0 is used
        let input = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: int32 = 3int32
    value3: int32 = int.add value1, value2
    value4: int32 = 4int32
    return value0
}
"#;

        // expected output
        let expected = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    return value0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Calls have side effects and are preserved even when result is unused.
    #[test]
    fn test_preserve_side_effect_call() {
        // source test
        let input = r#"
function test(): void {
entry0:
    value0: int32 = 1int32
    value1: int32 = call sideEffect(value0): (int32) -> int32
    return
}

function sideEffect(value0: int32): int32 {
entry0(value0: int32):
    return value0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_unchanged(input);
    }

    /// Dead code is eliminated from all blocks in the function.
    #[test]
    fn test_eliminate_dead_in_multiple_blocks() {
        // v2, v3, v4, v5 are all dead
        let input = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 1int32
    value2: int32 = 2int32
    branch value0, block1(), block2()

block1:
    value3: int32 = 3int32
    value4: int32 = 4int32
    return value1

block2:
    value5: int32 = 5int32
    return value1
}
"#;

        // expected output
        let expected = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 1int32
    branch value0, block1(), block2()

block1:
    return value1

block2:
    return value1
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Volatile loads are kept even when unused.
    #[test]
    fn test_preserve_volatile_load() {
        let input = r#"
function test(): void {
entry0:
    value0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value1: int32 = load value0
    return
}
"#;

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
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        test.run_pass(&DeadCodeEliminate);
        test.assert_unchanged(input);
    }

    /// Volatile stores are not removed even when overwritten.
    #[test]
    fn test_preserve_volatile_store_overwritten() {
        let input = r#"
function test(): void {
entry0:
    value0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value1: int32 = 1int32
    store value0, value1
    value2: int32 = 2int32
    store value0, value2
    return
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let store_id = test
            .entry_instructions(function_id)
            .into_iter()
            .find(|instruction_id| {
                matches!(
                    test.tree.get(*instruction_id),
                    mir::Instruction::Store { .. }
                )
            })
            .expect("missing store");

        test.insert_pointer_access_with_options(
            store_id,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        test.run_pass(&DeadCodeEliminate);
        test.assert_unchanged(input);
    }

    /// Values used in branch terminators are preserved.
    #[test]
    fn test_preserve_terminator_uses() {
        // source test
        let input = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: boolean = int.gt.s value0, value1
    branch value2, block1(), block2()

block1:
    return value0

block2:
    return value1
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_unchanged(input);
    }

    /// Transitive chains of dead code are all eliminated.
    #[test]
    fn test_eliminate_transitive_dead() {
        // v2, v3, v4 depend on each other but none used in return
        let input = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: int32 = int.add value0, value1
    value3: int32 = int.mul value2, value0
    value4: int32 = int.sub value3, value1
    return value0
}
"#;

        // expected output
        let expected = r#"
function test(): int32 {
entry0:
    value0: int32 = 1int32
    return value0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Values passed as block arguments are preserved.
    #[test]
    fn test_preserve_block_arguments() {
        // v3 is dead, but v1 and v2 are used as block arguments
        let input = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 1int32
    value2: int32 = 2int32
    value3: int32 = 3int32
    branch value0, block1(value1), block1(value2)

block1(value4: int32):
    return value4
}
"#;

        // expected output
        let expected = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 1int32
    value2: int32 = 2int32
    branch value0, block1(value1), block1(value2)

block1(value4: int32):
    return value4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// All instructions are eliminated when none are used by terminator.
    #[test]
    fn test_eliminate_all_instructions() {
        // source test
        let input = r#"
function test(): void {
entry0:
    value0: int32 = 1int32
    value1: int32 = 2int32
    value2: int32 = int.add value0, value1
    return
}
"#;

        // expected output
        let expected = r#"
function test(): void {
entry0:
    return
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Dead code inside loops is eliminated.
    #[test]
    fn test_eliminate_dead_in_loop() {
        // v3, v4, v5 are all dead (none of their results are used)
        let input = r#"
function test(): int32 {
entry0:
    value0: int32 = 0int32
    value1: int32 = 100int32
    jump block1()

block1:
    value2: boolean = int.lt.s value0, value1
    value3: int32 = 999int32
    branch value2, block2(), block3()

block2:
    value4: int32 = 1int32
    value5: int32 = int.mul value3, value3
    jump block1()

block3:
    return value0
}
"#;

        // expected output
        // v3, v4, v5 are all eliminated since their results are never used
        let expected = r#"
function test(): int32 {
entry0:
    value0: int32 = 0int32
    value1: int32 = 100int32
    jump block1()

block1:
    value2: boolean = int.lt.s value0, value1
    branch value2, block2(), block3()

block2:
    jump block1()

block3:
    return value0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Dead code in diamond CFG branches is eliminated.
    #[test]
    fn test_eliminate_dead_in_diamond() {
        // v2, v3, v5 are dead
        let input = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 1int32
    branch value0, block1(), block2()

block1:
    value2: int32 = 2int32
    value3: int32 = 3int32
    jump block3(value1)

block2:
    value4: int32 = 4int32
    value5: int32 = 5int32
    jump block3(value4)

block3(value6: int32):
    return value6
}
"#;

        // expected output
        let expected = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 1int32
    branch value0, block1(), block2()

block1:
    jump block3(value1)

block2:
    value4: int32 = 4int32
    jump block3(value4)

block3(value6: int32):
    return value6
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Multiple side-effect calls are all preserved.
    #[test]
    fn test_preserve_multiple_side_effect_calls() {
        // source test
        let input = r#"
function test(): void {
entry0:
    value0: int32 = 1int32
    value1: int32 = call sideEffect(value0): (int32) -> int32
    value2: int32 = call sideEffect(value0): (int32) -> int32
    value3: int32 = call sideEffect(value0): (int32) -> int32
    return
}

function sideEffect(value0: int32): int32 {
entry0(value0: int32):
    return value0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_unchanged(input);
    }

    /// Overwritten local sets are removed when not observed.
    #[test]
    fn test_remove_overwritten_local_set() {
        // source test
        let input = r#"
function test(value0: int32): int32 {
    local local0: int32, owned

entry0(value0: int32):
    local.set local0, value0
    value1: int32 = 3int32
    local.set local0, value1
    value2: int32 = local.get local0
    return value2
}
"#;

        // expected output
        let expected = r#"
function test(value0: int32): int32 {
    local local0: int32, owned

entry0(value0: int32):
    value1: int32 = 3int32
    local.set local0, value1
    value2: int32 = local.get local0
    return value2
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Local stores without any reads are removed.
    #[test]
    fn test_remove_unread_local_set() {
        // source test
        let input = r#"
function test(value0: int32): void {
    local local0: int32, owned

entry0(value0: int32):
    local.set local0, value0
    return
}
"#;

        // expected output
        let expected = r#"
function test(value0: int32): void {
    local local0: int32, owned

entry0(value0: int32):
    return
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Stores overwritten before any read are eliminated.
    #[test]
    fn test_remove_overwritten_store() {
        // source test
        let input = r#"
function test(): void {
entry0:
    value0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value1: int32 = 1int32
    value2: int32 = 2int32
    store value0, value1
    store value0, value2
    return
}
"#;

        // expected output
        let expected = r#"
function test(): void {
entry0:
    value0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value2: int32 = 2int32
    store value0, value2
    return
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_output(expected);
    }

    /// Stores read by a load are preserved.
    #[test]
    fn test_preserve_store_used_by_load() {
        // source test
        let input = r#"
function test(): int32 {
entry0:
    value0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    value1: int32 = 1int32
    store value0, value1
    value2: int32 = load value0
    value3: int32 = 2int32
    store value0, value3
    return value2
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadCodeEliminate);
        test.assert_unchanged(input);
    }
}
