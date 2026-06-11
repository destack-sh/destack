use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{ControlFlowGraph, DominatorTree};
use crate::common::mir::{
    ExpressionKey, apply_substitutions_in_dominated_blocks, build_use_def_maps,
    clone_instruction_metadata, expression_key_from_instruction, instruction_is_speculatable,
    instruction_map, instruction_substitute_uses_in_tree,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Hoist common instructions out of diamonds.
    ///
    /// Finds identical, speculatable instruction prefixes in both sides of a
    /// branch and hoists them into the branching block.
    ///
    /// ```mir
    /// function before(v0: int32, v1: int32, v2: boolean): int32 {
    /// b0(v0: int32, v1: int32, v2: boolean):
    ///     branch v2, b1, b2
    /// b1:
    ///     v3 = int.add v0, v1
    ///     jump b3(v3)
    /// b2:
    ///     v4 = int.add v0, v1
    ///     jump b3(v4)
    /// b3(v5: int32):
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32, v1: int32, v2: boolean): int32 {
    /// b0(v0: int32, v1: int32, v2: boolean):
    ///     v6 = int.add v0, v1
    ///     branch v2, b1, b2
    /// b1:
    ///     jump b3(v6)
    /// b2:
    ///     jump b3(v6)
    /// b3(v5: int32):
    ///     return v5
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only hoists identical speculatable instructions present in both arms
    /// - Only hoists instructions whose operands dominate the header
    /// - Requires branch successors with a single predecessor
    /// - Limits hoisting per diamond to keep compile time predictable
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
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // recompute value ids for inserted instructions
        function.recompute_next_value_id(tree);

        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();

        // run the hoisting pass
        let changed = run_code_hoisting(function, tree, &cfg, &domtree);

        // preserve analyses when nothing changed
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

/// Hoist common instructions out of branch diamonds.
fn run_code_hoisting(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
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
        // load block data
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // require a conditional branch
        let (then_target, else_target) = match terminator {
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => (then_target.clone(), else_target.clone()),
            _ => continue,
        };

        let Some(then_block) = then_target.block.block() else {
            continue;
        };
        let Some(else_block) = else_target.block.block() else {
            continue;
        };

        let Some(then_arguments) = then_target
            .arguments
            .iter()
            .copied()
            .map(|argument| argument.value())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };

        let Some(else_arguments) = else_target
            .arguments
            .iter()
            .copied()
            .map(|argument| argument.value())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
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

        // record whether the function changed
        changed |= hoisted;
    }

    changed
}

/// Hoist a common instruction prefix for a branch.
#[allow(clippy::too_many_arguments)]
fn hoist_common_prefix(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
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

    // build substitution maps for both sides
    let mut then_substitutions = HashMap::new();
    let mut else_substitutions = HashMap::new();
    let mut new_header_instructions = tree.get(header).instructions.clone();

    // track which values are hoisted already
    let mut hoisted_values: HashSet<mir::Value> = HashSet::new();
    let mut hoisted_then_ids: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    let mut hoisted_else_ids: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();

    // hoist candidates while dependencies are available
    let mut progress = true;
    while progress && hoisted_then_ids.len() < MAX_HOISTED_INSTRUCTIONS {
        // reset progress until a candidate is hoisted
        progress = false;

        // rebuild expression indices using current substitutions
        let mut then_value_map = then_param_rewrites.clone();
        then_value_map.extend(then_substitutions.clone());
        let mut else_value_map = else_param_rewrites.clone();
        else_value_map.extend(else_substitutions.clone());

        let then_index = build_expression_index(&then_data, &then_value_map, tree);
        let else_index = build_expression_index(&else_data, &else_value_map, tree);

        // collect matching expression pairs
        let mut candidates = Vec::new();
        for (key, then_entry) in &then_index {
            // skip when the else side has no match
            let Some(else_entry) = else_index.get(key) else {
                continue;
            };

            // skip entries already hoisted
            if hoisted_then_ids.contains(&then_entry.instruction_id)
                || hoisted_else_ids.contains(&else_entry.instruction_id)
            {
                continue;
            }

            candidates.push(HoistCandidate {
                then_id: then_entry.instruction_id,
                else_id: else_entry.instruction_id,
                instruction: then_entry.instruction.clone(),
                then_dest: then_entry.destination,
                else_dest: else_entry.destination,
            });
        }

        // stop when no more candidates are available
        if candidates.is_empty() {
            break;
        }

        // process candidate pairs
        for candidate in candidates {
            // stop when the configured limit is reached
            if hoisted_then_ids.len() >= MAX_HOISTED_INSTRUCTIONS {
                break;
            }

            // build a remap for existing hoisted values
            let mut value_map = then_param_rewrites.clone();
            value_map.extend(then_substitutions.clone());
            let normalized =
                instruction_substitute_uses_in_tree(&candidate.instruction, &value_map, tree);

            // ensure operands are available in the header
            if !instruction_operands_available(
                &normalized,
                header,
                def_blocks,
                domtree,
                &hoisted_values,
            ) {
                continue;
            }

            // allocate a new destination for the hoisted instruction
            let new_dest = function.next_typed_value_like(candidate.then_dest);
            value_map.insert(candidate.then_dest, new_dest);

            // clone instruction with updated destinations and operands
            let hoisted_instruction = instruction_map(&candidate.instruction, &value_map, tree);
            let hoisted_id = tree.insert(hoisted_instruction);
            clone_instruction_metadata(tree, candidate.then_id, hoisted_id, &value_map);
            new_header_instructions.push(hoisted_id);

            // record substitutions for both branches
            then_substitutions.insert(candidate.then_dest, new_dest);
            else_substitutions.insert(candidate.else_dest, new_dest);
            hoisted_values.insert(new_dest);
            hoisted_then_ids.insert(candidate.then_id);
            hoisted_else_ids.insert(candidate.else_id);
            progress = true;
        }
    }

    // bail when nothing is hoisted
    if hoisted_then_ids.is_empty() {
        return false;
    }

    // update the header block with hoisted instructions
    let mut header_block = tree.get(header).clone();
    header_block.instructions = new_header_instructions;
    tree.set(header, header_block);

    // drop hoisted instructions from both successor blocks
    let then_trimmed = drop_instructions(&then_data, &hoisted_then_ids);
    let else_trimmed = drop_instructions(&else_data, &hoisted_else_ids);
    tree.set(then_block, then_trimmed);
    tree.set(else_block, else_trimmed);

    // apply substitutions to dominated blocks
    let then_changed = apply_substitutions_in_dominated_blocks(
        function,
        tree,
        domtree,
        then_block,
        &then_substitutions,
    );
    let else_changed = apply_substitutions_in_dominated_blocks(
        function,
        tree,
        domtree,
        else_block,
        &else_substitutions,
    );

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

    // map then parameters to their incoming arguments
    for (then_param, then_arg) in then_block.parameters.iter().zip(then_arguments.iter()) {
        let Some(then_param) = then_param.value.value() else {
            continue;
        };

        then_rewrites.insert(then_param, *then_arg);
    }

    // map else parameters to their incoming arguments
    for (else_param, else_arg) in else_block.parameters.iter().zip(else_arguments.iter()) {
        let Some(else_param) = else_param.value.value() else {
            continue;
        };

        else_rewrites.insert(else_param, *else_arg);
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
) -> bool {
    // scan all operands
    for value in instruction.uses() {
        let Some(value) = value.value() else {
            return false;
        };

        // skip values already hoisted into the header
        if hoisted_values.contains(&value) {
            continue;
        }

        // skip values with no known definition block
        let Some(def_block) = def_blocks.get(&value) else {
            continue;
        };

        // require the definition to dominate the header
        if !domtree.dominates(*def_block, header) {
            return false;
        }
    }

    true
}

/// Remove specific instructions from a block.
fn drop_instructions(
    block: &mir::Block,
    removed: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) -> mir::Block {
    // clone the original block
    let mut updated = block.clone();

    // retain instructions not removed
    updated
        .instructions
        .retain(|instruction_id| !removed.contains(instruction_id));

    updated
}

/// Index entry for hoistable expressions.
struct ExpressionEntry {
    /// Instruction id for this expression.
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    /// The instruction itself.
    instruction: mir::Instruction,
    /// The destination value.
    destination: mir::Value,
}

/// Candidate instruction pair to hoist.
struct HoistCandidate {
    /// Instruction id in the then block.
    then_id: mir::LocalNodeId<mir::Instruction>,
    /// Instruction id in the else block.
    else_id: mir::LocalNodeId<mir::Instruction>,
    /// The instruction to clone.
    instruction: mir::Instruction,
    /// Destination value in the then block.
    then_dest: mir::Value,
    /// Destination value in the else block.
    else_dest: mir::Value,
}

/// Build an expression index for a block.
fn build_expression_index(
    block: &mir::Block,
    value_rewrites: &HashMap<mir::Value, mir::Value>,
    tree: &mut mir::Tree,
) -> HashMap<ExpressionKey, ExpressionEntry> {
    // allocate the index map
    let mut index = HashMap::new();

    // scan instructions in test order
    for &instruction_id in &block.instructions {
        // clone the instruction for inspection
        let instruction = tree.get(instruction_id).clone();

        // skip instructions without destinations
        let Some(destination) = instruction
            .destination()
            .and_then(|destination| destination.value())
        else {
            continue;
        };

        // skip non speculatable instructions
        if !instruction_is_speculatable(&instruction, tree) {
            continue;
        }

        // normalize operands before hashing
        let normalized = instruction_substitute_uses_in_tree(&instruction, value_rewrites, tree);

        // skip instructions without a stable key
        let Some(key) = expression_key_from_instruction(&normalized, tree) else {
            continue;
        };

        // insert the first instance for the key
        index.entry(key).or_insert(ExpressionEntry {
            instruction_id,
            instruction,
            destination,
        });
    }

    index
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::CodeHoisting;

    /// Identical branch instructions are hoisted into the header.
    #[test]
    fn test_hoist_simple_diamond() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    jump b3(v3)
b2:
    v4: int32 = int.add v0, v1
    jump b3(v4)
b3(v5: int32):
    return v5
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    jump b3(v3)
b2:
    jump b3(v3)
b3(v4: int32):
    return v4
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CodeHoisting);
        test.assert_output(expected);
    }

    /// Non speculatable instructions are not hoisted.
    #[test]
    fn test_hoist_skips_division() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.div.s v0, v1
    jump b3(v3)
b2:
    v4: int32 = int.div.s v0, v1
    jump b3(v4)
b3(v5: int32):
    return v5
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CodeHoisting);
        test.assert_output(input);
    }

    /// Differing branch instructions are not hoisted.
    #[test]
    fn test_hoist_requires_equivalence() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    jump b3(v3)
b2:
    v4: int32 = int.sub v0, v1
    jump b3(v4)
b3(v5: int32):
    return v5
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CodeHoisting);
        test.assert_output(input);
    }

    /// Chains of identical instructions are hoisted together.
    #[test]
    fn test_hoist_common_prefix_chain() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v3, v1
    jump b3(v4)
b2:
    v5: int32 = int.add v0, v1
    v6: int32 = int.add v5, v1
    jump b3(v6)
b3(v7: int32):
    return v7
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v3, v1
    branch v2, b1, b2
b1:
    jump b3(v4)
b2:
    jump b3(v4)
b3(v5: int32):
    return v5
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CodeHoisting);
        test.assert_output(expected);
    }

    /// Branch parameters are rewritten before hoisting.
    #[test]
    fn test_hoist_rewrites_branch_params() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1(v0, v1), b2(v0, v1)
b1(v3: int32, v4: int32):
    v5: int32 = int.add v3, v4
    jump b3(v5)
b2(v6: int32, v7: int32):
    v8: int32 = int.add v6, v7
    jump b3(v8)
b3(v9: int32):
    return v9
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1(v0, v1), b2(v0, v1)
b1(v4: int32, v5: int32):
    jump b3(v3)
b2(v6: int32, v7: int32):
    jump b3(v3)
b3(v8: int32):
    return v8
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CodeHoisting);
        test.assert_output(expected);
    }

    /// Identical instructions can be hoisted even when not in the prefix.
    #[test]
    fn test_hoist_non_prefix_match() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.mul v0, v1
    v4: int32 = int.add v0, v1
    jump b3(v4)
b2:
    v5: int32 = int.sub v0, v1
    v6: int32 = int.add v0, v1
    jump b3(v6)
b3(v7: int32):
    return v7
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    v4: int32 = int.mul v0, v1
    jump b3(v3)
b2:
    v5: int32 = int.sub v0, v1
    jump b3(v3)
b3(v6: int32):
    return v6
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CodeHoisting);
        test.assert_output(expected);
    }

    /// Branches with multiple predecessors are not hoisted.
    #[test]
    fn test_hoist_requires_single_predecessor() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    jump b3(v3)
b2:
    v4: int32 = int.add v0, v1
    jump b3(v4)
b3(v5: int32):
    return v5
b4:
    jump b2
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CodeHoisting);
        test.assert_output(input);
    }
}
