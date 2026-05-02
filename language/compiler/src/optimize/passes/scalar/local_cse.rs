use std::collections::{HashMap, HashSet};

use crate::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::AliasAnalysis;
use crate::optimize::common::{
    MemoryLocation, instruction_may_affect_memory, instruction_requires_exact_access,
};
use crate::optimize::{
    AnalysisPreservation, ExpressionKey, FunctionPass, PipelineContext,
    expression_key_from_instruction, expression_key_substitute, instruction_has_side_effects,
    instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    resolve_substitution_chains, terminator_substitute_uses,
};

declare_pass! {
    /// Local Common Subexpression Elimination.
    ///
    /// Eliminates redundant computations within a single basic block by tracking
    /// expressions and replacing duplicates with the original result. This is a
    /// lightweight, fast pass that runs in O(n) per block.
    ///
    /// For cross-block elimination, see GVN (Global Value Numbering).
    ///
    /// ```mir
    /// function before(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = int.add v0, v1
    ///     v3 = int.add v0, v1
    ///     v4 = int.add v2, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = int.add v0, v1
    ///     v4 = int.add v2, v2
    ///     return v4
    /// }
    /// ```
    #[pass(id = "local-cse")]
    pub LocalCse,
    "Eliminate redundant expressions within blocks"
}

impl FunctionPass for LocalCse {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // build alias analysis
        let analyses = ctx.function_analyses(function, tree);
        let alias = analyses.get::<AliasAnalysis>();

        // run local CSE
        let changed = run_local_cse(function, tree, &alias);

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "LocalCse"
    }

    fn id(&self) -> &'static str {
        "local-cse"
    }
}

/// Run local CSE on all blocks in a function.
fn run_local_cse(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    alias: &AliasAnalysis,
) -> bool {
    // track whether any block changes
    let mut changed = false;

    // run local CSE per block
    for &block_id in &function.blocks {
        changed |= eliminate_common_subexpressions_in_block(block_id, tree, alias);
    }
    changed
}

/// Eliminate common subexpressions within a single basic block.
///
/// Returns true if any changes were made.
fn eliminate_common_subexpressions_in_block(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mut mir::Tree,
    alias: &AliasAnalysis,
) -> bool {
    // expression table: key -> defining value
    let mut expression_table: HashMap<ExpressionKey, mir::Value> = HashMap::new();

    // substitutions to apply: old value -> new value
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();

    // instructions to remove (now redundant)
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();

    // track redundant loads within the block
    let mut load_table: Vec<LoadEntry> = Vec::new();

    // track local values
    let mut local_values: HashMap<mir::LocalNodeId<mir::Local>, mir::Value> = HashMap::new();

    // scan instructions for redundant expressions
    let block = tree.get(block_id);
    let instruction_ids: Vec<_> = block.instructions.clone();

    // scan instructions in test order
    for instruction_id in instruction_ids {
        let instruction = tree.get(instruction_id);

        // treat exact accesses as barriers for load forwarding
        if instruction_requires_exact_access(tree, instruction_id) {
            load_table.clear();
            continue;
        }

        // handle local get forwarding
        if let mir::Instruction::LocalGet { destination, local } = instruction {
            let Some(destination) = destination.value() else {
                continue;
            };
            let Some(local) = local.local() else {
                continue;
            };

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
            let Some(local) = local.local() else {
                continue;
            };
            let Some(value) = value.value() else {
                continue;
            };

            local_values.insert(local, value);
        }

        // handle load forwarding
        if let mir::Instruction::Load {
            destination,
            pointer,
            ..
        } = instruction
        {
            let Some(destination) = destination.value() else {
                continue;
            };
            let Some(pointer) = pointer.value() else {
                continue;
            };

            let location = MemoryLocation::from_ptr(pointer);
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
        if instruction_may_clobber_memory(instruction, instruction_id, &load_table, alias) {
            load_table = prune_load_table(instruction_id, &load_table, alias);
        }

        // skip instructions with side effects (don't CSE across side effects)
        if instruction_has_side_effects(instruction) {
            continue;
        }

        // try to get an expression key for this instruction
        let Some(key) = expression_key_from_instruction(instruction, tree) else {
            continue;
        };

        // get the destination value
        let Some(destination) = instruction
            .destination()
            .and_then(|destination| destination.value())
        else {
            continue;
        };

        // apply existing substitutions to the key (transitively)
        let key = expression_key_substitute(key, &substitutions);

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
            tree.replace(instruction_id, new_instruction);
            remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
        }
    }

    // apply substitutions to terminator
    let block = tree.get(block_id);
    let terminator_id = block.terminator;
    let terminator = tree.get(block.terminator).clone();
    let new_terminator = terminator_substitute_uses(&terminator, &substitutions);

    // update block: remove redundant instructions and update terminator
    let mut new_block = block.clone();
    new_block.instructions.retain(|id| !to_remove.contains(id));
    tree.replace(block_id, new_block);
    tree.replace(terminator_id, new_terminator);

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
    alias: &AliasAnalysis,
) -> Option<mir::Value> {
    // scan load table from most recent to oldest
    for entry in load_table.iter().rev() {
        // treat identical pointers as a must alias
        if entry.location.ptr == location.ptr {
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
    instruction: &mir::Instruction,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    load_table: &[LoadEntry],
    alias: &AliasAnalysis,
) -> bool {
    // skip instructions that do not touch memory
    if !instruction_may_affect_memory(instruction) {
        return false;
    }

    // check if any tracked load is clobbered
    for entry in load_table {
        if alias.may_clobber(instruction_id, &entry.location) {
            return true;
        }
    }

    false
}

/// Remove any load entries clobbered by an instruction.
fn prune_load_table(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    load_table: &[LoadEntry],
    alias: &AliasAnalysis,
) -> Vec<LoadEntry> {
    // retain only loads not clobbered by the instruction
    load_table
        .iter()
        .filter(|entry| !alias.may_clobber(instruction_id, &entry.location))
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
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v2, v2
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Commutative operands are recognized as equivalent (v0 + v1 == v1 + v0).
    #[test]
    fn test_eliminate_commutative() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v1, v0
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v2, v2
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Transitive chains of redundant expressions are all eliminated.
    #[test]
    fn test_eliminate_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v0, v1
    v4: int32 = int.mul v2, v2
    v5: int32 = int.mul v3, v3
    v6: int32 = int.add v4, v5
    return v6
}"#;
        // v3 -> v2, then v5 = int.mul v2, v2 = v4
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.mul v2, v2
    v4: int32 = int.add v3, v3
    return v4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Constants are not CSE'd by this pass (handled by constant folding).
    #[test]
    fn test_skip_constants() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    v1: int32 = 42int32
    v2: int32 = int.add v0, v1
    return v2
}"#;
        // should be unchanged: constant CSE is not done by this pass
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_unchanged(input);
    }

    /// Expressions are not CSE'd across basic blocks (that's GVN's job).
    #[test]
    fn test_skip_cross_block_expressions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v2, v3
    return v4
}"#;
        // should be unchanged: v3 is in a different block
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_unchanged(input);
    }

    /// Non-commutative operations with swapped operands are distinct.
    #[test]
    fn test_distinguish_non_commutative_operands() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.sub v0, v1
    v3: int32 = int.sub v1, v0
    v4: int32 = int.add v2, v3
    return v4
}"#;
        // should be unchanged: v0 - v1 != v1 - v0
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_unchanged(input);
    }

    /// Unary operations are properly CSE'd.
    #[test]
    fn test_eliminate_unary() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.negate v0
    v2: int32 = int.negate v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.negate v0
    v2: int32 = int.add v1, v1
    return v2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Multiple redundant expressions in sequence are all eliminated.
    #[test]
    fn test_eliminate_multiple() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v0, v1
    v5: int32 = int.add v0, v1
    v6: int32 = int.add v2, v5
    return v6
}"#;
        // all int.add v0, v1 collapse to v2
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v2, v2
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Field accesses with same base and index are CSE'd.
    #[test]
    fn test_eliminate_field_get() {
        let input = r#"
function test(v0: (int32, int32)): int32 {
b0(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    v2: int32 = field.get v0, 0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(v0: (int32, int32)): int32 {
b0(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    v2: int32 = int.add v1, v1
    return v2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Redundant loads in a block are eliminated when not clobbered.
    #[test]
    fn test_eliminate_redundant_loads() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Loads are not CSE'd across clobbering stores.
    #[test]
    fn test_preserve_loads_after_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = load v0
    v4: int32 = int.add v1, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(input);
    }

    /// Different field indices are not CSE'd.
    #[test]
    fn test_distinguish_different_field_indices() {
        let input = r#"
function test(v0: (int32, int32)): int32 {
b0(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    v2: int32 = field.get v0, 1
    v3: int32 = int.add v1, v2
    return v3
}"#;
        // should be unchanged: different field indices
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_unchanged(input);
    }

    /// Element accesses with same base and index are CSE'd.
    #[test]
    fn test_eliminate_element_get() {
        let input = r#"
function test(v0: int32[10], v1: int64): int32 {
b0(v0: int32[10], v1: int64):
    v2: int32 = element.get v0, v1
    v3: int32 = element.get v0, v1
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(v0: int32[10], v1: int64): int32 {
b0(v0: int32[10], v1: int64):
    v2: int32 = element.get v0, v1
    v3: int32 = int.add v2, v2
    return v3
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Casts to identical types are CSE'd.
    #[test]
    fn test_eliminate_casts() {
        let input = r#"
function test(v0: int32): int64 {
b0(v0: int32):
    v1: int64 = cast.extend.s v0 -> int64
    v2: int64 = cast.extend.s v0 -> int64
    v3: int64 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(v0: int32): int64 {
b0(v0: int32):
    v1: int64 = cast.extend.s v0 -> int64
    v2: int64 = int.add v1, v1
    return v2
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Unique expressions are preserved unchanged.
    #[test]
    fn test_preserve_unique_expressions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.sub v0, v1
    v4: int32 = int.mul v2, v3
    return v4
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_unchanged(input);
    }

    /// Substitutions propagate to terminator.
    #[test]
    fn test_propagate_substitutions_to_terminator() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v0, v1
    branch v2, b1(v4), b2(v4)
b1(v5: int32):
    return v5
b2(v6: int32):
    return v6
}"#;
        // v4 -> v3, and the branch should use v3
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1(v3), b2(v3)
b1(v4: int32):
    return v4
b2(v5: int32):
    return v5
}"#;
        let mut test = TestProgram::new(input);
        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }

    /// Exact access only blocks forwarding for the accessed location.
    #[test]
    fn test_load_forwarding_respects_exact_access() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let block = test.tree.get(function.blocks[0]);
        let volatile_id = block.instructions[1];

        test.insert_pointer_access_with_options(
            volatile_id,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );

        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    v2: int32 = load v0
    return v2
}"#;

        test.run_pass(&LocalCse);
        test.assert_output(expected);
    }
}
