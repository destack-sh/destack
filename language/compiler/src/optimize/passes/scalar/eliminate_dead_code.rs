use std::collections::VecDeque;

use crate::optimize::declare_pass;
use destack_core::FxIndexSet;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasTable, DefinitionTable, MemoryLocation, MemoryTable, Mutation,
    instruction_has_side_effects,
};

declare_pass! {
    /// Aggressive Dead Code Elimination (ADCE).
    ///
    /// ```mir
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1: int32 = 42
    ///     v2: int32 = add v0, v1
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
    #[pass(id = "eliminate-dead-code")]
    pub EliminateDeadCode,
    "Eliminate dead code"
}

impl FunctionPass for EliminateDeadCode {
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

        // build memory analyses for local dead store elimination
        let alias = analyses.alias(function, tree);
        let memory = analyses.memory(function, tree, accesses, effects);

        // run dead code elimination
        let changed = run_dead_code_elimination(function, tree, accesses, &alias, &memory);

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Core dead code elimination logic (shared by both pass implementations).
fn run_dead_code_elimination(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mir::AccessTable,
    alias: &AliasTable,
    memory: &MemoryTable,
) -> bool {
    // drop dead stores before liveness
    let mut changed = remove_dead_stores(function, tree, accesses, alias, memory);

    // snapshot value definitions before tracing liveness
    let definitions = DefinitionTable::build(function, tree);

    // seed live roots and worklist
    let mut live: FxIndexSet<mir::LocalNodeId<mir::Instruction>> = FxIndexSet::default();
    let mut worklist: VecDeque<mir::LocalNodeId<mir::Instruction>> = VecDeque::new();

    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
        let block = tree.get(block_id);

        // record side effecting instructions as live
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if (instruction_has_side_effects(instruction)
                || accesses.requires_exact_position(instruction_id, tree))
                && live.insert(instruction_id)
            {
                worklist.push_back(instruction_id);
            }
        }

        // record terminator uses as live
        let terminator = tree.get(block.terminator);
        for value in terminator.uses(tree) {
            if let Some(instruction_id) = definitions.instruction(value)
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
            if let Some(def_instruction_id) = definitions.instruction(value)
                && live.insert(def_instruction_id)
            {
                worklist.push_back(def_instruction_id);
            }
        }

        // mark external argument uses as live
        if let Some(args_slice) = instruction.argument_slice() {
            for &arg in tree.get_values(args_slice) {
                if let Some(def_instruction_id) = definitions.instruction(arg)
                    && live.insert(def_instruction_id)
                {
                    worklist.push_back(def_instruction_id);
                }
            }
        }
    }

    // remove dead instructions from blocks
    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
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
            function.replace_block_instructions(block_id, live_instructions, tree);
            changed = true;
        }
    }

    changed
}

/// Remove dead stores and return true when changes are made.
fn remove_dead_stores(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mir::AccessTable,
    alias: &AliasTable,
    memory: &MemoryTable,
) -> bool {
    // collect locals that are read anywhere
    let mut locals_read = FxIndexSet::default();
    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            if let mir::Instruction::LocalGet { local, .. } = tree.get(instruction_id) {
                locals_read.insert(*local);
            }
        }
    }

    // find dead store instructions
    let mut dead_stores = FxIndexSet::default();
    for &block_id in function.blocks() {
        let block = tree.get(block_id);
        let instruction_ids = block.instructions.clone();

        for (index, instruction_id) in instruction_ids.iter().copied().enumerate() {
            let instruction = tree.get(instruction_id);

            // classify stores and check for overwrites
            match instruction {
                mir::Instruction::LocalSet { local, .. } => {
                    let local = *local;

                    if !locals_read.contains(&local)
                        || local_set_overwritten(&instruction_ids, index, local, tree)
                    {
                        dead_stores.insert(instruction_id);
                    }
                }
                mir::Instruction::Store { pointer, .. } => {
                    let pointer = *pointer;

                    if accesses.requires_exact_position(instruction_id, tree) {
                        continue;
                    }

                    if store_overwritten_in_block(
                        &instruction_ids,
                        index,
                        pointer,
                        tree,
                        alias,
                        memory,
                    ) {
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
    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
        let block = tree.get(block_id);
        if block.instructions.iter().any(|id| dead_stores.contains(id)) {
            let mut instructions = block.instructions.clone();
            instructions.retain(|id| !dead_stores.contains(id));
            function.replace_block_instructions(block_id, instructions, tree);
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
    tree: &mir::Tree,
    alias: &AliasTable,
    memory: &MemoryTable,
) -> bool {
    // build a memory location for the stored pointer
    let location = MemoryLocation::from_address(pointer);

    // scan later instructions in the block
    for instruction_id in instruction_ids.iter().skip(start + 1).copied() {
        let instruction = tree.get(instruction_id);

        // stop when a later store overwrites this location
        if let mir::Instruction::Store {
            pointer: other_reference,
            ..
        } = instruction
        {
            let other_loc = MemoryLocation::from_address(*other_reference);
            let alias_result = alias.alias(&location, &other_loc);

            if alias_result.is_must_alias() || location.address == other_loc.address {
                return true;
            }
            if alias_result.may_alias() {
                return false;
            }
            continue;
        }

        // stop when any later memory effect can observe or clobber the store
        let touches_location = memory
            .instruction_effects(instruction_id)
            .any(|effect| effect.may_touch_location(&location, alias));
        if touches_location {
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
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = add v0, v1
    return v0
}
"#;

        // expected output
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    return v0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Instructions used in return chain are preserved.
    #[test]
    fn test_preserve_used_chain() {
        // source test
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = add v0, v1
    return v2
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_unchanged(input);
    }

    /// Multiple unused instructions are all eliminated.
    #[test]
    fn test_eliminate_multiple_dead() {
        // only v0 is used
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = 3
    v3: int32 = add v1, v2
    v4: int32 = 4
    return v0
}
"#;

        // expected output
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    return v0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Calls have side effects and are preserved even when result is unused.
    #[test]
    fn test_preserve_side_effect_call() {
        // source test
        let input = r#"
function test(): void {
entry:
    v0: int32 = 1
    v1: int32 = call sideEffect(v0): (int32) => int32
    return
}

function sideEffect(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_unchanged(input);
    }

    /// Dead code is eliminated from all blocks in the function.
    #[test]
    fn test_eliminate_dead_in_multiple_blocks() {
        // v2, v3, v4, v5 are all dead
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    v2: int32 = 2
    branch v0 => b1 | b2

b1:
    v3: int32 = 3
    v4: int32 = 4
    return v1

b2:
    v5: int32 = 5
    return v1
}
"#;

        // expected output
        let expected = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    branch v0 => b1 | b2

b1:
    return v1

b2:
    return v1
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Volatile loads are kept even when unused.
    #[test]
    fn test_preserve_volatile_load() {
        let input = r#"
function test(): void {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
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
                    test.optimized.tree.get(*instruction_id),
                    mir::Instruction::Load { .. }
                )
            })
            .expect("missing load");

        test.insert_pointer_access_with_options(
            load_id,
            mir::MemoryOperation::Read,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        test.run_pass(&EliminateDeadCode);
        test.assert_unchanged(input);
    }

    /// Volatile stores are not removed even when overwritten.
    #[test]
    fn test_preserve_volatile_store_overwritten() {
        let input = r#"
function test(): void {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 1
    store v0, v1
    v2: int32 = 2
    store v0, v2
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
                    test.optimized.tree.get(*instruction_id),
                    mir::Instruction::Store { .. }
                )
            })
            .expect("missing store");

        test.insert_pointer_access_with_options(
            store_id,
            mir::MemoryOperation::Write,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        test.run_pass(&EliminateDeadCode);
        test.assert_unchanged(input);
    }

    /// Values used in branch terminators are preserved.
    #[test]
    fn test_preserve_terminator_uses() {
        // source test
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: boolean = gt v0, v1
    branch v2 => b1 | b2

b1:
    return v0

b2:
    return v1
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_unchanged(input);
    }

    /// Transitive chains of dead code are all eliminated.
    #[test]
    fn test_eliminate_transitive_dead() {
        // v2, v3, v4 depend on each other but none used in return
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = add v0, v1
    v3: int32 = mul v2, v0
    v4: int32 = sub v3, v1
    return v0
}
"#;

        // expected output
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 1
    return v0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Values passed as block arguments are preserved.
    #[test]
    fn test_preserve_block_arguments() {
        // v3 is dead, but v1 and v2 are used as block arguments
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = 3
    branch v0 => b1(v1) | b1(v2)

b1(v4: int32):
    return v4
}
"#;

        // expected output
        let expected = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    v2: int32 = 2
    branch v0 => b1(v1) | b1(v2)

b1(v4: int32):
    return v4
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// All instructions are eliminated when none are used by terminator.
    #[test]
    fn test_eliminate_all_instructions() {
        // source test
        let input = r#"
function test(): void {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = add v0, v1
    return
}
"#;

        // expected output
        let expected = r#"
function test(): void {
entry:
    return
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Dead code inside loops is eliminated.
    #[test]
    fn test_eliminate_dead_in_loop() {
        // v3, v4, v5 are all dead (none of their results are used)
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 0
    v1: int32 = 100
    jump b1

b1:
    v2: boolean = lt v0, v1
    v3: int32 = 999
    branch v2 => b2 | b3

b2:
    v4: int32 = 1
    v5: int32 = mul v3, v3
    jump b1

b3:
    return v0
}
"#;

        // expected output
        // v3, v4, v5 are all eliminated since their results are never used
        let expected = r#"
function test(): int32 {
entry:
    v0: int32 = 0
    v1: int32 = 100
    jump b1

b1:
    v2: boolean = lt v0, v1
    branch v2 => b2 | b3

b2:
    jump b1

b3:
    return v0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Dead code in diamond CFG branches is eliminated.
    #[test]
    fn test_eliminate_dead_in_diamond() {
        // v2, v3, v5 are dead
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    branch v0 => b1 | b2

b1:
    v2: int32 = 2
    v3: int32 = 3
    jump b3(v1)

b2:
    v4: int32 = 4
    v5: int32 = 5
    jump b3(v4)

b3(v6: int32):
    return v6
}
"#;

        // expected output
        let expected = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    branch v0 => b1 | b2

b1:
    jump b3(v1)

b2:
    v4: int32 = 4
    jump b3(v4)

b3(v6: int32):
    return v6
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Multiple side-effect calls are all preserved.
    #[test]
    fn test_preserve_multiple_side_effect_calls() {
        // source test
        let input = r#"
function test(): void {
entry:
    v0: int32 = 1
    v1: int32 = call sideEffect(v0): (int32) => int32
    v2: int32 = call sideEffect(v0): (int32) => int32
    v3: int32 = call sideEffect(v0): (int32) => int32
    return
}

function sideEffect(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_unchanged(input);
    }

    /// Overwritten local sets are removed when not observed.
    #[test]
    fn test_remove_overwritten_local_set() {
        // source test
        let input = r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = 3
    local.set l0, v1
    v2: int32 = local.get l0
    return v2
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    v1: int32 = 3
    local.set l0, v1
    v2: int32 = local.get l0
    return v2
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Local stores without any reads are removed.
    #[test]
    fn test_remove_unread_local_set() {
        // source test
        let input = r#"
function test(v0: int32): void {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    return
}
"#;

        // expected output
        let expected = r#"
function test(v0: int32): void {
    local l0: int32

entry(v0: int32):
    return
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Stores overwritten before any read are eliminated.
    #[test]
    fn test_remove_overwritten_store() {
        // source test
        let input = r#"
function test(): void {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 1
    v2: int32 = 2
    store v0, v1
    store v0, v2
    return
}
"#;

        // expected output
        let expected = r#"
function test(): void {
    local l0: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v2: int32 = 2
    store v0, v2
    return
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_output(expected);
    }

    /// Stores read by a load are preserved.
    #[test]
    fn test_preserve_store_used_by_load() {
        // source test
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 1
    store v0, v1
    v2: int32 = load v0
    v3: int32 = 2
    store v0, v3
    return v2
}
"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadCode);
        test.assert_unchanged(input);
    }
}
