use std::collections::HashMap;

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::ControlFlowGraph;
use crate::common::mir::{
    clone_instruction_metadata, instruction_is_speculatable, instruction_map,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Convert simple diamonds into select instructions.
    ///
    /// This removes branches by speculatively executing both sides of a small
    /// conditional and selecting the result with a `select`.
    ///
    /// ```mir
    /// function before(v0: boolean, v1: int32, v2: int32): int32 {
    /// b0(v0: boolean, v1: int32, v2: int32):
    ///     branch v0, b1(v1, v2), b2(v1, v2)
    /// b1(v3: int32, v4: int32):
    ///     v5 = int.add v3, v4
    ///     jump b3(v5)
    /// b2(v6: int32, v7: int32):
    ///     v8 = int.sub v6, v7
    ///     jump b3(v8)
    /// b3(v9: int32):
    ///     return v9
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: boolean, v1: int32, v2: int32): int32 {
    /// b0(v0: boolean, v1: int32, v2: int32):
    ///     v10 = int.add v1, v2
    ///     v11 = int.sub v1, v2
    ///     v12 = select v0, v10, v11
    ///     jump b3(v12)
    /// b1(v3: int32, v4: int32):
    ///     v5 = int.add v3, v4
    ///     jump b3(v5)
    /// b2(v6: int32, v7: int32):
    ///     v8 = int.sub v6, v7
    ///     jump b3(v8)
    /// b3(v9: int32):
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
const BALANCEI_CONVERT_BUDGET: usize = 48;
/// Threshold for treating a branch as highly biased.
const BIASED_BRANCH_RATIO: f64 = 0.90;

impl FunctionPass for IfConvert {
    /// Run if conversion on a function.
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
    then_arguments: Vec<mir::ValueReference>,
    /// Arguments passed to the else block.
    else_arguments: Vec<mir::ValueReference>,
}

/// Run if conversion and return true when changes were made.
fn run_if_convert(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
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
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
) -> Option<IfConvertCandidate> {
    // read header terminator
    let header_block = tree.get(header);
    let header_terminator = tree.get(header_block.terminator);
    let (condition, then_block, else_block, then_arguments, else_arguments) =
        match header_terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => (
                condition.value()?,
                then_target.block.block()?,
                else_target.block.block()?,
                then_target.arguments.clone(),
                else_target.arguments.clone(),
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
    let then_terminator = tree.get(then_block_data.terminator);
    let else_terminator = tree.get(else_block_data.terminator);

    let merge_block = match (then_terminator, else_terminator) {
        (mir::Terminator::Jump { target }, mir::Terminator::Jump { target: other })
            if target.block == other.block =>
        {
            target.block.block()?
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
    tree: &mut mir::Tree,
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
    let then_terminator = tree.get(then_block.terminator);
    let Some(then_merge_args) = jump_arguments(then_terminator) else {
        return false;
    };
    let else_terminator = tree.get(else_block.terminator);
    let Some(else_merge_args) = jump_arguments(else_terminator) else {
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
        let Some(then_value) = then_value.value() else {
            return false;
        };
        let Some(else_value) = else_value.value() else {
            return false;
        };

        let then_value = remap_value(then_value, &then_value_map);
        let else_value = remap_value(else_value, &else_value_map);

        let destination = function.next_typed_value_like(then_value);
        let select = mir::Instruction::Select {
            destination: destination.into(),
            condition: candidate.condition.into(),
            then_value: then_value.into(),
            else_value: else_value.into(),
        };
        let select_id = tree.insert(select);
        new_instructions.push(select_id);
        select_args.push(destination.into());
    }

    // update header block
    let mut header = tree.get(candidate.header).clone();
    header.instructions = new_instructions;
    let new_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: candidate.merge_block.into(),
            arguments: select_args,
        },
    };
    tree.replace(candidate.header, header);
    tree.replace(tree.get(candidate.header).terminator, new_terminator);

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
        let is_balanced = ((1.0 - BIASED_BRANCH_RATIO)..=BIASED_BRANCH_RATIO).contains(&then_ratio)
            && ((1.0 - BIASED_BRANCH_RATIO)..=BIASED_BRANCH_RATIO).contains(&else_ratio);

        if is_balanced {
            return total_cost <= BALANCEI_CONVERT_BUDGET;
        }

        return total_cost <= BASE_CONVERT_BUDGET;
    }

    // fall back to size balance when no profile data is available
    let min_cost = then_cost.min(else_cost);
    if min_cost == 0 {
        return total_cost <= BALANCEI_CONVERT_BUDGET;
    }

    let size_ratio = then_cost.max(else_cost) as f64 / min_cost as f64;
    if size_ratio <= 1.25 {
        return total_cost <= BALANCEI_CONVERT_BUDGET;
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
    profile: Option<&mir::Profile>,
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

    let then_count = profile.edge_count(&then_edge)?;
    let else_count = profile.edge_count(&else_edge)?;

    Some((then_count, else_count))
}

/// Build a remapping of block parameters and instruction destinations.
fn build_value_map(
    function: &mut mir::Function,
    tree: &mir::Tree,
    block: &mir::Block,
    arguments: &[mir::ValueReference],
) -> Option<HashMap<mir::Value, mir::Value>> {
    // validate parameter arity
    if block.parameters.len() != arguments.len() {
        return None;
    }

    // map parameters to incoming arguments
    let mut value_map = HashMap::new();
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        value_map.insert(param.value.value()?, arg.value()?);
    }

    // map instruction destinations to fresh values
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        if let Some(destination) = instruction
            .destination()
            .and_then(|destination| destination.value())
        {
            let new_value = function.next_typed_value_like(destination);
            value_map.insert(destination, new_value);
        }
    }

    Some(value_map)
}

/// Clone a block's instructions into a header instruction list.
fn clone_block_instructions(
    tree: &mut mir::Tree,
    block: &mir::Block,
    value_map: &HashMap<mir::Value, mir::Value>,
    target: &mut Vec<mir::LocalNodeId<mir::Instruction>>,
) {
    // clone instructions in order
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id).clone();
        let cloned = instruction_map(&instruction, value_map, tree);
        let cloned_id = tree.insert(cloned);
        clone_instruction_metadata(tree, instruction_id, cloned_id, value_map);
        target.push(cloned_id);
    }
}

/// Check whether all instructions are speculatable.
fn instructions_speculatable(
    instructions: &[mir::LocalNodeId<mir::Instruction>],
    tree: &mir::Tree,
) -> bool {
    // scan instructions for unsafe operations
    for &instruction_id in instructions {
        let instruction = tree.get(instruction_id);
        if !instruction_is_speculatable(instruction, tree) {
            return false;
        }
    }

    true
}

/// Read jump arguments from a block terminator.
fn jump_arguments(terminator: &mir::Terminator) -> Option<Vec<mir::ValueReference>> {
    match terminator {
        mir::Terminator::Jump { target } => Some(target.arguments.clone()),
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
        let input = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1(v1, v2), b2(v1, v2)
b1(v3: int32, v4: int32):
    v5: int32 = int.add v3, v4
    jump b3(v5)
b2(v6: int32, v7: int32):
    v8: int32 = int.sub v6, v7
    jump b3(v8)
b3(v9: int32):
    return v9
}"#;
        let expected = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    v3: int32 = int.add v1, v2
    v4: int32 = int.sub v1, v2
    v5: int32 = select v0, v3, v4
    jump b3(v5)
b1(v6: int32, v7: int32):
    v8: int32 = int.add v6, v7
    jump b3(v8)
b2(v9: int32, v10: int32):
    v11: int32 = int.sub v9, v10
    jump b3(v11)
b3(v12: int32):
    return v12
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&IfConvert);
        test.assert_output(expected);
    }

    /// Cloned instructions keep memory access metadata with remapped values.
    #[test]
    fn test_if_convert_clones_memory_access_metadata() {
        let input = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1(v1, v2), b2(v1, v2)
b1(v3: int32, v4: int32):
    v5: int32 = int.add v3, v4
    jump b3(v5)
b2(v6: int32, v7: int32):
    v8: int32 = int.sub v6, v7
    jump b3(v8)
b3(v9: int32):
    return v9
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.function_id_by_name("test");
        let function = test.tree.get(function_id);
        let header_block_id = function.blocks[0];
        let then_block_id = function.blocks[1];
        let else_block_id = function.blocks[2];

        let then_instruction = test.instructions_in_block(then_block_id)[0];
        let else_instruction = test.instructions_in_block(else_block_id)[0];
        let then_param = test.tree.get(then_block_id).parameters[0]
            .value
            .value()
            .expect("then block parameter should be concrete");
        let else_param = test.tree.get(else_block_id).parameters[1]
            .value
            .value()
            .expect("else block parameter should be concrete");

        test.insert_pointer_access_with_options(
            then_instruction,
            mir::MemoryAccessKind::Read,
            then_param,
            Some(4),
            false,
            None,
        );
        test.insert_pointer_access_with_options(
            else_instruction,
            mir::MemoryAccessKind::Read,
            else_param,
            Some(4),
            false,
            None,
        );

        test.run_pass(&IfConvert);

        let header_block = test.tree.get(header_block_id);
        let then_arg = header_block.parameters[1]
            .value
            .value()
            .expect("converted then parameter should be concrete");
        let else_arg = header_block.parameters[2]
            .value
            .value()
            .expect("converted else parameter should be concrete");
        let mut saw_then = false;
        let mut saw_else = false;

        for &instruction_id in &header_block.instructions {
            let instruction = test.tree.get(instruction_id);
            match instruction {
                mir::Instruction::Binary {
                    operator: mir::BinaryOperator::Add,
                    ..
                } => {
                    let accesses = test
                        .tree
                        .metadata
                        .memory
                        .memory_accesses(instruction_id)
                        .expect("missing metadata for hoisted add");
                    assert_eq!(accesses.len(), 1);
                    assert_eq!(
                        accesses[0].target,
                        mir::MemoryAccessTarget::Pointer(then_arg)
                    );
                    saw_then = true;
                }
                mir::Instruction::Binary {
                    operator: mir::BinaryOperator::Subtract,
                    ..
                } => {
                    let accesses = test
                        .tree
                        .metadata
                        .memory
                        .memory_accesses(instruction_id)
                        .expect("missing metadata for hoisted sub");
                    assert_eq!(accesses.len(), 1);
                    assert_eq!(
                        accesses[0].target,
                        mir::MemoryAccessTarget::Pointer(else_arg)
                    );
                    saw_else = true;
                }
                _ => {}
            }
        }

        assert!(saw_then);
        assert!(saw_else);
    }

    /// Convert larger diamonds when balanced and speculatable.
    #[test]
    fn test_if_convert_large_balanced_blocks() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    branch v0, b1(v1), b2(v1)
b1(v2: int32):
    v3: int32 = int.add v2, v2
    v4: int32 = int.add v3, v2
    v5: int32 = int.add v4, v2
    v6: int32 = int.add v5, v2
    v7: int32 = int.add v6, v2
    v8: int32 = int.add v7, v2
    v9: int32 = int.add v8, v2
    v10: int32 = int.add v9, v2
    v11: int32 = int.add v10, v2
    jump b3(v11)
b2(v12: int32):
    v13: int32 = int.sub v12, v12
    v14: int32 = int.add v13, v12
    v15: int32 = int.add v14, v12
    v16: int32 = int.add v15, v12
    v17: int32 = int.add v16, v12
    v18: int32 = int.add v17, v12
    v19: int32 = int.add v18, v12
    v20: int32 = int.add v19, v12
    v21: int32 = int.add v20, v12
    jump b3(v21)
b3(v22: int32):
    return v22
}"#;
        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    v2: int32 = int.add v1, v1
    v3: int32 = int.add v2, v1
    v4: int32 = int.add v3, v1
    v5: int32 = int.add v4, v1
    v6: int32 = int.add v5, v1
    v7: int32 = int.add v6, v1
    v8: int32 = int.add v7, v1
    v9: int32 = int.add v8, v1
    v10: int32 = int.add v9, v1
    v11: int32 = int.sub v1, v1
    v12: int32 = int.add v11, v1
    v13: int32 = int.add v12, v1
    v14: int32 = int.add v13, v1
    v15: int32 = int.add v14, v1
    v16: int32 = int.add v15, v1
    v17: int32 = int.add v16, v1
    v18: int32 = int.add v17, v1
    v19: int32 = int.add v18, v1
    v20: int32 = select v0, v10, v19
    jump b3(v20)
b1(v21: int32):
    v22: int32 = int.add v21, v21
    v23: int32 = int.add v22, v21
    v24: int32 = int.add v23, v21
    v25: int32 = int.add v24, v21
    v26: int32 = int.add v25, v21
    v27: int32 = int.add v26, v21
    v28: int32 = int.add v27, v21
    v29: int32 = int.add v28, v21
    v30: int32 = int.add v29, v21
    jump b3(v30)
b2(v31: int32):
    v32: int32 = int.sub v31, v31
    v33: int32 = int.add v32, v31
    v34: int32 = int.add v33, v31
    v35: int32 = int.add v34, v31
    v36: int32 = int.add v35, v31
    v37: int32 = int.add v36, v31
    v38: int32 = int.add v37, v31
    v39: int32 = int.add v38, v31
    v40: int32 = int.add v39, v31
    jump b3(v40)
b3(v41: int32):
    return v41
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&IfConvert);
        test.assert_output(expected);
    }

    /// Skip conversion when branch instructions may trap.
    #[test]
    fn test_if_convert_skips_trapping_ops() {
        let input = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1(v1, v2), b2(v1, v2)
b1(v3: int32, v4: int32):
    v5: int32 = int.div.s v3, v4
    jump b3(v5)
b2(v6: int32, v7: int32):
    v8: int32 = int.sub v6, v7
    jump b3(v8)
b3(v9: int32):
    return v9
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&IfConvert);
        test.assert_output(input);
    }

    /// Skip conversion when a branch target has multiple predecessors.
    #[test]
    fn test_if_convert_requires_single_pred() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    branch v0, b1(v1), b2(v1)
b1(v2: int32):
    jump b3(v2)
b2(v3: int32):
    jump b1(v3)
b3(v4: int32):
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&IfConvert);
        test.assert_output(input);
    }

    /// Merge blocks with extra predecessors are not converted.
    #[test]
    fn test_if_convert_requires_single_merge_pred() {
        // source test
        let input = r#"
function test(v0: boolean, v1: boolean, v2: int32, v3: int32): int32 {
b0(v0: boolean, v1: boolean, v2: int32, v3: int32):
    branch v0, b1(v1, v2, v3), b4(v2)
b1(v4: boolean, v5: int32, v6: int32):
    branch v4, b2(v5, v6), b3(v5, v6)
b2(v7: int32, v8: int32):
    v9: int32 = int.add v7, v8
    jump b5(v9)
b3(v10: int32, v11: int32):
    v12: int32 = int.sub v10, v11
    jump b5(v12)
b4(v13: int32):
    jump b5(v13)
b5(v14: int32):
    return v14
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&IfConvert);
        test.assert_output(input);
    }

    /// Multiple merge arguments become multiple select instructions.
    #[test]
    fn test_if_convert_multiple_merge_args() {
        // source test
        let input = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1(v1, v2), b2(v1, v2)
b1(v3: int32, v4: int32):
    v5: int32 = int.add v3, v4
    jump b3(v5, v3)
b2(v6: int32, v7: int32):
    v8: int32 = int.sub v6, v7
    jump b3(v8, v7)
b3(v9: int32, v10: int32):
    v11: int32 = int.add v9, v10
    return v11
}"#;
        // expected output
        let expected = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    v3: int32 = int.add v1, v2
    v4: int32 = int.sub v1, v2
    v5: int32 = select v0, v3, v4
    v6: int32 = select v0, v1, v2
    jump b3(v5, v6)
b1(v7: int32, v8: int32):
    v9: int32 = int.add v7, v8
    jump b3(v9, v7)
b2(v10: int32, v11: int32):
    v12: int32 = int.sub v10, v11
    jump b3(v12, v11)
b3(v13: int32, v14: int32):
    v15: int32 = int.add v13, v14
    return v15
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&IfConvert);
        test.assert_output(expected);
    }
}
