use destack_core::{FxIndexMap, FxIndexSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasTable, MemoryLocation, MemoryTable, Mutation, PureExpression,
    instruction_has_side_effects, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, resolve_substitution_chains, terminator_substitute_uses,
};

declare_pass! {
    /// Local Common Subexpression Elimination.
    ///
    /// ```mir
    /// function before(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2: int32 = add v0, v1
    ///     v3: int32 = add v0, v1
    ///     v4: int32 = add v2, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2: int32 = add v0, v1
    ///     v4: int32 = add v2, v2
    ///     return v4
    /// }
    /// ```
    #[pass(id = "eliminate-local-common-subexpressions")]
    pub EliminateLocalCommonSubexpressions,
    "Eliminate redundant expressions within blocks"
}

impl FunctionPass for EliminateLocalCommonSubexpressions {
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

        // build memory analyses
        let alias = analyses.alias(function, tree);
        let memory = analyses.memory(function, tree, accesses, effects);

        // run local CSE
        let changed =
            run_eliminate_local_common_subexpressions(function, tree, accesses, &alias, &memory);

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Run local CSE on all blocks in a function.
fn run_eliminate_local_common_subexpressions(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    alias: &AliasTable,
    memory: &MemoryTable,
) -> bool {
    // track whether any block changes
    let mut changed = false;

    // run local CSE per block
    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
        changed |= eliminate_common_subexpressions_in_block(
            function, block_id, tree, accesses, alias, memory,
        );
    }
    changed
}

/// Eliminate common subexpressions within a single basic block.
///
/// Returns true if any changes were made.
fn eliminate_common_subexpressions_in_block(
    function: &mut mir::Function,
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    alias: &AliasTable,
    memory: &MemoryTable,
) -> bool {
    // expression table: key -> defining value
    let mut expression_table: FxIndexMap<PureExpression, mir::Value> = FxIndexMap::default();

    // substitutions to apply: old value -> new value
    let mut substitutions: FxIndexMap<mir::Value, mir::Value> = FxIndexMap::default();

    // instructions to remove (now redundant)
    let mut to_remove: FxIndexSet<mir::LocalNodeId<mir::Instruction>> = FxIndexSet::default();

    // track redundant loads within the block
    let mut load_table: Vec<LoadEntry> = Vec::new();

    // track local values
    let mut local_values: FxIndexMap<mir::LocalNodeId<mir::Local>, mir::Value> =
        FxIndexMap::default();

    // scan instructions for redundant expressions
    let block = tree.get(block_id).clone();
    let instruction_ids: Vec<_> = block.instructions.clone();

    // scan instructions in test order
    for instruction_id in instruction_ids {
        let instruction = tree.get(instruction_id);

        // treat exact accesses as barriers for load forwarding
        if accesses.requires_exact_position(instruction_id, tree) {
            load_table.clear();
            continue;
        }

        // handle local get forwarding
        if let mir::Instruction::LocalGet { destination, local } = instruction {
            let destination = *destination;
            let local = *local;

            if let Some(existing) = local_values.get(&local) {
                substitutions.insert(destination, *existing);
                to_remove.insert(instruction_id);
            } else {
                local_values.insert(local, destination);
            }
            continue;
        }

        // update local state on set
        if let mir::Instruction::LocalSet { local, value } = instruction {
            let local = *local;
            let value = *value;

            local_values.insert(local, value);
        }

        // handle load forwarding
        if let mir::Instruction::Load {
            destination,
            pointer,
            ..
        } = instruction
        {
            let destination = *destination;
            let pointer = *pointer;

            let location = MemoryLocation::from_address(pointer);
            if let Some(existing) = find_load_redundancy(&load_table, &location, alias) {
                substitutions.insert(destination, existing);
                to_remove.insert(instruction_id);
            } else {
                load_table.push(LoadEntry {
                    location,
                    value: destination,
                });
            }
            continue;
        }

        // invalidate load entries on memory clobbers
        if instruction_may_clobber_memory(instruction_id, &load_table, alias, memory) {
            load_table = prune_load_table(instruction_id, &load_table, alias, memory);
        }

        // skip instructions with side effects (don't CSE across side effects)
        if instruction_has_side_effects(instruction) {
            continue;
        }

        // try to get an expression key for this instruction
        let Some(key) = PureExpression::from_instruction(instruction) else {
            continue;
        };

        // get the destination value
        let Some(destination) = instruction.destination() else {
            continue;
        };

        // apply existing substitutions to the key (transitively)
        let key = key.substitute(&substitutions);

        // check if we've seen this expression before
        if let Some(&existing_value) = expression_table.get(&key) {
            // found a match: this instruction is redundant
            substitutions.insert(destination, existing_value);
            to_remove.insert(instruction_id);
        }
        // otherwise record this as a new expression
        else {
            expression_table.insert(key, destination);
        }
    }

    // nothing to do if no redundancies found
    if to_remove.is_empty() {
        return false;
    }

    // resolve transitive substitution chains (v4 -> v2 -> v0 becomes v4 -> v0)
    let substitutions = resolve_substitution_chains(substitutions);

    // apply substitutions to remaining instructions
    let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();
    for instruction_id in instruction_ids {
        // skip instructions slated for removal
        if to_remove.contains(&instruction_id) {
            continue;
        }

        // rewrite instruction operands
        let instruction = tree.get(instruction_id).clone();
        let new_instruction =
            instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);
        if new_instruction != instruction {
            tree.set(instruction_id, new_instruction);
            remap_instruction_memory_accesses(accesses, instruction_id, &substitutions);
        }
    }

    // apply substitutions to terminator
    let terminator_id = tree.get(block_id).terminator;
    let terminator = tree.get(terminator_id).clone();
    let new_terminator = terminator_substitute_uses(tree, &terminator, &substitutions);

    // update block: remove redundant instructions and update terminator
    let mut instructions = tree.get(block_id).instructions.clone();
    instructions.retain(|id| !to_remove.contains(id));
    function.replace_block_instructions(block_id, instructions, tree);
    tree.set(terminator_id, new_terminator);

    true
}

/// Load entry in the local CSE table.
#[derive(Clone)]
struct LoadEntry {
    /// Memory location for the load.
    location: MemoryLocation,
    /// The value produced by the load.
    value: mir::Value,
}

/// Find a redundant load using alias analysis.
fn find_load_redundancy(
    load_table: &[LoadEntry],
    location: &MemoryLocation,
    alias: &AliasTable,
) -> Option<mir::Value> {
    // scan load table from most recent to oldest
    for entry in load_table.iter().rev() {
        // treat identical pointers as a must alias
        if entry.location.address == location.address {
            return Some(entry.value);
        }

        // consult alias analysis for memory overlap
        let result = alias.alias(&entry.location, location);
        if result.is_no_alias() {
            continue;
        }
        if result.is_must_alias() {
            return Some(entry.value);
        }
        return None;
    }

    None
}

/// Check if an instruction may clobber any tracked load.
fn instruction_may_clobber_memory(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    load_table: &[LoadEntry],
    alias: &AliasTable,
    memory: &MemoryTable,
) -> bool {
    // check if any tracked load is clobbered
    for entry in load_table {
        let is_clobbered = memory
            .instruction_effects(instruction_id)
            .any(|effect| effect.clobbers_location(&entry.location, alias));
        if is_clobbered {
            return true;
        }
    }

    false
}

/// Remove any load entries clobbered by an instruction.
fn prune_load_table(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    load_table: &[LoadEntry],
    alias: &AliasTable,
    memory: &MemoryTable,
) -> Vec<LoadEntry> {
    // retain only loads not clobbered by the instruction
    load_table
        .iter()
        .filter(|entry| {
            memory
                .instruction_effects(instruction_id)
                .all(|effect| !effect.clobbers_location(&entry.location, alias))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Identical expressions in the same block are deduplicated.
    #[test]
    fn test_eliminate_simple_redundancy() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v3: int32 = add v0, v1
    v4: int32 = add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v4: int32 = add v2, v2
    return v4
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Commutative operands are recognized as equivalent (v0 + v1 == v1 + v0).
    #[test]
    fn test_eliminate_commutative() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v3: int32 = add v1, v0
    v4: int32 = add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v4: int32 = add v2, v2
    return v4
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Transitive chains of redundant expressions are all eliminated.
    #[test]
    fn test_eliminate_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v3: int32 = add v0, v1
    v4: int32 = mul v2, v2
    v5: int32 = mul v3, v3
    v6: int32 = add v4, v5
    return v6
}
"#;
        // v3 -> v2, then v5 = mul v2, v2 = v4
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v4: int32 = mul v2, v2
    v6: int32 = add v4, v4
    return v6
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Constants are not CSE'd by this pass (handled by constant folding).
    #[test]
    fn test_skip_constants() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    v1: int32 = 42
    v2: int32 = add v0, v1
    return v2
}
"#;
        // should be unchanged: constant CSE is not done by this pass
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_unchanged(input);
    }

    /// Expressions are not CSE'd across basic blocks (that's redundant-expression elimination's job).
    #[test]
    fn test_skip_cross_block_expressions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    jump b1

b1:
    v3: int32 = add v0, v1
    v4: int32 = add v2, v3
    return v4
}
"#;
        // should be unchanged: v3 is in a different block
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_unchanged(input);
    }

    /// Non-commutative operations with swapped operands are distinct.
    #[test]
    fn test_distinguish_non_commutative_operands() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = sub v0, v1
    v3: int32 = sub v1, v0
    v4: int32 = add v2, v3
    return v4
}
"#;
        // should be unchanged: v0 - v1 != v1 - v0
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_unchanged(input);
    }

    /// Unary operations are properly CSE'd.
    #[test]
    fn test_eliminate_unary() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = negate v0
    v2: int32 = negate v0
    v3: int32 = add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = negate v0
    v3: int32 = add v1, v1
    return v3
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Multiple redundant expressions in sequence are all eliminated.
    #[test]
    fn test_eliminate_multiple() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v3: int32 = add v0, v1
    v4: int32 = add v0, v1
    v5: int32 = add v0, v1
    v6: int32 = add v2, v5
    return v6
}
"#;
        // all add v0, v1 collapse to v2
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v6: int32 = add v2, v2
    return v6
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Field accesses with same base and index are CSE'd.
    #[test]
    fn test_eliminate_field_get() {
        let input = r#"
function test(v0: (int32, int32)): int32 {
entry(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    v2: int32 = field.get v0, 0
    v3: int32 = add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: (int32, int32)): int32 {
entry(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    v3: int32 = add v1, v1
    return v3
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Redundant loads in a block are eliminated when not clobbered.
    #[test]
    fn test_eliminate_redundant_loads() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    v3: int32 = add v1, v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Loads are not CSE'd across clobbering stores.
    #[test]
    fn test_preserve_loads_after_store() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    v2: int32 = 1
    store v0, v2
    v3: int32 = load v0
    v4: int32 = add v1, v3
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(input);
    }

    /// Different field indices are not CSE'd.
    #[test]
    fn test_distinguish_different_field_indices() {
        let input = r#"
function test(v0: (int32, int32)): int32 {
entry(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    v2: int32 = field.get v0, 1
    v3: int32 = add v1, v2
    return v3
}
"#;
        // should be unchanged: different field indices
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_unchanged(input);
    }

    /// Repeated element gets with the same base and index are eliminated.
    #[test]
    fn test_eliminate_repeated_element_get() {
        let input = r#"
function test(v0: [int32; 10], v1: int64): int32 {
entry(v0: [int32; 10], v1: int64):
    v2: int32 = element.get v0, 0
    v3: int32 = element.get v0, 0
    v4: int32 = add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: [int32; 10], v1: int64): int32 {
entry(v0: [int32; 10], v1: int64):
    v2: int32 = element.get v0, 0
    v4: int32 = add v2, v2
    return v4
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Casts to identical types are CSE'd.
    #[test]
    fn test_eliminate_casts() {
        let input = r#"
function test(v0: int32): int64 {
entry(v0: int32):
    v1: int64 = cast.extend.s v0 -> int64
    v2: int64 = cast.extend.s v0 -> int64
    v3: int64 = add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: int32): int64 {
entry(v0: int32):
    v1: int64 = cast.extend.s v0 -> int64
    v3: int64 = add v1, v1
    return v3
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Unique expressions are preserved unchanged.
    #[test]
    fn test_preserve_unique_expressions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    v3: int32 = sub v0, v1
    v4: int32 = mul v2, v3
    return v4
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_unchanged(input);
    }

    /// Substitutions propagate to terminator.
    #[test]
    fn test_propagate_substitutions_to_terminator() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = add v0, v1
    v4: int32 = add v0, v1
    branch v2 => b1(v4) | b2(v4)

b1(v5: int32):
    return v5

b2(v6: int32):
    return v6
}
"#;
        // v4 -> v3, and the branch should use v3
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = add v0, v1
    branch v2 => b1(v3) | b2(v3)

b1(v5: int32):
    return v5

b2(v6: int32):
    return v6
}
"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }

    /// Exact access only blocks forwarding for the accessed location.
    #[test]
    fn test_load_forwarding_respects_exact_access() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = load v0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.optimized.tree.get(function_id);
        let block = test.optimized.tree.get(function.block(0));
        let volatile_id = block.instructions[1];

        test.insert_pointer_access_with_options(
            volatile_id,
            mir::MemoryOperation::Read,
            mir::Value::new(0),
            Some(4),
            true,
            None,
        );

        let expected = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    v2: int32 = load v0
    return v2
}
"#;

        test.run_pass(&EliminateLocalCommonSubexpressions);
        test.assert_output(expected);
    }
}
