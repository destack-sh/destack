use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::AnalysisKind;
use crate::optimize::{
    AnalysisPreservation, ExpressionKey, FunctionPass, OptimizationContext, Pass, PassMetadata,
    expression_key_from_instruction, expression_key_substitute, instruction_has_side_effects,
    instruction_substitute_uses, resolve_substitution_chains, terminator_substitute_uses,
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
    /// function @before(v0: i32, v1: i32) -> i32 {
    /// block0(v0: i32, v1: i32):
    ///     v2 = iadd v0, v1
    ///     v3 = iadd v0, v1
    ///     v4 = iadd v2, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32, v1: i32) -> i32 {
    /// block0(v0: i32, v1: i32):
    ///     v2 = iadd v0, v1
    ///     v4 = iadd v2, v2
    ///     return v4
    /// }
    /// ```
    #[pass(id = "local-cse")]
    pub LocalCse,
    "Eliminate redundant expressions within blocks"
}

impl Pass for LocalCse {
    fn metadata(&self) -> &'static PassMetadata {
        LocalCse::metadata()
    }
}

impl FunctionPass for LocalCse {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let mut changed = false;

        for &block_id in &function.blocks {
            let block_changed = eliminate_common_subexpressions_in_block(block_id, tree);
            changed |= block_changed;
        }

        if changed {
            AnalysisPreservation::Some(vec![AnalysisKind::ControlFlowGraph])
        } else {
            AnalysisPreservation::all()
        }
    }
}

/// Eliminate common subexpressions within a single basic block.
///
/// Returns true if any changes were made.
fn eliminate_common_subexpressions_in_block(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mut mir::NodeTree,
) -> bool {
    // expression table: key -> defining value
    let mut expression_table: HashMap<ExpressionKey, mir::Value> = HashMap::new();

    // substitutions to apply: old value -> new value
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();

    // instructions to remove (now redundant)
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();

    // scan instructions for redundant expressions
    let block = tree.get(block_id);
    let instruction_ids: Vec<_> = block.instructions.clone();

    for instruction_id in instruction_ids {
        let instruction = tree.get(instruction_id);

        // skip instructions with side effects (conservative: don't CSE across side effects)
        if instruction_has_side_effects(instruction) {
            continue;
        }

        // try to get an expression key for this instruction
        let Some(key) = expression_key_from_instruction(instruction, tree) else {
            continue;
        };

        // get the destination value
        let Some(destination) = instruction.destination() else {
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
        if to_remove.contains(&instruction_id) {
            continue;
        }

        let instruction = tree.get(instruction_id);
        let new_instruction = instruction_substitute_uses(instruction, &substitutions);
        if new_instruction != *instruction {
            tree.replace(instruction_id, new_instruction);
        }
    }

    // apply substitutions to terminator
    let block = tree.get(block_id);
    let new_terminator = terminator_substitute_uses(&block.terminator, &substitutions);

    // update block: remove redundant instructions and update terminator
    let mut new_block = block.clone();
    new_block.instructions.retain(|id| !to_remove.contains(id));
    new_block.terminator = new_terminator;
    tree.replace(block_id, new_block);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Identical expressions in the same block are deduplicated.
    #[test]
    fn test_eliminate_simple_redundancy() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = iadd v0, v1
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v4 = iadd v2, v2
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Commutative operands are recognized as equivalent (v0 + v1 == v1 + v0).
    #[test]
    fn test_eliminate_commutative() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = iadd v1, v0
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v4 = iadd v2, v2
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Transitive chains of redundant expressions are all eliminated.
    #[test]
    fn test_eliminate_chain() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = iadd v0, v1
    v4 = imul v2, v2
    v5 = imul v3, v3
    v6 = iadd v4, v5
    return v6
}"#;
        // v3 -> v2, then v5 = imul v2, v2 = v4
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v4 = imul v2, v2
    v6 = iadd v4, v4
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Constants are not CSE'd by this pass (handled by constant folding).
    #[test]
    fn test_skip_constants() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    v1 = iconst 42i32
    v2 = iadd v0, v1
    return v2
}"#;
        // should be unchanged: constant CSE is not done by this pass
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_unchanged(input);
    }

    /// Expressions are not CSE'd across basic blocks (that's GVN's job).
    #[test]
    fn test_skip_cross_block_expressions() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    v3 = iadd v0, v1
    v4 = iadd v2, v3
    return v4
}"#;
        // should be unchanged: v3 is in a different block
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_unchanged(input);
    }

    /// Non-commutative operations with swapped operands are distinct.
    #[test]
    fn test_distinguish_non_commutative_operands() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = isub v0, v1
    v3 = isub v1, v0
    v4 = iadd v2, v3
    return v4
}"#;
        // should be unchanged: v0 - v1 != v1 - v0
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_unchanged(input);
    }

    /// Unary operations are properly CSE'd.
    #[test]
    fn test_eliminate_unary() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = ineg v0
    v2 = ineg v0
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = ineg v0
    v3 = iadd v1, v1
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Multiple redundant expressions in sequence are all eliminated.
    #[test]
    fn test_eliminate_multiple() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = iadd v0, v1
    v4 = iadd v0, v1
    v5 = iadd v0, v1
    v6 = iadd v2, v5
    return v6
}"#;
        // all iadd v0, v1 collapse to v2
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v6 = iadd v2, v2
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Field accesses with same base and index are CSE'd.
    #[test]
    fn test_eliminate_field_get() {
        let input = r#"function @test(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = field.get v0, 0
    v2 = field.get v0, 0
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = field.get v0, 0
    v3 = iadd v1, v1
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Different field indices are not CSE'd.
    #[test]
    fn test_distinguish_different_field_indices() {
        let input = r#"function @test(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = field.get v0, 0
    v2 = field.get v0, 1
    v3 = iadd v1, v2
    return v3
}"#;
        // should be unchanged: different field indices
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_unchanged(input);
    }

    /// Element accesses with same base and index are CSE'd.
    #[test]
    fn test_eliminate_element_get() {
        let input = r#"function @test(v0: [i32; 10], v1: i64) -> i32 {
block0(v0: [i32; 10], v1: i64):
    v2 = element.get v0, v1
    v3 = element.get v0, v1
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test(v0: [i32; 10], v1: i64) -> i32 {
block0(v0: [i32; 10], v1: i64):
    v2 = element.get v0, v1
    v4 = iadd v2, v2
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Casts to identical types are CSE'd.
    #[test]
    fn test_eliminate_casts() {
        let input = r#"function @test(v0: i32) -> i64 {
block0(v0: i32):
    v1 = sextend v0 -> i64
    v2 = sextend v0 -> i64
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: i32) -> i64 {
block0(v0: i32):
    v1 = sextend v0 -> i64
    v3 = iadd v1, v1
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }

    /// Unique expressions are preserved unchanged.
    #[test]
    fn test_preserve_unique_expressions() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = isub v0, v1
    v4 = imul v2, v3
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_unchanged(input);
    }

    /// Substitutions propagate to terminator.
    #[test]
    fn test_propagate_substitutions_to_terminator() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    v4 = iadd v0, v1
    branch v2, block1(v4), block2(v4)
block1(v5: i32):
    return v5
block2(v6: i32):
    return v6
}"#;
        // v4 -> v3, and the branch should use v3
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    branch v2, block1(v3), block2(v3)
block1(v5: i32):
    return v5
block2(v6: i32):
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&LocalCse);
        program.assert_output(expected);
    }
}
