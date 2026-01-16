use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::ControlFlowGraph;
use crate::optimize::common::{instruction_is_speculatable, instruction_map};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Convert simple diamonds into select instructions.
    ///
    /// This removes branches by speculatively executing both sides of a small
    /// conditional and selecting the result with a `select`.
    ///
    /// ```mir
    /// function @before(v0: bool, v1: i32, v2: i32) -> i32 {
    /// block0(v0: bool, v1: i32, v2: i32):
    ///     branch v0, block1(v1, v2), block2(v1, v2)
    /// block1(v3: i32, v4: i32):
    ///     v5 = iadd v3, v4
    ///     jump block3(v5)
    /// block2(v6: i32, v7: i32):
    ///     v8 = isub v6, v7
    ///     jump block3(v8)
    /// block3(v9: i32):
    ///     return v9
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: bool, v1: i32, v2: i32) -> i32 {
    /// block0(v0: bool, v1: i32, v2: i32):
    ///     v10 = iadd v1, v2
    ///     v11 = isub v1, v2
    ///     v12 = select v0, v10, v11
    ///     jump block3(v12)
    /// block1(v3: i32, v4: i32):
    ///     v5 = iadd v3, v4
    ///     jump block3(v5)
    /// block2(v6: i32, v7: i32):
    ///     v8 = isub v6, v7
    ///     jump block3(v8)
    /// block3(v9: i32):
    ///     return v9
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only converts diamonds with a single predecessor per side block
    /// - Requires both branches to contain only speculatable instructions
    /// - Requires both branches to jump to a single common merge block
    #[pass(id = "if-convert")]
    pub IfConvert,
    "Convert small diamonds into select instructions"
}

/// Base instruction budget for if conversion.
const BASE_CONVERT_BUDGET: usize = 16;
/// Larger budget when profile indicates balanced branches.
const BALANCED_CONVERT_BUDGET: usize = 48;
/// Threshold for treating a branch as highly biased.
const BIASED_BRANCH_RATIO: f64 = 0.90;

impl FunctionPass for IfConvert {
    /// Run if conversion on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // run if conversion
        let changed = run_if_convert(function, tree, ctx);

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "IfConvert"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "if-convert"
    }
}

/// Candidate diamond for conversion.
#[derive(Debug, Clone)]
struct IfConvertCandidate {
    /// The header block containing the branch.
    header: mir::LocalNodeId<mir::Block>,
    /// The branch condition value.
    condition: mir::Value,
    /// The then block.
    then_block: mir::LocalNodeId<mir::Block>,
    /// The else block.
    else_block: mir::LocalNodeId<mir::Block>,
    /// The merge block.
    merge_block: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to the then block.
    then_arguments: Vec<mir::Value>,
    /// Arguments passed to the else block.
    else_arguments: Vec<mir::Value>,
}

/// Run if conversion and return true when changes were made.
fn run_if_convert(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // build control flow graph
    let analyses = ctx.function_analyses(function, tree);
    let cfg = analyses.get::<ControlFlowGraph>().clone();

    // collect candidates before mutation
    let mut candidates = Vec::new();
    for &block_id in &function.blocks {
        let Some(candidate) = find_if_convert_candidate(block_id, function, tree, &cfg) else {
            continue;
        };

        candidates.push(candidate);
    }

    // apply conversions
    if candidates.is_empty() {
        return false;
    }

    function.recompute_next_value_id(tree);
    let mut changed = false;
    for candidate in candidates {
        changed |= apply_if_convert(candidate, function, tree, ctx);
    }

    changed
}

/// Find a diamond pattern rooted at the header block.
fn find_if_convert_candidate(
    header: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mir::NodeTree,
    cfg: &ControlFlowGraph,
) -> Option<IfConvertCandidate> {
    // read header terminator
    let header_block = tree.get(header);
    let (condition, then_block, else_block, then_arguments, else_arguments) =
        match &header_block.terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => (
                *condition,
                *then_target,
                *else_target,
                then_arguments.clone(),
                else_arguments.clone(),
            ),
            _ => return None,
        };

    // ignore degenerate branches
    if then_block == else_block {
        return None;
    }

    // require single predecessor for each side
    if cfg.predecessors(then_block).len() != 1 || cfg.predecessors(else_block).len() != 1 {
        return None;
    }

    // ensure both side blocks are within the function
    if !function.blocks.contains(&then_block) || !function.blocks.contains(&else_block) {
        return None;
    }

    // read side blocks and require a common merge
    let then_block_data = tree.get(then_block);
    let else_block_data = tree.get(else_block);

    let merge_block = match (&then_block_data.terminator, &else_block_data.terminator) {
        (mir::Terminator::Jump { target, .. }, mir::Terminator::Jump { target: other, .. })
            if target == other =>
        {
            *target
        }
        _ => return None,
    };

    // ensure merge block exists
    if !function.blocks.contains(&merge_block) {
        return None;
    }

    // require the merge block to have only the diamond predecessors
    let merge_preds = cfg.predecessors(merge_block);
    if merge_preds.len() != 2
        || !merge_preds.contains(&then_block)
        || !merge_preds.contains(&else_block)
    {
        return None;
    }

    // require compatible block parameters
    if then_block_data.parameters.len() != then_arguments.len() {
        return None;
    }

    if else_block_data.parameters.len() != else_arguments.len() {
        return None;
    }

    Some(IfConvertCandidate {
        header,
        condition,
        then_block,
        else_block,
        merge_block,
        then_arguments,
        else_arguments,
    })
}

/// Apply if conversion to the candidate.
fn apply_if_convert(
    candidate: IfConvertCandidate,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // build value maps for each branch
    let then_block = tree.get(candidate.then_block).clone();
    let else_block = tree.get(candidate.else_block).clone();

    // ensure both sides are speculatable
    if !instructions_speculatable(&then_block.instructions, tree)
        || !instructions_speculatable(&else_block.instructions, tree)
    {
        return false;
    }

    // build value maps
    let Some(then_value_map) =
        build_value_map(function, tree, &then_block, &candidate.then_arguments)
    else {
        return false;
    };
    let Some(else_value_map) =
        build_value_map(function, tree, &else_block, &candidate.else_arguments)
    else {
        return false;
    };

    // clone branch instructions into the header
    let mut new_instructions = tree.get(candidate.header).instructions.clone();
    clone_block_instructions(tree, &then_block, &then_value_map, &mut new_instructions);
    clone_block_instructions(tree, &else_block, &else_value_map, &mut new_instructions);

    // read merge arguments
    let Some(then_merge_args) = jump_arguments(&then_block) else {
        return false;
    };
    let Some(else_merge_args) = jump_arguments(&else_block) else {
        return false;
    };
    if then_merge_args.len() != else_merge_args.len() {
        return false;
    }

    // check conversion cost model
    if !should_convert(
        &candidate,
        &then_block,
        &else_block,
        then_merge_args.len(),
        ctx,
    ) {
        return false;
    }

    // require merge parameters to match the argument count
    let merge_block = tree.get(candidate.merge_block);
    if merge_block.parameters.len() != then_merge_args.len() {
        return false;
    }

    // build select values for merge arguments
    let mut select_args = Vec::with_capacity(then_merge_args.len());
    for (then_value, else_value) in then_merge_args.iter().zip(else_merge_args.iter()) {
        let then_value = remap_value(*then_value, &then_value_map);
        let else_value = remap_value(*else_value, &else_value_map);

        let destination = function.next_value();
        let select = mir::Instruction::Select {
            destination,
            condition: candidate.condition,
            then_value,
            else_value,
        };
        let select_id = tree.insert(select);
        new_instructions.push(select_id);
        select_args.push(destination);
    }

    // update header block
    let mut header = tree.get(candidate.header).clone();
    header.instructions = new_instructions;
    header.terminator = mir::Terminator::Jump {
        target: candidate.merge_block,
        arguments: select_args,
    };
    tree.replace(candidate.header, header);

    true
}

/// Decide whether to convert a candidate based on cost and profile data.
fn should_convert(
    candidate: &IfConvertCandidate,
    then_block: &mir::Block,
    else_block: &mir::Block,
    merge_args: usize,
    ctx: &PipelineContext<'_>,
) -> bool {
    // compute instruction costs for each branch
    let then_cost = block_instruction_cost(then_block);
    let else_cost = block_instruction_cost(else_block);
    let select_cost = merge_args;
    let total_cost = then_cost + else_cost + select_cost;

    // allow small conversions unconditionally
    if total_cost <= BASE_CONVERT_BUDGET {
        return true;
    }

    // check branch profile balance when available
    if let Some((then_count, else_count)) = branch_profile_counts(candidate, ctx.profile()) {
        let total_count = then_count + else_count;
        if total_count == 0 {
            return total_cost <= BASE_CONVERT_BUDGET;
        }

        let then_ratio = then_count as f64 / total_count as f64;
        let else_ratio = else_count as f64 / total_count as f64;
        let is_balanced = then_ratio >= (1.0 - BIASED_BRANCH_RATIO)
            && then_ratio <= BIASED_BRANCH_RATIO
            && else_ratio >= (1.0 - BIASED_BRANCH_RATIO)
            && else_ratio <= BIASED_BRANCH_RATIO;

        if is_balanced {
            return total_cost <= BALANCED_CONVERT_BUDGET;
        }

        return total_cost <= BASE_CONVERT_BUDGET;
    }

    // fall back to size balance when no profile data is available
    let min_cost = then_cost.min(else_cost);
    if min_cost == 0 {
        return total_cost <= BALANCED_CONVERT_BUDGET;
    }

    let size_ratio = then_cost.max(else_cost) as f64 / min_cost as f64;
    if size_ratio <= 1.25 {
        return total_cost <= BALANCED_CONVERT_BUDGET;
    }

    total_cost <= BASE_CONVERT_BUDGET
}

/// Compute the instruction cost for a block.
fn block_instruction_cost(block: &mir::Block) -> usize {
    block.instructions.len()
}

/// Read branch profile counts when available.
fn branch_profile_counts(
    candidate: &IfConvertCandidate,
    profile: Option<&mir::ProfileTable>,
) -> Option<(u64, u64)> {
    let profile = profile?;
    let then_edge = mir::EdgeKey::new(
        candidate.header,
        mir::EdgeKind::BranchThen,
        candidate.then_block,
    );
    let else_edge = mir::EdgeKey::new(
        candidate.header,
        mir::EdgeKind::BranchElse,
        candidate.else_block,
    );

    let then_count = profile.edge_count(&then_edge)?.value;
    let else_count = profile.edge_count(&else_edge)?.value;

    Some((then_count, else_count))
}

/// Build a remapping of block parameters and instruction destinations.
fn build_value_map(
    function: &mut mir::Function,
    tree: &mir::NodeTree,
    block: &mir::Block,
    arguments: &[mir::Value],
) -> Option<HashMap<mir::Value, mir::Value>> {
    // validate parameter arity
    if block.parameters.len() != arguments.len() {
        return None;
    }

    // map parameters to incoming arguments
    let mut value_map = HashMap::new();
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        value_map.insert(param.value, *arg);
    }

    // map instruction destinations to fresh values
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        if let Some(destination) = instruction.destination() {
            let new_value = function.next_value();
            value_map.insert(destination, new_value);
        }
    }

    Some(value_map)
}

/// Clone a block's instructions into a header instruction list.
fn clone_block_instructions(
    tree: &mut mir::NodeTree,
    block: &mir::Block,
    value_map: &HashMap<mir::Value, mir::Value>,
    target: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
) {
    // clone instructions in order
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id).clone();
        let cloned = instruction_map(&instruction, value_map, tree);
        let cloned_id = tree.insert(cloned);
        target.push(cloned_id);
    }
}

/// Check whether all instructions are speculatable.
fn instructions_speculatable(
    instructions: &[mir::LocalNodeId<mir::Instruction>],
    tree: &mir::NodeTree,
) -> bool {
    // scan instructions for unsafe operations
    for &instruction_id in instructions {
        let instruction = tree.get(instruction_id);
        if !instruction_is_speculatable(instruction) {
            return false;
        }
    }

    true
}

/// Read jump arguments from a block terminator.
fn jump_arguments(block: &mir::Block) -> Option<Vec<mir::Value>> {
    match &block.terminator {
        mir::Terminator::Jump { arguments, .. } => Some(arguments.clone()),
        _ => None,
    }
}

/// Remap a value through a value map.
fn remap_value(value: mir::Value, value_map: &HashMap<mir::Value, mir::Value>) -> mir::Value {
    value_map.get(&value).copied().unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Convert a simple diamond with speculatable ops into a select.
    #[test]
    fn test_if_convert_simple_diamond() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    branch v0, block1(v1, v2), block2(v1, v2)
block1(v3: i32, v4: i32):
    v5 = iadd v3, v4
    jump block3(v5)
block2(v6: i32, v7: i32):
    v8 = isub v6, v7
    jump block3(v8)
block3(v9: i32):
    return v9
}"#;
        let expected = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    v10 = iadd v1, v2
    v11 = isub v1, v2
    v12 = select v0, v10, v11
    jump block3(v12)
block1(v3: i32, v4: i32):
    v5 = iadd v3, v4
    jump block3(v5)
block2(v6: i32, v7: i32):
    v8 = isub v6, v7
    jump block3(v8)
block3(v9: i32):
    return v9
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&IfConvert);
        program.assert_output(expected);
    }

    /// Convert larger diamonds when balanced and speculatable.
    #[test]
    fn test_if_convert_large_balanced_blocks() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1(v1), block2(v1)
block1(v2: i32):
    v3 = iadd v2, v2
    v4 = iadd v3, v2
    v5 = iadd v4, v2
    v6 = iadd v5, v2
    v7 = iadd v6, v2
    v8 = iadd v7, v2
    v9 = iadd v8, v2
    v10 = iadd v9, v2
    v11 = iadd v10, v2
    jump block3(v11)
block2(v12: i32):
    v13 = isub v12, v12
    v14 = iadd v13, v12
    v15 = iadd v14, v12
    v16 = iadd v15, v12
    v17 = iadd v16, v12
    v18 = iadd v17, v12
    v19 = iadd v18, v12
    v20 = iadd v19, v12
    v21 = iadd v20, v12
    jump block3(v21)
block3(v22: i32):
    return v22
}"#;
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v23 = iadd v1, v1
    v24 = iadd v23, v1
    v25 = iadd v24, v1
    v26 = iadd v25, v1
    v27 = iadd v26, v1
    v28 = iadd v27, v1
    v29 = iadd v28, v1
    v30 = iadd v29, v1
    v31 = iadd v30, v1
    v32 = isub v1, v1
    v33 = iadd v32, v1
    v34 = iadd v33, v1
    v35 = iadd v34, v1
    v36 = iadd v35, v1
    v37 = iadd v36, v1
    v38 = iadd v37, v1
    v39 = iadd v38, v1
    v40 = iadd v39, v1
    v41 = select v0, v31, v40
    jump block3(v41)
block1(v2: i32):
    v3 = iadd v2, v2
    v4 = iadd v3, v2
    v5 = iadd v4, v2
    v6 = iadd v5, v2
    v7 = iadd v6, v2
    v8 = iadd v7, v2
    v9 = iadd v8, v2
    v10 = iadd v9, v2
    v11 = iadd v10, v2
    jump block3(v11)
block2(v12: i32):
    v13 = isub v12, v12
    v14 = iadd v13, v12
    v15 = iadd v14, v12
    v16 = iadd v15, v12
    v17 = iadd v16, v12
    v18 = iadd v17, v12
    v19 = iadd v18, v12
    v20 = iadd v19, v12
    v21 = iadd v20, v12
    jump block3(v21)
block3(v22: i32):
    return v22
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&IfConvert);
        program.assert_output(expected);
    }

    /// Skip conversion when branch instructions may trap.
    #[test]
    fn test_if_convert_skips_trapping_ops() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    branch v0, block1(v1, v2), block2(v1, v2)
block1(v3: i32, v4: i32):
    v5 = sdiv v3, v4
    jump block3(v5)
block2(v6: i32, v7: i32):
    v8 = isub v6, v7
    jump block3(v8)
block3(v9: i32):
    return v9
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&IfConvert);
        program.assert_output(input);
    }

    /// Skip conversion when a branch target has multiple predecessors.
    #[test]
    fn test_if_convert_requires_single_pred() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    branch v0, block1(v1), block2(v1)
block1(v2: i32):
    jump block3(v2)
block2(v3: i32):
    jump block1(v3)
block3(v4: i32):
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&IfConvert);
        program.assert_output(input);
    }

    /// Merge blocks with extra predecessors are not converted.
    #[test]
    fn test_if_convert_requires_single_merge_pred() {
        // source program
        let input = r#"function @test(v0: bool, v1: bool, v2: i32, v3: i32) -> i32 {
block0(v0: bool, v1: bool, v2: i32, v3: i32):
    branch v0, block1(v1, v2, v3), block4(v2)
block1(v4: bool, v5: i32, v6: i32):
    branch v4, block2(v5, v6), block3(v5, v6)
block2(v7: i32, v8: i32):
    v9 = iadd v7, v8
    jump block5(v9)
block3(v10: i32, v11: i32):
    v12 = isub v10, v11
    jump block5(v12)
block4(v13: i32):
    jump block5(v13)
block5(v14: i32):
    return v14
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&IfConvert);
        program.assert_output(input);
    }

    /// Multiple merge arguments become multiple select instructions.
    #[test]
    fn test_if_convert_multiple_merge_args() {
        // source program
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    branch v0, block1(v1, v2), block2(v1, v2)
block1(v3: i32, v4: i32):
    v5 = iadd v3, v4
    jump block3(v5, v3)
block2(v6: i32, v7: i32):
    v8 = isub v6, v7
    jump block3(v8, v7)
block3(v9: i32, v10: i32):
    v11 = iadd v9, v10
    return v11
}"#;
        // expected output
        let expected = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    v12 = iadd v1, v2
    v13 = isub v1, v2
    v14 = select v0, v12, v13
    v15 = select v0, v1, v2
    jump block3(v14, v15)
block1(v3: i32, v4: i32):
    v5 = iadd v3, v4
    jump block3(v5, v3)
block2(v6: i32, v7: i32):
    v8 = isub v6, v7
    jump block3(v8, v7)
block3(v9: i32, v10: i32):
    v11 = iadd v9, v10
    return v11
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&IfConvert);
        program.assert_output(expected);
    }
}
