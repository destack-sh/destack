use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree};
use crate::optimize::common::{
    build_use_def_maps, expression_key_from_instruction, instruction_is_speculatable,
    instruction_map, instruction_substitute_uses_in_tree, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

declare_pass! {
    /// Hoist common instructions out of diamonds.
    ///
    /// Finds identical, speculatable instruction prefixes in both sides of a
    /// branch and hoists them into the branching block.
    ///
    /// ```mir
    /// function @before(v0: i32, v1: i32, v2: bool) -> i32 {
    /// block0(v0: i32, v1: i32, v2: bool):
    ///     branch v2, block1, block2
    /// block1:
    ///     v3 = iadd v0, v1
    ///     jump block3(v3)
    /// block2:
    ///     v4 = iadd v0, v1
    ///     jump block3(v4)
    /// block3(v5: i32):
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32, v1: i32, v2: bool) -> i32 {
    /// block0(v0: i32, v1: i32, v2: bool):
    ///     v6 = iadd v0, v1
    ///     branch v2, block1, block2
    /// block1:
    ///     jump block3(v6)
    /// block2:
    ///     jump block3(v6)
    /// block3(v5: i32):
    ///     return v5
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only hoists identical instruction prefixes
    /// - Only hoists speculatable instructions
    /// - Requires branch successors with a single predecessor
    #[pass(id = "hoist")]
    pub CodeHoisting,
    "Hoist redundant instructions"
}

/// Maximum number of instructions to hoist per branch.
const MAX_HOISTED_INSTRUCTIONS: usize = 8;

impl FunctionPass for CodeHoisting {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // recompute value ids for inserted instructions
        function.recompute_next_value_id(tree);

        // gather analyses
        let analyses = FunctionAnalyses::new(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();

        // run the hoisting pass
        let changed = run_code_hoisting(function, tree, &cfg, &domtree);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "CodeHoisting"
    }

    fn id(&self) -> &'static str {
        "hoist"
    }
}

// NOTE #Incomplete: extend to non prefix redundancy via gvn

/// Hoist common instructions out of branch diamonds.
fn run_code_hoisting(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
) -> bool {
    // build definition metadata
    let use_def = build_use_def_maps(function, tree);

    // track whether any changes were made
    let mut changed = false;

    // scan each block for a branch candidate
    let block_ids = function.blocks.clone();
    for block_id in block_ids {
        let block = tree.get(block_id);

        // require a conditional branch
        let (then_block, else_block, then_arguments, else_arguments) = match &block.terminator {
            mir::Terminator::Branch {
                then_target,
                else_target,
                then_arguments,
                else_arguments,
                ..
            } => (
                *then_target,
                *else_target,
                then_arguments.clone(),
                else_arguments.clone(),
            ),
            _ => continue,
        };

        // reject degenerate branches
        if then_block == else_block {
            continue;
        }

        // require single predecessor for both sides
        if cfg.predecessors(then_block).len() != 1 || cfg.predecessors(else_block).len() != 1 {
            continue;
        }

        // hoist when a common prefix exists
        let hoisted = hoist_common_prefix(
            function,
            tree,
            domtree,
            &use_def.def_block,
            block_id,
            then_block,
            else_block,
            &then_arguments,
            &else_arguments,
        );
        changed |= hoisted;
    }

    changed
}

/// Hoist a common instruction prefix for a branch.
#[allow(clippy::too_many_arguments)]
fn hoist_common_prefix(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    domtree: &DominatorTree,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    header: mir::LocalNodeId<mir::Block>,
    then_block: mir::LocalNodeId<mir::Block>,
    else_block: mir::LocalNodeId<mir::Block>,
    then_arguments: &[mir::Value],
    else_arguments: &[mir::Value],
) -> bool {
    // collect block data
    let then_data = tree.get(then_block).clone();
    let else_data = tree.get(else_block).clone();

    // require well formed branch arguments
    if then_data.parameters.len() != then_arguments.len()
        || else_data.parameters.len() != else_arguments.len()
    {
        return false;
    }

    // build parameter rewrites to their incoming arguments
    let (then_param_rewrites, else_param_rewrites) =
        build_param_rewrites(&then_data, &else_data, then_arguments, else_arguments);

    // track equivalence for hoisted values
    let mut equivalence: HashMap<mir::Value, mir::Value> = HashMap::new();

    // track which then values have been hoisted already
    let mut hoisted_then_values: HashSet<mir::Value> = HashSet::new();

    // collect hoistable instruction pairs
    let mut hoist_pairs: Vec<(
        mir::LocalNodeId<mir::Instruction>,
        mir::LocalNodeId<mir::Instruction>,
    )> = Vec::new();

    // compute the common prefix length
    let max_len = then_data
        .instructions
        .len()
        .min(else_data.instructions.len());
    for index in 0..max_len {
        // stop after the configured limit
        if hoist_pairs.len() >= MAX_HOISTED_INSTRUCTIONS {
            break;
        }

        // read the instruction pair
        let then_id = then_data.instructions[index];
        let else_id = else_data.instructions[index];
        let then_instruction = tree.get(then_id).clone();
        let else_instruction = tree.get(else_id).clone();

        // require destinations for both instructions
        let Some(then_dest) = then_instruction.destination() else {
            break;
        };
        let Some(else_dest) = else_instruction.destination() else {
            break;
        };

        // require speculatable instructions on both sides
        if !instruction_is_speculatable(&then_instruction)
            || !instruction_is_speculatable(&else_instruction)
        {
            break;
        }

        // require equivalent expressions
        let Some(_) = expression_key_from_instruction(&then_instruction, tree) else {
            break;
        };
        let then_normalized =
            instruction_substitute_uses_in_tree(&then_instruction, &then_param_rewrites, tree);
        let Some(then_key) = expression_key_from_instruction(&then_normalized, tree) else {
            break;
        };

        let mut else_normalize_map = else_param_rewrites.clone();
        for (else_value, then_value) in &equivalence {
            else_normalize_map.insert(*else_value, *then_value);
        }
        let else_normalized =
            instruction_substitute_uses_in_tree(&else_instruction, &else_normalize_map, tree);
        let Some(else_key) = expression_key_from_instruction(&else_normalized, tree) else {
            break;
        };
        if then_key != else_key {
            break;
        }

        // ensure operands are available in the header
        if !instruction_operands_available(
            &then_instruction,
            header,
            def_blocks,
            domtree,
            &hoisted_then_values,
            &then_param_rewrites,
        ) {
            break;
        }

        // record this instruction pair for hoisting
        hoist_pairs.push((then_id, else_id));
        hoisted_then_values.insert(then_dest);
        equivalence.insert(else_dest, then_dest);
    }

    // bail when no instructions can be hoisted
    if hoist_pairs.is_empty() {
        return false;
    }

    // build substitution maps for both sides
    let mut then_substitutions = HashMap::new();
    let mut else_substitutions = HashMap::new();
    let mut new_header_instructions = tree.get(header).instructions.clone();

    // insert hoisted instructions into the header
    for (then_id, else_id) in &hoist_pairs {
        // read the original instruction
        let then_instruction = tree.get(*then_id).clone();
        let else_instruction = tree.get(*else_id).clone();

        // extract destinations for substitution maps
        let Some(then_dest) = then_instruction.destination() else {
            continue;
        };
        let Some(else_dest) = else_instruction.destination() else {
            continue;
        };

        // allocate a new destination for the hoisted instruction
        let new_dest = function.next_value();

        // build a remap for existing hoisted values
        let mut value_map = then_param_rewrites.clone();
        value_map.extend(then_substitutions.clone());
        value_map.insert(then_dest, new_dest);

        // clone instruction with updated destinations and operands
        let hoisted_instruction = instruction_map(&then_instruction, &value_map, tree);
        let hoisted_id = tree.insert(hoisted_instruction);
        new_header_instructions.push(hoisted_id);

        // record substitutions for both branches
        then_substitutions.insert(then_dest, new_dest);
        else_substitutions.insert(else_dest, new_dest);
    }

    // update the header block with hoisted instructions
    let mut header_block = tree.get(header).clone();
    header_block.instructions = new_header_instructions;
    tree.replace(header, header_block);

    // drop hoisted instructions from both successor blocks
    let then_trimmed = drop_prefix_instructions(&then_data, hoist_pairs.len());
    let else_trimmed = drop_prefix_instructions(&else_data, hoist_pairs.len());
    tree.replace(then_block, then_trimmed);
    tree.replace(else_block, else_trimmed);

    // apply substitutions to dominated blocks
    let then_changed =
        apply_substitutions(function, tree, domtree, then_block, &then_substitutions);
    let else_changed =
        apply_substitutions(function, tree, domtree, else_block, &else_substitutions);

    then_changed || else_changed || !then_substitutions.is_empty()
}

/// Build parameter rewrites for branch arguments.
fn build_param_rewrites(
    then_block: &mir::Block,
    else_block: &mir::Block,
    then_arguments: &[mir::Value],
    else_arguments: &[mir::Value],
) -> (
    HashMap<mir::Value, mir::Value>,
    HashMap<mir::Value, mir::Value>,
) {
    // prepare empty mappings
    let mut then_rewrites = HashMap::new();
    let mut else_rewrites = HashMap::new();

    // require matching parameter arity
    if then_block.parameters.len() != then_arguments.len()
        || else_block.parameters.len() != else_arguments.len()
    {
        return (then_rewrites, else_rewrites);
    }

    // map parameters to their incoming arguments
    for (then_param, then_arg) in then_block.parameters.iter().zip(then_arguments.iter()) {
        then_rewrites.insert(then_param.value, *then_arg);
    }
    for (else_param, else_arg) in else_block.parameters.iter().zip(else_arguments.iter()) {
        else_rewrites.insert(else_param.value, *else_arg);
    }

    (then_rewrites, else_rewrites)
}

/// Check that all operands are available in the header.
fn instruction_operands_available(
    instruction: &mir::Instruction,
    header: mir::LocalNodeId<mir::Block>,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
    hoisted_values: &HashSet<mir::Value>,
    param_rewrites: &HashMap<mir::Value, mir::Value>,
) -> bool {
    // scan all operands
    for value in instruction.uses() {
        let normalized = param_rewrites.get(&value).copied().unwrap_or(value);

        if hoisted_values.contains(&normalized) {
            continue;
        }

        let Some(def_block) = def_blocks.get(&normalized) else {
            return false;
        };
        if !domtree.dominates(*def_block, header) {
            return false;
        }
    }

    true
}

/// Remove a prefix of instructions from a block.
fn drop_prefix_instructions(block: &mir::Block, count: usize) -> mir::Block {
    // clone the original block
    let mut updated = block.clone();

    // drop instructions from the start
    if count > 0 && count <= updated.instructions.len() {
        updated.instructions.drain(0..count);
    }

    updated
}

/// Apply a substitution map to dominated blocks.
fn apply_substitutions(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    domtree: &DominatorTree,
    root: mir::LocalNodeId<mir::Block>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> bool {
    // skip when there is nothing to substitute
    if substitutions.is_empty() {
        return false;
    }

    // track whether any changes were made
    let mut changed = false;

    // update blocks dominated by the root
    for &block_id in &function.blocks {
        if !domtree.dominates(root, block_id) {
            continue;
        }

        // rewrite instructions in place
        let mut block = tree.get(block_id).clone();
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id).clone();
            let updated = instruction_substitute_uses_in_tree(&instruction, substitutions, tree);
            if updated != instruction {
                tree.replace(instruction_id, updated);
                changed = true;
            }
        }

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(&block.terminator, substitutions);
        if new_terminator != block.terminator {
            block.terminator = new_terminator;
            tree.replace(block_id, block);
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::CodeHoisting;

    /// Identical branch instructions are hoisted into the header.
    #[test]
    fn test_hoist_simple_diamond() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3(v3)
block2:
    v4 = iadd v0, v1
    jump block3(v4)
block3(v5: i32):
    return v5
}"#;
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v6 = iadd v0, v1
    branch v2, block1, block2
block1:
    jump block3(v6)
block2:
    jump block3(v6)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CodeHoisting);
        program.assert_output(expected);
    }

    /// Non speculatable instructions are not hoisted.
    #[test]
    fn test_hoist_skips_division() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = sdiv v0, v1
    jump block3(v3)
block2:
    v4 = sdiv v0, v1
    jump block3(v4)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CodeHoisting);
        program.assert_output(input);
    }

    /// Differing branch instructions are not hoisted.
    #[test]
    fn test_hoist_requires_equivalence() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3(v3)
block2:
    v4 = isub v0, v1
    jump block3(v4)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CodeHoisting);
        program.assert_output(input);
    }

    /// Chains of identical instructions are hoisted together.
    #[test]
    fn test_hoist_common_prefix_chain() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    v4 = iadd v3, v1
    jump block3(v4)
block2:
    v5 = iadd v0, v1
    v6 = iadd v5, v1
    jump block3(v6)
block3(v7: i32):
    return v7
}"#;
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v8 = iadd v0, v1
    v9 = iadd v8, v1
    branch v2, block1, block2
block1:
    jump block3(v9)
block2:
    jump block3(v9)
block3(v7: i32):
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CodeHoisting);
        program.assert_output(expected);
    }

    /// Branch parameters are rewritten before hoisting.
    #[test]
    fn test_hoist_rewrites_branch_params() {
        // source program
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1(v0, v1), block2(v0, v1)
block1(v3: i32, v4: i32):
    v5 = iadd v3, v4
    jump block3(v5)
block2(v6: i32, v7: i32):
    v8 = iadd v6, v7
    jump block3(v8)
block3(v9: i32):
    return v9
}"#;
        // expected output
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v10 = iadd v0, v1
    branch v2, block1(v0, v1), block2(v0, v1)
block1(v3: i32, v4: i32):
    jump block3(v10)
block2(v6: i32, v7: i32):
    jump block3(v10)
block3(v9: i32):
    return v9
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&CodeHoisting);
        program.assert_output(expected);
    }

    /// Branches with multiple predecessors are not hoisted.
    #[test]
    fn test_hoist_requires_single_predecessor() {
        // source program
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3(v3)
block2:
    v4 = iadd v0, v1
    jump block3(v4)
block3(v5: i32):
    return v5
block4:
    jump block2
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&CodeHoisting);
        program.assert_output(input);
    }
}
