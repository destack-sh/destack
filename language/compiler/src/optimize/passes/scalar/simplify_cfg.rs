use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{
    ConstantPropagation, DominatorTree, LoopAnalysis, RangeAnalysis, RangeMap, ValueRange,
};
use crate::common::mir::{
    block_parameters_used_outside_block, block_uses_available_in_predecessor,
    clone_instruction_metadata,
};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, apply_substitutions_in_dominated_blocks,
    bool_from_range, build_use_def_maps, build_value_instruction_map, build_value_use_counts,
    constraint_truth_value, evaluate_integer_range_comparison, function_thread_jumps,
    instruction_is_speculatable, instruction_map, is_comparison_operator, substitute_values,
    swap_comparison_operator, terminator_remap, terminator_substitute_uses,
};

/// Return block metadata for canonicalization.
#[derive(Debug, Clone)]
struct ReturnBlockInfo {
    /// Block parameters for the return block.
    params: Vec<mir::Parameter>,
    /// The returned value, if any.
    return_value: Option<mir::ValueReference>,
}

/// Maximum instructions to duplicate during tail duplication.
const MAX_TAIL_DUP_INSTRUCTIONS: usize = 6;
/// Maximum predecessors to duplicate per block.
const MAX_TAIL_DUP_PREIECESSORS: usize = 4;
/// Ratio of total edge count required to duplicate all hot edges.
const TAIL_DUP_HOT_EIGE_RATIO: f64 = 0.70;
/// Ratio of total edge count required to duplicate the hottest edge.
const TAIL_DUP_MIN_EIGE_RATIO: f64 = 0.20;
/// Maximum rounds of CFG simplification before reanalysis.
const MAX_SIMPLIFY_CFG_ITERATIONS: usize = 8;

declare_mir_pass! {
    /// Simplify the control flow graph.
    ///
    /// This pass performs several CFG simplifications:
    /// 1. Branch folding: converts `branch cond, A, B` to `jump` when cond is constant or range proven
    /// 2. Path sensitive threading: threads edges using edge specific range facts
    /// 3. Jump threading: threads jumps through empty or passthrough blocks
    /// 4. Return canonicalization: merges empty return blocks into one
    /// 5. Switch canonicalization: folds constant/range switches and lowers single case switches
    /// 6. Same target folding: replaces branches to the same target with `select` + `jump`
    /// 7. Tail duplication: duplicates small jump targets into jump predecessors
    /// 8. Block merging: merges blocks with single predecessor/successor
    /// 9. Unreachable block elimination: removes blocks not reachable from entry
    /// 10. Critical edge splitting: splits edges from multi successor blocks into multi predecessor blocks
    ///
    /// The pass iterates to a bounded fixed point.
    ///
    /// ```mir
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = true
    ///     branch v1, b1, b2
    /// b1:
    ///     return v0
    /// b2:
    ///     v2 = 0int32
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     return v0
    /// }
    /// ```
    /// ```mir
    /// function beforeSelect(v0: boolean, v1: int32, v2: int32): int32 {
    /// b0(v0: boolean, v1: int32, v2: int32):
    ///     branch v0, b1(v1), b1(v2)
    /// b1(v3: int32):
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function afterSelect(v0: boolean, v1: int32, v2: int32): int32 {
    /// b0(v0: boolean, v1: int32, v2: int32):
    ///     v4 = select v0, v1, v2
    ///     return v4
    /// }
    /// ```
    #[pass(id = "simplify-cfg")]
    pub SimplifyCfg,
    "Simplify control flow graph"
}

impl FunctionPass for SimplifyCfg {
    /// Run CFG simplification on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // run simplify cfg with bounded fixed point
        let changed = run_simplify_cfg(function, tree, ctx.profile(), ctx);

        // select preservation based on CFG changes
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "SimplifyCfg"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "simplify-cfg"
    }
}

/// SimplifyCFG logic. Returns true if changes were made.
fn run_simplify_cfg(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    profile: Option<&mir::ProfileTable>,
    ctx: &PipelineContext<'_>,
) -> bool {
    // track whether any changes were made
    let mut changed = false;

    // keep track of profile guided tail duplication targets
    let mut profiled_tail_dup_targets: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

    // run until fixed point or iteration cap
    let mut iteration = 0;
    loop {
        // refresh analyses for this iteration
        let (constants, ranges, domtree, loop_blocks) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<ConstantPropagation>().clone(),
                analyses.get::<RangeAnalysis>().clone(),
                analyses.get::<DominatorTree>().clone(),
                analyses
                    .get::<LoopAnalysis>()
                    .loops()
                    .iter()
                    .flat_map(|loop_info| loop_info.blocks.iter().copied())
                    .collect::<HashSet<_>>(),
            )
        };

        // phase 1: branch folding
        // converts `branch always_true, A, B` to `jump A`
        let mut changed_this_round =
            fold_branches(function, tree, &constants, &ranges, &loop_blocks);

        // phase 2: path sensitive jump threading
        // threads edges using edge specific range facts
        changed_this_round |=
            thread_edge_conditions(function, tree, &constants, &ranges, &domtree, &loop_blocks);

        // phase 3: jump threading
        // threads jumps through empty blocks
        changed_this_round |= function_thread_jumps(function, tree);

        // phase 4: canonicalize return blocks
        changed_this_round |= canonicalize_return_blocks(function, tree);

        // phase 5: fold branches and checks with identical edges
        changed_this_round |= fold_redundant_edges(function, tree);

        // phase 6: fold branches that share the same target
        changed_this_round |= fold_same_target_branches(function, tree);

        // restart after early control flow rewrites
        if changed_this_round {
            changed = true;

            iteration += 1;
            if iteration >= MAX_SIMPLIFY_CFG_ITERATIONS {
                break;
            }

            continue;
        }

        // phase 7: block merging
        // merges blocks with single predecessor/successor
        if let Some(entry) = function.entry
            && merge_blocks(function, tree, entry, &domtree)
        {
            changed = true;

            iteration += 1;
            if iteration >= MAX_SIMPLIFY_CFG_ITERATIONS {
                break;
            }

            continue;
        }

        // phase 8: eliminate unreachable blocks
        if let Some(entry) = function.entry
            && eliminate_unreachable_blocks(function, tree, entry)
        {
            changed = true;

            iteration += 1;
            if iteration >= MAX_SIMPLIFY_CFG_ITERATIONS {
                break;
            }

            continue;
        }

        // phase 9: tail duplicate small jump targets
        if tail_duplicate_blocks(
            function,
            tree,
            profile,
            &domtree,
            &mut profiled_tail_dup_targets,
        ) {
            changed = true;

            iteration += 1;
            if iteration >= MAX_SIMPLIFY_CFG_ITERATIONS {
                break;
            }

            continue;
        }

        // stop when no changes are made
        break;
    }

    // split critical edges after simplification converges
    changed |= split_critical_edges(function, tree);

    changed
}

/// Fold branches on constant conditions into unconditional jumps.
/// Returns true if any branches were folded.
fn fold_branches(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> bool {
    // track whether any changes were made
    let mut changed = false;

    // build value definitions for boolean detection
    let value_definitions = build_value_instruction_map(function, tree);

    // snapshot block list to avoid borrow conflicts
    let block_ids = function.blocks.clone();

    // fold branches with constant or range proven conditions
    for block_id in block_ids {
        let block = tree.get(block_id).clone();
        let terminator_id = block.terminator;
        let terminator = tree.get(terminator_id).clone();
        let exit_ranges = ranges.exit(block_id);
        let is_range_allowed = !loop_blocks.contains(&block_id);
        match &terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                // resolve condition constant
                let condition_constant = condition
                    .value()
                    .and_then(|condition| constants.constant_at_exit(block_id, condition));
                let condition_value = match condition_constant {
                    Some(mir::Constant::Boolean { value }) => Some(*value),
                    _ => None,
                };
                let condition_value = condition_value
                    .or_else(|| {
                        let condition = condition.value()?;
                        if is_range_allowed {
                            bool_from_range(exit_ranges.get(condition))
                        } else {
                            None
                        }
                    })
                    .or_else(|| assume_truth_value(&block, tree, *condition));

                if let Some(is_true) = condition_value {
                    let target = if is_true {
                        then_target.clone()
                    } else {
                        else_target.clone()
                    };

                    let new_block = block.clone();
                    tree.replace(block_id, new_block);
                    tree.replace(terminator_id, mir::Terminator::Jump { target });
                    changed = true;
                }
            }
            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let condition_value = if is_range_allowed {
                    constraint_truth_value(constraint, exit_ranges)
                } else {
                    None
                };

                if let Some(is_true) = condition_value {
                    let target = if is_true {
                        success.clone()
                    } else {
                        failure.clone()
                    };

                    let new_block = block.clone();
                    tree.replace(block_id, new_block);
                    tree.replace(terminator_id, mir::Terminator::Jump { target });
                    changed = true;
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                // fold switches when value is constant or range restricted
                let constant_value = value
                    .value()
                    .and_then(|value| constants.constant_at_exit(block_id, value));
                let range_value = if is_range_allowed {
                    value.value().and_then(|value| exit_ranges.get(value))
                } else {
                    None
                };
                let is_boolean_value = value_is_boolean(*value, range_value, &value_definitions);
                if let Some(new_terminator) =
                    fold_switch(*value, default, cases, constant_value, range_value)
                {
                    let mut new_block = block.clone();
                    if let mir::Terminator::Switch {
                        value,
                        default,
                        cases,
                    } = &new_terminator
                    {
                        // lower boolean switches into branches
                        if let Some(lowered) =
                            lower_boolean_switch(*value, default, cases, is_boolean_value)
                        {
                            tree.replace(block_id, new_block);
                            tree.replace(terminator_id, lowered);
                            changed = true;
                            continue;
                        }

                        // lower to a branch when a single case remains
                        if let Some((new_instructions, lowered)) = lower_single_case_switch(
                            function,
                            tree,
                            *value,
                            default,
                            cases,
                            range_value,
                        ) {
                            new_block.instructions.extend(new_instructions);
                            tree.replace(terminator_id, lowered);
                        } else {
                            tree.replace(terminator_id, new_terminator);
                        }
                    } else {
                        tree.replace(terminator_id, new_terminator);
                    }
                    tree.replace(block_id, new_block);
                    changed = true;
                    continue;
                }

                // lower boolean switches into branches
                if let Some(new_terminator) =
                    lower_boolean_switch(*value, default, cases, is_boolean_value)
                {
                    let new_block = block.clone();
                    tree.replace(block_id, new_block);
                    tree.replace(terminator_id, new_terminator);
                    changed = true;
                    continue;
                }

                // lower single case switches into branches when safe
                if let Some((new_instructions, new_terminator)) =
                    lower_single_case_switch(function, tree, *value, default, cases, range_value)
                {
                    let mut new_block = block.clone();
                    new_block.instructions.extend(new_instructions);
                    tree.replace(block_id, new_block);
                    tree.replace(terminator_id, new_terminator);
                    changed = true;
                }
            }
            _ => {}
        }
    }

    changed
}

/// Thread edges through empty or condition only blocks using edge specific facts.
fn thread_edge_conditions(
    function: &mir::Function,
    tree: &mut mir::Tree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    domtree: &DominatorTree,
    loop_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> bool {
    // build definition metadata
    let value_def_blocks = build_use_def_maps(function, tree).def_block;

    // build value definition and use maps
    let value_definitions = build_value_instruction_map(function, tree);
    let value_use_counts = build_value_use_counts(function, tree);

    // track whether any changes were made
    let mut changed = false;

    // scan blocks for edge threading opportunities
    for &block_id in &function.blocks {
        if loop_blocks.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);
        let terminator_id = block.terminator;
        let terminator = tree.get(terminator_id);
        let new_terminator = match terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                // resolve then and else edges using edge specific ranges
                let then_edge = resolve_edge_if_available(
                    block_id,
                    *condition,
                    true,
                    then_target,
                    tree,
                    constants,
                    ranges,
                    &value_definitions,
                    &value_use_counts,
                    &value_def_blocks,
                    domtree,
                );
                let else_edge = resolve_edge_if_available(
                    block_id,
                    *condition,
                    false,
                    else_target,
                    tree,
                    constants,
                    ranges,
                    &value_definitions,
                    &value_use_counts,
                    &value_def_blocks,
                    domtree,
                );

                // select rewritten or original edges
                let new_then_target = then_edge.unwrap_or_else(|| then_target.clone());
                let new_else_target = else_edge.unwrap_or_else(|| else_target.clone());

                // update terminator when edges change
                let changed_edge =
                    new_then_target != *then_target || new_else_target != *else_target;
                if changed_edge {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: new_then_target,
                        else_target: new_else_target,
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let _ = (
                    constraint,
                    success,
                    failure,
                    block_id,
                    constants,
                    ranges,
                    &value_definitions,
                    &value_use_counts,
                    &value_def_blocks,
                    domtree,
                );
                None
            }
            _ => None,
        };

        // update block terminator when changes were made
        if let Some(terminator) = new_terminator {
            tree.replace(terminator_id, terminator);
            changed = true;
        }
    }

    changed
}

/// Resolve a single edge and ensure its arguments are available.
#[allow(clippy::too_many_arguments)]
fn resolve_edge_if_available(
    source_block: mir::LocalNodeId<mir::Block>,
    condition: mir::ValueReference,
    is_true: bool,
    target: &mir::BlockTarget,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
    value_use_counts: &HashMap<mir::Value, usize>,
    value_def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> Option<mir::BlockTarget> {
    // resolve the edge target
    let resolved = resolve_edge_target(
        source_block,
        condition,
        is_true,
        target,
        tree,
        constants,
        ranges,
        value_definitions,
        value_use_counts,
    )?;

    // require values to be available at the source block
    if !values_available_in_block(source_block, &resolved.arguments, value_def_blocks, domtree) {
        return None;
    }

    Some(resolved)
}

/// Resolve a single edge to a threaded target using edge specific facts.
#[allow(clippy::too_many_arguments)]
fn resolve_edge_target(
    source_block: mir::LocalNodeId<mir::Block>,
    condition: mir::ValueReference,
    is_true: bool,
    target: &mir::BlockTarget,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
    value_use_counts: &HashMap<mir::Value, usize>,
) -> Option<mir::BlockTarget> {
    // fetch the edge target block
    let target_block = target.block.block()?;
    let block = tree.get(target_block);
    let terminator = tree.get(block.terminator);

    // require an empty or condition only block
    if !is_threadable_condition_block(block, terminator, tree, value_use_counts) {
        return None;
    }

    // require argument counts to match parameters
    if block.parameters.len() != target.arguments.len() {
        return None;
    }

    // build parameter substitutions for this edge
    let mut param_substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    for (param, arg) in block.parameters.iter().zip(target.arguments.iter()) {
        let (Some(param), Some(arg)) = (param.value.value(), arg.value()) else {
            return None;
        };

        param_substitutions.insert(param, arg);
    }

    // build edge specific ranges and apply parameter ranges
    let edge_ranges = edge_ranges_for_condition(
        source_block,
        condition,
        is_true,
        ranges,
        constants,
        value_definitions,
    );
    let mut target_ranges = edge_ranges.clone();
    apply_block_param_ranges_for_edge(block, &target.arguments, &edge_ranges, &mut target_ranges);

    // resolve the target terminator
    let resolved = match terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let condition_value = resolve_condition_value(
                *condition,
                target_block,
                &target_ranges,
                constants,
                value_definitions,
            )?;
            // choose the resolved branch target
            let target = if condition_value {
                then_target
            } else {
                else_target
            };
            let resolved_args = substitute_values(&target.arguments, &param_substitutions);
            Some(mir::BlockTarget {
                block: target.block,
                arguments: resolved_args,
            })
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let condition_value = constraint_truth_value(constraint, &target_ranges)?;
            // choose the resolved check target
            let target = if condition_value { success } else { failure };
            let resolved_args = substitute_values(&target.arguments, &param_substitutions);
            Some(mir::BlockTarget {
                block: target.block,
                arguments: resolved_args,
            })
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let resolved = resolve_switch_target(
                *value,
                target_block,
                default,
                cases,
                constants,
                &target_ranges,
            )?;
            // forward the resolved switch edge
            let resolved_args = substitute_values(&resolved.arguments, &param_substitutions);
            Some(mir::BlockTarget {
                block: resolved.block,
                arguments: resolved_args,
            })
        }
        _ => None,
    }?;

    // require arguments to match the resolved target parameters
    let resolved_block = tree.get(resolved.block.block()?);
    if resolved_block.parameters.len() != resolved.arguments.len() {
        return None;
    }

    Some(resolved)
}

/// Return true when a block is safe to bypass for edge threading.
fn is_threadable_condition_block(
    block: &mir::Block,
    terminator: &mir::Terminator,
    tree: &mir::Tree,
    value_use_counts: &HashMap<mir::Value, usize>,
) -> bool {
    // accept empty blocks
    if block.instructions.is_empty() {
        return true;
    }

    // require a single instruction for condition only blocks
    if block.instructions.len() != 1 {
        return false;
    }

    // fetch the condition value used by the terminator
    let condition_value = match terminator {
        mir::Terminator::Branch { condition, .. } => condition.value(),
        mir::Terminator::Switch { value, .. } => value.value(),
        _ => None,
    };
    let Some(condition_value) = condition_value else {
        return false;
    };

    // require the single instruction to define the condition value
    let instruction_id = block.instructions[0];
    let instruction = tree.get(instruction_id);
    let Some(destination) = instruction.destination() else {
        return false;
    };
    if destination.value() != Some(condition_value) {
        return false;
    }

    // require that the condition is used only by the terminator
    let use_count = value_use_counts.get(&condition_value).copied().unwrap_or(0);
    use_count == 1
}

/// Build edge specific ranges for a branch condition.
fn edge_ranges_for_condition(
    block_id: mir::LocalNodeId<mir::Block>,
    condition: mir::ValueReference,
    is_true: bool,
    ranges: &RangeAnalysis,
    constants: &ConstantPropagation,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
) -> RangeMap {
    // seed ranges from the source block exit
    let mut edge_ranges = ranges.exit(block_id).clone();
    let Some(condition) = condition.value() else {
        return edge_ranges;
    };

    // constrain the condition value for this edge
    let condition_range = ValueRange::Boolean {
        can_be_true: is_true,
        can_be_false: !is_true,
    };
    edge_ranges.insert(condition, condition_range);

    // apply comparison derived constraints when available
    apply_comparison_constraint(
        condition,
        is_true,
        block_id,
        constants,
        value_definitions,
        &mut edge_ranges,
    );

    edge_ranges
}

/// Apply branch comparison constraints to the edge ranges when possible.
fn apply_comparison_constraint(
    condition: mir::Value,
    is_true: bool,
    block_id: mir::LocalNodeId<mir::Block>,
    constants: &ConstantPropagation,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
    edge_ranges: &mut RangeMap,
) {
    // look up the condition definition
    let Some(instruction) = value_definitions.get(&condition) else {
        return;
    };
    let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = instruction
    else {
        return;
    };
    // ignore non comparison instructions
    if !operator.is_comparison() || operator.is_float() {
        return;
    }

    // resolve constant operands when possible
    let left_const = left
        .value()
        .and_then(|left| resolve_integer_constant(left, edge_ranges, constants, block_id));
    let right_const = right
        .value()
        .and_then(|right| resolve_integer_constant(right, edge_ranges, constants, block_id));

    // refine the left operand when the right is constant
    if let (None, Some(right_const)) = (left_const, right_const) {
        let Some(left) = left.value() else {
            return;
        };

        refine_range_for_comparison(edge_ranges, left, *operator, is_true, right_const);
    }

    // refine the right operand when the left is constant
    if let (Some(left_const), None) = (left_const, right_const) {
        let Some(swapped) = swap_comparison_operator(*operator) else {
            return;
        };
        let Some(right) = right.value() else {
            return;
        };

        refine_range_for_comparison(edge_ranges, right, swapped, is_true, left_const);
    }
}

/// Apply block parameter ranges for a single edge.
fn apply_block_param_ranges_for_edge(
    block: &mir::Block,
    arguments: &[mir::ValueReference],
    source_ranges: &RangeMap,
    target_ranges: &mut RangeMap,
) {
    // map each parameter to the range of its incoming argument
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        let Some(param) = param.value.value() else {
            continue;
        };

        if let Some(arg) = arg.value()
            && let Some(range) = source_ranges.get(arg)
        {
            target_ranges.insert(param, range.clone());
        } else {
            target_ranges.remove(param);
        }
    }
}

/// Resolve a boolean condition using ranges or comparison evaluation.
fn resolve_condition_value(
    condition: mir::ValueReference,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeMap,
    constants: &ConstantPropagation,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
) -> Option<bool> {
    let condition = condition.value()?;

    // check constant propagation facts
    let constant = constants
        .constant_at_entry(block_id, condition)
        .or_else(|| constants.constant_at_exit(block_id, condition));
    if let Some(mir::Constant::Boolean { value }) = constant {
        return Some(*value);
    }

    // check range derived booleans
    if let Some(value) = bool_from_range(ranges.get(condition)) {
        return Some(value);
    }

    // evaluate comparison conditions from operand ranges
    let instruction = value_definitions.get(&condition)?;
    let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = instruction
    else {
        return None;
    };
    if !operator.is_comparison() || operator.is_float() {
        return None;
    }

    let left_range = ranges.get(*left)?;
    let right_range = ranges.get(*right)?;
    evaluate_integer_range_comparison(*operator, left_range, right_range)
}

/// Resolve a switch to a single target using edge specific ranges.
fn resolve_switch_target(
    value: mir::ValueReference,
    block_id: mir::LocalNodeId<mir::Block>,
    default: &mir::BlockTarget,
    cases: &[mir::SwitchCase],
    constants: &ConstantPropagation,
    ranges: &RangeMap,
) -> Option<mir::BlockTarget> {
    let value = value.value()?;

    // check constant propagation first
    let constant = constants
        .constant_at_entry(block_id, value)
        .or_else(|| constants.constant_at_exit(block_id, value));
    if let Some(mir::Constant::Int {
        value: constant_value,
        is_signed,
        ..
    }) = constant
    {
        let terminator = resolve_switch_case(*constant_value, default, cases, *is_signed);
        return extract_switch_target(terminator);
    }
    if let Some(mir::Constant::UInt {
        value: constant_value,
        ..
    }) = constant
    {
        let constant_value = i128::try_from(*constant_value).ok()?;
        let terminator = resolve_switch_case(constant_value, default, cases, false);
        return extract_switch_target(terminator);
    }

    // fall back to a single valued integer range
    let ValueRange::Integer {
        min,
        max,
        is_signed,
        ..
    } = ranges.get(value)?
    else {
        return None;
    };
    if min != max {
        return None;
    }

    let terminator = resolve_switch_case(*min, default, cases, *is_signed);
    extract_switch_target(terminator)
}

/// Extract the jump target from a switch resolution terminator.
fn extract_switch_target(terminator: mir::Terminator) -> Option<mir::BlockTarget> {
    match terminator {
        mir::Terminator::Jump { target } => Some(target),
        _ => None,
    }
}

/// Resolve an integer constant from range or constant propagation.
fn resolve_integer_constant(
    value: mir::Value,
    ranges: &RangeMap,
    constants: &ConstantPropagation,
    block_id: mir::LocalNodeId<mir::Block>,
) -> Option<i128> {
    // consult constant propagation first
    let constant = constants
        .constant_at_exit(block_id, value)
        .or_else(|| constants.constant_at_entry(block_id, value));
    if let Some(constant) = constant {
        return integer_constant_to_i128(constant);
    }

    // fall back to range based constants
    let range_constant = ranges.get(value)?.as_constant()?;
    integer_constant_to_i128(&range_constant)
}

/// Convert integer constants into i128 values.
fn integer_constant_to_i128(constant: &mir::Constant) -> Option<i128> {
    match constant {
        mir::Constant::Int { value, .. } => Some(*value),
        mir::Constant::UInt { value, .. } => i128::try_from(*value).ok(),
        _ => None,
    }
}

/// Refine an integer range based on a comparison with a constant.
fn refine_range_for_comparison(
    ranges: &mut RangeMap,
    value: mir::Value,
    operator: mir::BinaryOperator,
    is_true: bool,
    constant: i128,
) {
    // extract the existing integer range
    let Some(range) = ranges.get(value) else {
        return;
    };
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return;
    };
    let min = *min;
    let max = *max;
    let width = *width;
    let is_signed = *is_signed;

    // reject comparisons that do not match signedness
    let expects_signed = matches!(
        operator,
        mir::BinaryOperator::SignedLessThan
            | mir::BinaryOperator::SignedLessEqual
            | mir::BinaryOperator::SignedGreaterThan
            | mir::BinaryOperator::SignedGreaterEqual
    );
    let expects_unsigned = matches!(
        operator,
        mir::BinaryOperator::UnsignedLessThan
            | mir::BinaryOperator::UnsignedLessEqual
            | mir::BinaryOperator::UnsignedGreaterThan
            | mir::BinaryOperator::UnsignedGreaterEqual
    );
    if expects_unsigned && is_signed {
        return;
    }
    if expects_signed && !is_signed {
        return;
    }
    // reject negative constants for unsigned comparisons
    if expects_unsigned && constant < 0 {
        return;
    }

    // compute bounds implied by the comparison
    let new_bounds = comparison_bounds(operator, is_true, constant);
    let Some((bound_min, bound_max)) = new_bounds else {
        return;
    };

    // intersect new bounds with the current range
    let updated_min = min.max(bound_min);
    let updated_max = max.min(bound_max);
    if updated_min > updated_max {
        return;
    }

    // store the refined range
    let updated_range = ValueRange::Integer {
        min: updated_min,
        max: updated_max,
        width,
        is_signed,
    };
    ranges.insert(value, updated_range);
}

/// Return bounds implied by a comparison with a constant.
fn comparison_bounds(
    operator: mir::BinaryOperator,
    is_true: bool,
    constant: i128,
) -> Option<(i128, i128)> {
    match operator {
        mir::BinaryOperator::Equal if is_true => Some((constant, constant)),
        mir::BinaryOperator::NotEqual if !is_true => Some((constant, constant)),
        mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::UnsignedLessThan => {
            if is_true {
                Some((i128::MIN, constant.saturating_sub(1)))
            } else {
                Some((constant, i128::MAX))
            }
        }
        mir::BinaryOperator::SignedLessEqual | mir::BinaryOperator::UnsignedLessEqual => {
            if is_true {
                Some((i128::MIN, constant))
            } else {
                Some((constant.saturating_add(1), i128::MAX))
            }
        }
        mir::BinaryOperator::SignedGreaterThan | mir::BinaryOperator::UnsignedGreaterThan => {
            if is_true {
                Some((constant.saturating_add(1), i128::MAX))
            } else {
                Some((i128::MIN, constant))
            }
        }
        mir::BinaryOperator::SignedGreaterEqual | mir::BinaryOperator::UnsignedGreaterEqual => {
            if is_true {
                Some((constant, i128::MAX))
            } else {
                Some((i128::MIN, constant.saturating_sub(1)))
            }
        }
        _ => None,
    }
}

/// Return true when a block assumes the condition is true.
fn assume_truth_value(
    block: &mir::Block,
    tree: &mir::Tree,
    condition: mir::ValueReference,
) -> Option<bool> {
    for instruction_id in &block.instructions {
        let instruction = tree.get(*instruction_id);
        let mir::Instruction::Assume { condition: assumed } = instruction else {
            continue;
        };

        if *assumed == condition {
            return Some(true);
        }
    }

    None
}

/// Fold switch terminators using constant or range information.
fn fold_switch(
    value: mir::ValueReference,
    default: &mir::BlockTarget,
    cases: &[mir::SwitchCase],
    constant_value: Option<&mir::Constant>,
    range_value: Option<&ValueRange>,
) -> Option<mir::Terminator> {
    // select a single known constant case
    if let Some(mir::Constant::Int {
        value, is_signed, ..
    }) = constant_value
    {
        return Some(resolve_switch_case(*value, default, cases, *is_signed));
    }
    if let Some(mir::Constant::UInt { value, .. }) = constant_value {
        let value = i128::try_from(*value).ok()?;

        return Some(resolve_switch_case(value, default, cases, false));
    }

    // narrow cases by integer range when possible
    let ValueRange::Integer {
        min,
        max,
        is_signed,
        ..
    } = range_value?
    else {
        return None;
    };

    if min == max {
        return Some(resolve_switch_case(*min, default, cases, *is_signed));
    }

    let mut filtered_cases: Vec<mir::SwitchCase> = Vec::new();
    for case in cases {
        let Some(case_value) = case.value.integer() else {
            continue;
        };
        let case_value = if *is_signed {
            case_value
        } else {
            let Ok(case_value) = u128::try_from(case_value) else {
                continue;
            };
            let Ok(case_value) = i128::try_from(case_value) else {
                continue;
            };
            case_value
        };
        if case_value >= *min && case_value <= *max {
            filtered_cases.push(case.clone());
        }
    }

    if filtered_cases.len() == cases.len() {
        return None;
    }

    if filtered_cases.is_empty() {
        return Some(mir::Terminator::Jump {
            target: default.clone(),
        });
    }

    Some(mir::Terminator::Switch {
        value,
        default: default.clone(),
        cases: filtered_cases,
    })
}

/// Resolve a switch into a jump when the value is known.
fn resolve_switch_case(
    value: i128,
    default: &mir::BlockTarget,
    cases: &[mir::SwitchCase],
    is_signed: bool,
) -> mir::Terminator {
    // scan cases for a matching value
    for case in cases {
        // normalize the case value for comparison
        let Some(case_value) = case.value.integer() else {
            continue;
        };
        let case_value = if is_signed {
            case_value
        } else {
            let Ok(case_value) = u128::try_from(case_value) else {
                continue;
            };
            let Ok(case_value) = i128::try_from(case_value) else {
                continue;
            };
            case_value
        };

        // return when the case matches
        if value == case_value {
            return mir::Terminator::Jump {
                target: case.target.clone(),
            };
        }
    }

    // fall back to the default target
    mir::Terminator::Jump {
        target: default.clone(),
    }
}

/// Return true when a value is known to be boolean.
fn value_is_boolean(
    value: mir::ValueReference,
    range_value: Option<&ValueRange>,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
) -> bool {
    // prefer range information when available
    if matches!(range_value, Some(ValueRange::Boolean { .. })) {
        return true;
    }

    // fall back to instruction based detection
    let Some(value) = value.value() else {
        return false;
    };

    let Some(instruction) = value_definitions.get(&value) else {
        return false;
    };

    match instruction {
        mir::Instruction::Const {
            value: mir::Constant::Boolean { .. },
            ..
        } => true,
        mir::Instruction::Binary { operator, .. } => is_comparison_operator(*operator),
        _ => false,
    }
}

/// Lower boolean switches into conditional branches when possible.
fn lower_boolean_switch(
    value: mir::ValueReference,
    default: &mir::BlockTarget,
    cases: &[mir::SwitchCase],
    is_boolean_value: bool,
) -> Option<mir::Terminator> {
    // require a boolean value
    if !is_boolean_value {
        return None;
    };

    // gather case edges for 0 and 1
    let mut case_zero: Option<&mir::SwitchCase> = None;
    let mut case_one: Option<&mir::SwitchCase> = None;
    for case in cases {
        match case.value.integer() {
            Some(0) => case_zero = Some(case),
            Some(1) => case_one = Some(case),
            _ => return None,
        }
    }

    // require at least one boolean case
    if case_zero.is_none() && case_one.is_none() {
        return None;
    }

    // map cases to branch edges
    let then_target = match case_one {
        Some(case) => case.target.clone(),
        None => default.clone(),
    };
    let else_target = match case_zero {
        Some(case) => case.target.clone(),
        None => default.clone(),
    };

    Some(mir::Terminator::Branch {
        condition: value,
        then_target,
        else_target,
    })
}

/// Lower a single case switch into a conditional branch when possible.
fn lower_single_case_switch(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    value: mir::ValueReference,
    default: &mir::BlockTarget,
    cases: &[mir::SwitchCase],
    range_value: Option<&ValueRange>,
) -> Option<(Vec<mir::LocalNodeId<mir::Instruction>>, mir::Terminator)> {
    // require exactly one case
    let [case] = cases else {
        return None;
    };

    // skip when the case is identical to the default
    if case.target == *default {
        return None;
    }

    // require integer range metadata to materialize the constant
    let ValueRange::Integer {
        width, is_signed, ..
    } = range_value?
    else {
        return None;
    };

    // materialize the case constant
    let type_id = tree.int_type(*width, *is_signed);
    let constant_value = function.next_typed_value(type_id);
    let Some(case_value) = case.value.integer() else {
        return None;
    };

    let constant = if *is_signed {
        mir::Constant::Int {
            value: case_value,
            width: *width,
            is_signed: true,
        }
    } else {
        let case_value = u128::try_from(case_value).ok()?;
        mir::Constant::UInt {
            value: case_value,
            width: *width,
        }
    };
    let constant_id = tree.insert(mir::Instruction::Const {
        destination: constant_value.into(),
        value: constant,
    });

    // compare the switch value against the case
    let bool_type = tree.boolean_type();
    let condition_value = function.next_typed_value(bool_type);
    let compare_id = tree.insert(mir::Instruction::Binary {
        destination: condition_value.into(),
        operator: mir::BinaryOperator::Equal,
        left: value,
        right: constant_value.into(),
    });

    // build the conditional branch
    let terminator = mir::Terminator::Branch {
        condition: condition_value.into(),
        then_target: case.target.clone(),
        else_target: default.clone(),
    };

    Some((vec![constant_id, compare_id], terminator))
}

/// Compute canonical return arguments for an edge into a return block.
fn remap_return_edge_arguments(
    target: &mir::BlockTarget,
    return_blocks: &HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo>,
    is_void_return: bool,
) -> Option<Vec<mir::ValueReference>> {
    // lookup return block metadata
    let target_block = target.block.block()?;
    let info = return_blocks.get(&target_block)?;

    // ensure the argument count matches the block parameters
    if info.params.len() != target.arguments.len() {
        return None;
    }

    // drop arguments for void returns
    if is_void_return {
        return Some(Vec::new());
    }

    // require a concrete return value for non void returns
    let return_value = info.return_value?;
    let mut remapped_value = return_value;

    // substitute the return value if it is a block parameter
    for (param, arg) in info.params.iter().zip(target.arguments.iter()) {
        if param.value == return_value {
            remapped_value = *arg;
            break;
        }
    }

    Some(vec![remapped_value])
}

/// Record a return block that could not be remapped.
fn record_kept_return(
    return_blocks: &HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo>,
    kept_returns: &mut HashSet<mir::LocalNodeId<mir::Block>>,
    target: mir::BlockReference,
) {
    // record the target when it is a return block
    if let Some(target) = target.block()
        && return_blocks.contains_key(&target)
    {
        kept_returns.insert(target);
    }
}

/// Check whether any return edge can be remapped into a canonical return block.
fn function_has_remappable_return_edges(
    function: &mir::Function,
    tree: &mir::Tree,
    return_blocks: &HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo>,
    is_void_return: bool,
) -> bool {
    // scan blocks for return targets that can be remapped
    for &block_id in &function.blocks {
        // read the terminator for the current block
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // inspect terminator targets
        match terminator {
            mir::Terminator::Jump { target } => {
                // check jump targets
                if remap_return_edge_arguments(target, return_blocks, is_void_return).is_some() {
                    return true;
                }
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                // check branch targets
                if remap_return_edge_arguments(then_target, return_blocks, is_void_return).is_some()
                    || remap_return_edge_arguments(else_target, return_blocks, is_void_return)
                        .is_some()
                {
                    return true;
                }
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                // check check targets
                if remap_return_edge_arguments(success, return_blocks, is_void_return).is_some()
                    || remap_return_edge_arguments(failure, return_blocks, is_void_return).is_some()
                {
                    return true;
                }
            }
            mir::Terminator::Switch { default, cases, .. } => {
                // check default switch edge
                if remap_return_edge_arguments(default, return_blocks, is_void_return).is_some() {
                    return true;
                }

                // check each switch case edge
                for case in cases {
                    if remap_return_edge_arguments(&case.target, return_blocks, is_void_return)
                        .is_some()
                    {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }

    false
}

/// Canonicalize empty return blocks by routing them to one return block.
fn canonicalize_return_blocks(function: &mut mir::Function, tree: &mut mir::Tree) -> bool {
    // collect candidate return and unreachable blocks
    let mut return_blocks: HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo> = HashMap::new();
    let mut unreachable_blocks: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();
    let mut changed = false;

    // determine return type expectations
    let Some(return_type_id) = function.return_type.ty() else {
        return false;
    };
    let return_type = tree.get(return_type_id);
    let is_void_return = matches!(return_type, mir::Type::Void);

    // scan blocks for empty return and unreachable terminators
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // skip blocks with instructions
        if !block.instructions.is_empty() {
            continue;
        }

        // classify empty terminators
        match terminator {
            mir::Terminator::Return { value } => {
                // skip returns that do not match the function signature
                if is_void_return && value.is_some() {
                    continue;
                }
                if !is_void_return && value.is_none() {
                    continue;
                }

                return_blocks.insert(
                    block_id,
                    ReturnBlockInfo {
                        params: block.parameters.clone(),
                        return_value: *value,
                    },
                );
            }
            mir::Terminator::Unreachable => {
                // record paramless unreachable blocks for merging
                if block.parameters.is_empty() {
                    unreachable_blocks.push(block_id);
                }
            }
            _ => {}
        }
    }

    // merge unreachable blocks into one canonical block
    if unreachable_blocks.len() > 1 {
        let canonical_unreachable = unreachable_blocks[0];
        let redirects: HashMap<_, _> = unreachable_blocks
            .iter()
            .skip(1)
            .map(|block_id| (*block_id, canonical_unreachable))
            .collect();

        changed |= remap_block_targets(function, tree, &redirects);
        function
            .blocks
            .retain(|block_id| !redirects.contains_key(block_id));
    }

    // skip when there is nothing to canonicalize
    if return_blocks.len() <= 1 {
        return changed;
    }

    // skip when no remappable return edges exist
    if !function_has_remappable_return_edges(function, tree, &return_blocks, is_void_return) {
        return changed;
    }

    // build a canonical return block
    let canonical_return = if is_void_return {
        let terminator = tree.insert(mir::Terminator::Return { value: None });
        let block = mir::Block::new(terminator);
        let canonical_id = tree.insert(block);
        function.blocks.push(canonical_id);
        canonical_id
    } else {
        let return_value = function.next_typed_value(return_type_id);
        let param = mir::Parameter {
            value: return_value.into(),
            ty: return_type_id.into(),
        };
        let terminator = tree.insert(mir::Terminator::Return {
            value: Some(return_value.into()),
        });
        let block = mir::Block::with_parameters(vec![param], terminator);
        let canonical_id = tree.insert(block);
        function.blocks.push(canonical_id);
        canonical_id
    };

    // redirect edges into canonical return
    let mut kept_returns: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();
    let mut referenced_returns: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

    // snapshot return block ids for later filtering
    let return_ids: HashSet<_> = return_blocks.keys().copied().collect();

    // rewrite terminators to target the canonical return block
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator_id = block.terminator;
        let terminator = tree.get(block.terminator);

        let new_terminator = rewrite_return_targets(
            terminator,
            &return_blocks,
            canonical_return,
            is_void_return,
            &mut kept_returns,
        );

        if new_terminator != *terminator {
            tree.replace(terminator_id, new_terminator);
            changed = true;
        }
    }

    // recompute referenced return blocks
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };

            if return_ids.contains(&successor) {
                referenced_returns.insert(successor);
            }
        }
    }

    // remove unreachable return blocks
    let entry = function.entry;
    function.blocks.retain(|block_id| {
        // keep the entry block
        if Some(*block_id) == entry {
            return true;
        }

        // keep the canonical return block
        if *block_id == canonical_return {
            return true;
        }

        // keep non return blocks
        if !return_ids.contains(block_id) {
            return true;
        }

        // keep return blocks that could not be remapped
        if kept_returns.contains(block_id) {
            return true;
        }

        // keep return blocks that remain referenced
        referenced_returns.contains(block_id)
    });

    changed
}

/// Rewrite terminator targets that point at return blocks.
fn rewrite_return_targets(
    terminator: &mir::Terminator,
    return_blocks: &HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo>,
    canonical_return: mir::LocalNodeId<mir::Block>,
    is_void_return: bool,
    kept_returns: &mut HashSet<mir::LocalNodeId<mir::Block>>,
) -> mir::Terminator {
    // rewrite return block targets based on the terminator kind
    match terminator {
        mir::Terminator::Jump { target } => {
            // remap jump targets that point at return blocks
            let remapped = remap_return_edge_arguments(target, return_blocks, is_void_return);

            // build the remapped jump when possible
            if let Some(arguments) = remapped {
                return mir::Terminator::Jump {
                    target: mir::BlockTarget {
                        block: canonical_return.into(),
                        arguments,
                    },
                };
            }

            // keep the return block when remapping is not possible
            record_kept_return(return_blocks, kept_returns, target.block);

            terminator.clone()
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            // remap then and else edges into the canonical return block
            let then_remap =
                remap_return_edge_arguments(then_target, return_blocks, is_void_return);
            let else_remap =
                remap_return_edge_arguments(else_target, return_blocks, is_void_return);

            // apply remapped then edge when available
            let mut remapped = false;
            let new_then_target = if let Some(arguments) = then_remap {
                remapped = true;
                mir::BlockTarget {
                    block: canonical_return.into(),
                    arguments,
                }
            } else {
                record_kept_return(return_blocks, kept_returns, then_target.block);
                then_target.clone()
            };

            // apply remapped else edge when available
            let new_else_target = if let Some(arguments) = else_remap {
                remapped = true;
                mir::BlockTarget {
                    block: canonical_return.into(),
                    arguments,
                }
            } else {
                record_kept_return(return_blocks, kept_returns, else_target.block);
                else_target.clone()
            };

            // rebuild the branch when any edge was remapped
            if remapped {
                return mir::Terminator::Branch {
                    condition: *condition,
                    then_target: new_then_target,
                    else_target: new_else_target,
                };
            }

            terminator.clone()
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            // remap check edges into the canonical return block
            let success_remap = remap_return_edge_arguments(success, return_blocks, is_void_return);
            let failure_remap = remap_return_edge_arguments(failure, return_blocks, is_void_return);

            // apply remapped success edge when available
            let mut remapped = false;
            let new_success = if let Some(arguments) = success_remap {
                remapped = true;
                mir::BlockTarget {
                    block: canonical_return.into(),
                    arguments,
                }
            } else {
                record_kept_return(return_blocks, kept_returns, success.block);
                success.clone()
            };

            // apply remapped failure edge when available
            let new_failure = if let Some(arguments) = failure_remap {
                remapped = true;
                mir::BlockTarget {
                    block: canonical_return.into(),
                    arguments,
                }
            } else {
                record_kept_return(return_blocks, kept_returns, failure.block);
                failure.clone()
            };

            // rebuild the check when any edge was remapped
            if remapped {
                return mir::Terminator::Check {
                    constraint: constraint.clone(),
                    success: new_success,
                    failure: new_failure,
                };
            }

            terminator.clone()
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            // remap the default edge into the canonical return block
            let default_remap = remap_return_edge_arguments(default, return_blocks, is_void_return);

            // apply the default remap when available
            let mut remapped = false;
            let new_default = if let Some(arguments) = default_remap {
                remapped = true;
                mir::BlockTarget {
                    block: canonical_return.into(),
                    arguments,
                }
            } else {
                record_kept_return(return_blocks, kept_returns, default.block);
                default.clone()
            };

            // remap switch cases into the canonical return block
            let mut new_cases = Vec::with_capacity(cases.len());
            for case in cases {
                let case_remap =
                    remap_return_edge_arguments(&case.target, return_blocks, is_void_return);

                // apply the case remap when available
                if let Some(arguments) = case_remap {
                    remapped = true;
                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: mir::BlockTarget {
                            block: canonical_return.into(),
                            arguments,
                        },
                    });
                } else {
                    record_kept_return(return_blocks, kept_returns, case.target.block);
                    new_cases.push(case.clone());
                }
            }

            // rebuild the switch when any edge was remapped
            if remapped {
                return mir::Terminator::Switch {
                    value: *value,
                    default: new_default,
                    cases: new_cases,
                };
            }

            terminator.clone()
        }
        _ => terminator.clone(),
    }
}

/// Remap block targets for terminators based on a redirect map.
fn remap_block_targets(
    function: &mir::Function,
    tree: &mut mir::Tree,
    redirects: &HashMap<mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>>,
) -> bool {
    // track whether any changes were made
    let mut changed = false;
    let value_map: HashMap<mir::Value, mir::Value> = HashMap::new();

    for &block_id in &function.blocks {
        if redirects.contains_key(&block_id) {
            continue;
        }

        // remap terminator targets in place
        let block = tree.get(block_id);
        let new_block = block.clone();
        let mut new_terminator = tree.get(new_block.terminator).clone();
        terminator_remap(&mut new_terminator, redirects, &value_map);

        if new_terminator != *tree.get(new_block.terminator) {
            tree.replace(new_block.terminator, new_terminator);
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Fold branches and checks that target identical edges.
fn fold_redundant_edges(function: &mir::Function, tree: &mut mir::Tree) -> bool {
    // track whether any changes were made
    let mut changed = false;

    // simplify terminators with identical targets
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let new_terminator = match terminator {
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } if then_target == else_target => Some(mir::Terminator::Jump {
                target: then_target.clone(),
            }),
            mir::Terminator::Check {
                success, failure, ..
            } if success == failure => Some(mir::Terminator::Jump {
                target: success.clone(),
            }),
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                // drop cases that match the default edge
                let mut filtered_cases: Vec<mir::SwitchCase> = Vec::new();
                let mut changed_cases = false;
                for case in cases {
                    if case.target == *default {
                        changed_cases = true;
                        continue;
                    }
                    filtered_cases.push(case.clone());
                }

                // replace with a jump when all edges are identical
                if filtered_cases.is_empty() {
                    Some(mir::Terminator::Jump {
                        target: default.clone(),
                    })
                } else if changed_cases {
                    Some(mir::Terminator::Switch {
                        value: *value,
                        default: default.clone(),
                        cases: filtered_cases,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(new_terminator) = new_terminator {
            tree.replace(block.terminator, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Fold branches that target the same block into a jump with selects.
fn fold_same_target_branches(function: &mut mir::Function, tree: &mut mir::Tree) -> bool {
    // track whether any changes were made
    let mut changed = false;

    // snapshot blocks to avoid borrowing conflicts with value allocation
    let block_ids = function.blocks.clone();

    for block_id in block_ids {
        // read the block
        let block = tree.get(block_id).clone();
        let terminator = tree.get(block.terminator).clone();
        let mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } = terminator
        else {
            continue;
        };

        // skip branches that do not target the same block
        if then_target != else_target {
            continue;
        }

        // skip malformed branches
        if then_target.arguments.len() != else_target.arguments.len() {
            continue;
        }

        // build new arguments using selects when needed
        let mut new_arguments = Vec::with_capacity(then_target.arguments.len());
        let mut new_block = block.clone();
        let mut inserted_select = false;

        for (then_arg, else_arg) in then_target
            .arguments
            .iter()
            .zip(else_target.arguments.iter())
        {
            // keep identical arguments unchanged
            if then_arg == else_arg {
                new_arguments.push(*then_arg);
                continue;
            }

            // materialize a select for differing arguments
            let Some(then_arg_value) = then_arg.value() else {
                continue;
            };
            let Some(else_arg_value) = else_arg.value() else {
                continue;
            };

            let destination = function.next_typed_value_like(then_arg_value);
            let instruction = mir::Instruction::Select {
                destination: destination.into(),
                condition,
                then_value: then_arg_value.into(),
                else_value: else_arg_value.into(),
            };
            let instruction_id = tree.insert(instruction);
            new_block.instructions.push(instruction_id);
            new_arguments.push(destination.into());
            inserted_select = true;
        }

        // skip when nothing changed
        if !inserted_select && then_target.arguments == else_target.arguments {
            continue;
        }

        // replace the branch with a jump to the shared target
        let new_terminator = mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: then_target.block,
                arguments: new_arguments,
            },
        };
        tree.replace(new_block.terminator, new_terminator);
        tree.replace(block_id, new_block);
        changed = true;
    }

    changed
}

/// Predecessor jump edge for tail duplication.
#[derive(Debug, Clone)]
struct JumpPredecessor {
    /// The predecessor block.
    pred: mir::LocalNodeId<mir::Block>,
    /// Arguments passed to the target block.
    arguments: Vec<mir::ValueReference>,
}

/// Duplicate small jump targets into jump predecessors.
fn tail_duplicate_blocks(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    profile: Option<&mir::ProfileTable>,
    domtree: &DominatorTree,
    profiled_targets: &mut HashSet<mir::LocalNodeId<mir::Block>>,
) -> bool {
    // build definition metadata
    let use_def = build_use_def_maps(function, tree);
    let value_def_blocks = &use_def.def_block;

    // collect predecessor counts and jump predecessors
    let mut predecessor_counts: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    let mut jump_predecessors: HashMap<mir::LocalNodeId<mir::Block>, Vec<JumpPredecessor>> =
        HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors() {
            if let Some(successor) = successor.block() {
                *predecessor_counts.entry(successor).or_insert(0) += 1;
            }
        }

        if let mir::Terminator::Jump { target } = terminator
            && let Some(target_block) = target.block.block()
        {
            jump_predecessors
                .entry(target_block)
                .or_default()
                .push(JumpPredecessor {
                    pred: block_id,
                    arguments: target.arguments.clone(),
                });
        }
    }

    // snapshot block ids to avoid mutation during iteration
    let block_ids = function.blocks.clone();
    let mut changed = false;

    for block_id in block_ids {
        // skip blocks already handled by profile guided duplication
        if profile.is_some() && profiled_targets.contains(&block_id) {
            continue;
        }

        // skip blocks with a single predecessor
        let predecessor_count = predecessor_counts.get(&block_id).copied().unwrap_or(0);
        if predecessor_count <= 1 {
            continue;
        }

        // skip blocks without jump predecessors
        let Some(jump_preds) = jump_predecessors.get(&block_id) else {
            continue;
        };

        // snapshot the block data
        let block = tree.get(block_id).clone();

        // skip entry blocks
        if function.entry == Some(block_id) {
            continue;
        }

        // skip blocks with no work to duplicate
        if block.instructions.is_empty() {
            continue;
        }

        // skip blocks with too many instructions
        if block.instructions.len() > MAX_TAIL_DUP_INSTRUCTIONS {
            continue;
        }

        // skip blocks whose parameters are used outside the block
        if block_parameters_used_outside_block(&block, &use_def.use_blocks, block_id) {
            continue;
        }

        // require speculatable instructions
        let mut all_speculatable = true;
        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id);
            if !instruction_is_speculatable(instruction, tree) {
                all_speculatable = false;
                break;
            }
        }
        if !all_speculatable {
            continue;
        }

        // only duplicate simple terminators
        let terminator = tree.get(block.terminator).clone();
        if !matches!(
            terminator,
            mir::Terminator::Return { .. } | mir::Terminator::Jump { .. }
        ) {
            continue;
        }

        let candidates = select_tail_dup_predecessors(block_id, jump_preds, profile);
        if candidates.is_empty() {
            continue;
        }

        let mut safe_candidates: Vec<JumpPredecessor> = Vec::new();
        for pred in candidates {
            if !block_uses_available_in_predecessor(
                block_id,
                &block,
                tree,
                pred.pred,
                value_def_blocks,
                domtree,
            ) {
                continue;
            }
            safe_candidates.push(pred);
        }

        if safe_candidates.is_empty() {
            continue;
        }

        if safe_candidates.len() > MAX_TAIL_DUP_PREIECESSORS {
            continue;
        }

        // mark when profile selects a strict subset to avoid cold duplication later
        if profile.is_some() && safe_candidates.len() < jump_preds.len() {
            profiled_targets.insert(block_id);
        }

        // ensure all predecessors pass the correct argument counts
        let mut arguments_match = true;
        for pred in &safe_candidates {
            if pred.arguments.len() != block.parameters.len() {
                arguments_match = false;
                break;
            }
        }
        if !arguments_match {
            continue;
        }

        // duplicate the block into each jump predecessor
        for pred in safe_candidates.clone() {
            if pred.pred == block_id {
                continue;
            }

            // build value map for parameters and new instruction values
            let mut value_map: HashMap<mir::Value, mir::Value> = HashMap::new();
            for (param, arg) in block.parameters.iter().zip(pred.arguments.iter()) {
                let (Some(param), Some(arg)) = (param.value.value(), arg.value()) else {
                    continue;
                };

                value_map.insert(param, arg);
            }

            // clone instructions with remapped values
            let mut new_instructions = Vec::with_capacity(block.instructions.len());
            for instruction_id in &block.instructions {
                let instruction = tree.get(*instruction_id).clone();

                if let Some(destination) = instruction.destination().and_then(|value| value.value())
                {
                    let new_destination = function.next_typed_value_like(destination);
                    value_map.insert(destination, new_destination);
                }

                let cloned = instruction_map(&instruction, &value_map, tree);
                let new_id = tree.insert(cloned);
                clone_instruction_metadata(tree, *instruction_id, new_id, &value_map);
                new_instructions.push(new_id);
            }

            // clone the terminator with remapped values
            let new_terminator = terminator_substitute_uses(&terminator, &value_map);

            // create the duplicated block
            let new_terminator_id = tree.insert(new_terminator);
            let mut new_block = mir::Block::new(new_terminator_id);
            new_block.instructions = new_instructions;

            // insert the duplicated block
            let new_block_id = tree.insert(new_block);
            insert_block_after(function, pred.pred, new_block_id);

            // rewrite the predecessor jump to target the duplicated block
            let pred_block = tree.get(pred.pred).clone();
            let updated_pred = pred_block.clone();
            let new_pred_terminator = mir::Terminator::Jump {
                target: mir::BlockTarget {
                    block: new_block_id.into(),
                    arguments: Vec::new(),
                },
            };
            tree.replace(updated_pred.terminator, new_pred_terminator);
            tree.replace(pred.pred, updated_pred);
            changed = true;
        }
    }

    changed
}

/// Return true when all block uses are available at a predecessor.
/// Return true when all values are available in the given block.
fn values_available_in_block(
    block_id: mir::LocalNodeId<mir::Block>,
    values: &[mir::ValueReference],
    value_def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> bool {
    // ensure each value definition dominates the block
    for value in values {
        let Some(value) = value.value() else {
            return false;
        };

        let Some(def_block) = value_def_blocks.get(&value) else {
            return false;
        };

        if !domtree.dominates(*def_block, block_id) {
            return false;
        }
    }

    true
}

/// Select jump predecessors to duplicate using profile guidance when available.
fn select_tail_dup_predecessors(
    block_id: mir::LocalNodeId<mir::Block>,
    jump_predecessors: &[JumpPredecessor],
    profile: Option<&mir::ProfileTable>,
) -> Vec<JumpPredecessor> {
    // fall back to all predecessors when profile data is missing
    let Some(profile) = profile else {
        return jump_predecessors.to_vec();
    };

    // collect edge counts for jump predecessors
    let mut total_count = 0_u64;
    let mut counts: HashMap<mir::LocalNodeId<mir::Block>, u64> = HashMap::new();
    for pred in jump_predecessors {
        let edge = mir::EdgeKey::new(pred.pred, mir::EdgeKind::Jump, block_id);
        let count = profile.edge_count(&edge).map(|c| c.value).unwrap_or(0);
        total_count = total_count.saturating_add(count);
        counts.insert(pred.pred, count);
    }

    // fall back to all predecessors when counts are missing
    if total_count == 0 {
        return jump_predecessors.to_vec();
    }

    // collect hot edges
    let mut hot_preds = Vec::new();
    for pred in jump_predecessors {
        let count = counts.get(&pred.pred).copied().unwrap_or(0);
        let ratio = count as f64 / total_count as f64;
        if ratio >= TAIL_DUP_HOT_EIGE_RATIO {
            hot_preds.push(pred.pred);
        }
    }
    if !hot_preds.is_empty() {
        return jump_predecessors
            .iter()
            .filter(|pred| hot_preds.contains(&pred.pred))
            .cloned()
            .collect();
    }

    // select the hottest edge when it dominates enough
    let mut hottest_pred: Option<mir::LocalNodeId<mir::Block>> = None;
    let mut hottest_count = 0_u64;
    for (pred, count) in &counts {
        if *count > hottest_count || (*count == hottest_count && Some(*pred) < hottest_pred) {
            hottest_pred = Some(*pred);
            hottest_count = *count;
        }
    }

    let Some(hottest_pred) = hottest_pred else {
        return Vec::new();
    };
    let ratio = hottest_count as f64 / total_count as f64;
    if ratio < TAIL_DUP_MIN_EIGE_RATIO {
        return Vec::new();
    }

    jump_predecessors
        .iter()
        .filter(|pred| pred.pred == hottest_pred)
        .cloned()
        .collect()
}

/// Insert a block after a specific block in the function ordering.
fn insert_block_after(
    function: &mut mir::Function,
    after: mir::LocalNodeId<mir::Block>,
    block: mir::LocalNodeId<mir::Block>,
) {
    // insert next to the requested block when possible
    if let Some(index) = function.blocks.iter().position(|id| *id == after) {
        function.blocks.insert(index + 1, block);
        return;
    }

    // fall back to appending when the block is missing
    function.blocks.push(block);
}

/// Split critical edges into their own blocks.
fn split_critical_edges(function: &mut mir::Function, tree: &mut mir::Tree) -> bool {
    // collect predecessor sets for each block
    let mut predecessors: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Block>>,
    > = HashMap::new();
    let mut successor_counts: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // record unique successors for the block
        let mut unique_successors = HashSet::new();
        for successor in terminator.successors() {
            if let Some(successor) = successor.block() {
                unique_successors.insert(successor);
            }
        }

        // store successor count for critical edge checks
        successor_counts.insert(block_id, unique_successors.len());

        // update predecessor sets for each successor
        for successor in unique_successors {
            predecessors.entry(successor).or_default().insert(block_id);
        }
    }

    // snapshot original blocks for iteration
    let original_blocks = function.blocks.clone();

    // track split blocks and changes
    let mut split_cache: HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    > = HashMap::new();
    let mut changed = false;

    for &block_id in &original_blocks {
        // skip blocks with a single successor
        let successor_count = successor_counts.get(&block_id).copied().unwrap_or(0);
        if successor_count <= 1 {
            continue;
        }

        // read the block
        let block = tree.get(block_id).clone();
        let terminator = tree.get(block.terminator).clone();

        // rewrite terminator edges when they are critical
        let new_terminator = match &terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                // split the then edge when critical
                let new_then_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    then_target,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or_else(|| then_target.clone());

                // split the else edge when critical
                let new_else_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    else_target,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or_else(|| else_target.clone());

                if new_then_target != *then_target || new_else_target != *else_target {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: new_then_target,
                        else_target: new_else_target,
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                // split the success edge when critical
                let new_success_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    success,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or_else(|| success.clone());

                // split the failure edge when critical
                let new_failure_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    failure,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or_else(|| failure.clone());

                if new_success_target != *success || new_failure_target != *failure {
                    Some(mir::Terminator::Check {
                        constraint: constraint.clone(),
                        success: new_success_target,
                        failure: new_failure_target,
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                // split the default edge when critical
                let new_default = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    default,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or_else(|| default.clone());

                // split each case edge when critical
                let mut updated_cases = Vec::with_capacity(cases.len());
                let mut remapped = new_default != *default;

                for case in cases {
                    let new_target = split_critical_edge_target(
                        function,
                        tree,
                        block_id,
                        &case.target,
                        &predecessors,
                        &mut split_cache,
                    )
                    .unwrap_or_else(|| case.target.clone());

                    if new_target != case.target {
                        remapped = true;
                    }

                    updated_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: new_target,
                    });
                }

                if remapped {
                    Some(mir::Terminator::Switch {
                        value: *value,
                        default: new_default,
                        cases: updated_cases,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        // update the terminator when it changes
        if let Some(new_terminator) = new_terminator {
            tree.replace(block.terminator, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Split a critical edge target and return the new block id when needed.
fn split_critical_edge_target(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    source: mir::LocalNodeId<mir::Block>,
    target: &mir::BlockTarget,
    predecessors: &HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Block>>>,
    split_cache: &mut HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    >,
) -> Option<mir::BlockTarget> {
    let target_block_id = target.block.block()?;

    // require multiple predecessors to be critical
    let target_preds = predecessors.get(&target_block_id).map_or(0, HashSet::len);
    if target_preds <= 1 {
        return None;
    }

    // reuse previously split edges for the same source and target
    if let Some(existing) = split_cache.get(&(source, target_block_id)) {
        return Some(mir::BlockTarget {
            block: (*existing).into(),
            arguments: target.arguments.clone(),
        });
    }

    // read the target block parameters
    let target_block = tree.get(target_block_id);
    if target_block.parameters.len() != target.arguments.len() {
        return None;
    }

    // build new parameters that mirror the target parameter types
    let mut new_parameters = Vec::with_capacity(target_block.parameters.len());
    let mut new_arguments = Vec::with_capacity(target_block.parameters.len());
    for param in &target_block.parameters {
        let Some(ty) = param.ty.ty() else {
            return None;
        };

        let value = function.next_typed_value(ty);
        new_parameters.push(mir::Parameter {
            value: value.into(),
            ty: ty.into(),
        });
        new_arguments.push(value.into());
    }

    // build the split block
    let new_terminator = tree.insert(mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: target_block_id.into(),
            arguments: new_arguments,
        },
    });
    let new_block = mir::Block::with_parameters(new_parameters, new_terminator);

    // insert the block and record it for reuse
    let new_block_id = tree.insert(new_block);
    insert_block_after(function, source, new_block_id);
    split_cache.insert((source, target_block_id), new_block_id);

    Some(mir::BlockTarget {
        block: new_block_id.into(),
        arguments: target.arguments.clone(),
    })
}

/// Merge blocks where predecessor has single successor and successor has single predecessor.
///
/// If block A unconditionally jumps to block B, and B has no other predecessors,
/// we can merge B's instructions and terminator into A.
///
/// Returns true if any blocks were merged.
fn merge_blocks(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    entry: mir::LocalNodeId<mir::Block>,
    domtree: &DominatorTree,
) -> bool {
    let value_def_blocks = build_use_def_maps(function, tree).def_block;

    // build predecessor count for each block
    let mut predecessor_count: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    for &block_id in &function.blocks {
        predecessor_count.entry(block_id).or_insert(0);
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors() {
            if let Some(successor) = successor.block() {
                *predecessor_count.entry(successor).or_insert(0) += 1;
            }
        }
    }

    let mut changed = false;
    let mut merged_away: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

    // iterate until no more merges possible
    loop {
        let mut merged_this_round = false;

        for &block_id in &function.blocks {
            if merged_away.contains(&block_id) {
                continue;
            }

            // extract info from block without holding borrow
            let (target, arguments, block_clone) = {
                let block = tree.get(block_id);
                let terminator = tree.get(block.terminator);
                let mir::Terminator::Jump { target } = terminator else {
                    continue;
                };
                let Some(target_block) = target.block.block() else {
                    continue;
                };
                (target_block, target.arguments.clone(), block.clone())
            };

            // don't merge into ourselves
            if target == block_id {
                continue;
            }

            // target must have exactly one predecessor (us)
            if predecessor_count.get(&target).copied().unwrap_or(0) != 1 {
                continue;
            }

            // don't merge away the entry block
            if target == entry {
                continue;
            }

            // target must not already be merged away
            if merged_away.contains(&target) {
                continue;
            }

            // extract target block info
            let param_to_arg = {
                let target_block = tree.get(target);

                // require argument counts to match parameters
                if target_block.parameters.len() != arguments.len() {
                    continue;
                }

                // require block uses to be available in the predecessor
                if !block_uses_available_in_predecessor(
                    target,
                    target_block,
                    tree,
                    block_id,
                    &value_def_blocks,
                    domtree,
                ) {
                    continue;
                }

                // build parameter substitutions for the merge
                let param_to_arg: HashMap<mir::Value, mir::Value> = target_block
                    .parameters
                    .iter()
                    .zip(arguments.iter())
                    .filter_map(|(param, arg)| Some((param.value.value()?, arg.value()?)))
                    .collect();
                param_to_arg
            };

            // propagate parameter substitutions into dominated blocks
            apply_substitutions_in_dominated_blocks(function, tree, domtree, target, &param_to_arg);

            // refresh the target block after substitution
            let (target_instructions, target_terminator) = {
                let target_block = tree.get(target);
                (
                    target_block.instructions.clone(),
                    tree.get(target_block.terminator).clone(),
                )
            };

            // merge: append target's instructions and replace our terminator
            let mut new_block = block_clone;
            let terminator_id = new_block.terminator;

            // copy and substitute instructions from target
            for instruction_id in target_instructions {
                let instruction = tree.get(instruction_id).clone();
                let new_id = tree.insert(instruction);
                new_block.instructions.push(new_id);
            }

            tree.replace(block_id, new_block);
            tree.replace(terminator_id, target_terminator);

            // mark target as merged away
            merged_away.insert(target);
            merged_this_round = true;
            changed = true;
        }

        if !merged_this_round {
            break;
        }
    }

    // remove merged blocks from function
    if !merged_away.is_empty() {
        function
            .blocks
            .retain(|block_id| !merged_away.contains(block_id));
    }

    changed
}

/// Return true when block parameters are used outside the block.
/// Eliminate blocks not reachable from the entry block.
/// Returns true if any blocks were removed.
fn eliminate_unreachable_blocks(
    function: &mut mir::Function,
    tree: &mir::Tree,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // find all reachable blocks via BFS from entry
    let mut reachable = HashSet::new();
    let mut worklist = vec![entry];

    while let Some(block_id) = worklist.pop() {
        if !reachable.insert(block_id) {
            continue; // already visited
        }

        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };

            if !reachable.contains(&successor) {
                worklist.push(successor);
            }
        }
    }

    // remove unreachable blocks from the function
    let original_len = function.blocks.len();
    function
        .blocks
        .retain(|&block_id| reachable.contains(&block_id));

    function.blocks.len() != original_len
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;
    use destack_mir as mir;

    /// Unreachable blocks are eliminated from the function.
    #[test]
    fn test_eliminate_unreachable_block() {
        // block1 is empty (just returns), block2 is unreachable
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    jump b1
b1:
    return v0
b2:
    v1: int32 = 2int32
    return v1
}"#;
        // b0's jump threads to return, b1 and b2 become unreachable
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Chains of unreachable blocks are all eliminated.
    #[test]
    fn test_eliminate_unreachable_chain() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    return v0
b1:
    jump b2
b2:
    v1: int32 = 2int32
    return v1
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Branch on constant true folds to unconditional jump to then target.
    #[test]
    fn test_fold_constant_true_branch() {
        // branch on true folds to jump, then threads through empty return block
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1, b2
b1:
    return v1
b2:
    return v2
}"#;
        // branch folds to jump, then threads to return
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 1int32
    v2: int32 = 2int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Branch on constant false folds to unconditional jump to else target.
    #[test]
    fn test_fold_constant_false_branch() {
        // branch on false folds to jump, then threads through empty return block
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = false
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1, b2
b1:
    return v1
b2:
    return v2
}"#;
        // branch folds to jump to block2, then threads to return v2
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = false
    v1: int32 = 1int32
    v2: int32 = 2int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Identical void return blocks are merged.
    #[test]
    fn test_merge_identical_return_blocks() {
        let input = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    return
b2:
    return
}"#;
        let expected = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Non void return blocks are routed through a canonical return block.
    #[test]
    fn test_canonicalize_non_void_return_blocks() {
        let input = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1(v1), b2(v2)
b1(v3: int32):
    return v3
b2(v4: int32):
    return v4
}"#;
        let expected = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    v3: int32 = select v0, v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Return values from parent blocks are forwarded through the canonical return block.
    #[test]
    fn test_canonicalize_return_with_outer_value() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1, b2
b1:
    return v1
b2:
    return v2
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    v3: int32 = select v0, v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Branches on readonly global loads are not folded.
    #[test]
    fn test_preserve_readonly_global_load_branch() {
        let input = r#"
global flag: boolean, readonly = true
function test(): int32 {
b0:
    v0: ref<boolean, raw, readonly> = global.address flag
    v1: boolean = load v0
    v2: int32 = 1int32
    v3: int32 = 2int32
    branch v1, b1, b2
b1:
    return v2
b2:
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_unchanged(input);
    }

    /// Branch on non constant condition is preserved.
    #[test]
    fn test_preserve_non_constant_branch() {
        // v0 is a parameter, not a constant
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1, b2
b1:
    v3: int32 = int.add v1, v2
    return v3
b2:
    v4: int32 = int.sub v2, v1
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_unchanged(input);
    }

    /// Branch on a block parameter constant folds to the selected target.
    #[test]
    fn test_fold_block_param_constant_branch() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: boolean = true
    branch v0, b1(v1), b2(v1)
b1(v2: boolean):
    jump b3(v2)
b2(v3: boolean):
    jump b3(v3)
b3(v4: boolean):
    branch v4, b4, b5
b4:
    v5: int32 = 1int32
    return v5
b5:
    v6: int32 = 2int32
    return v6
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: boolean = true
    v2: int32 = 1int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Constant branch folding makes the else target unreachable.
    #[test]
    fn test_fold_and_eliminate_combined() {
        // branch on true folds to jump to block1, then threads to return
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 42int32
    branch v0, b1, b2
b1:
    return v1
b2:
    v2: int32 = 0int32
    return v2
}"#;
        // branch folds, jump threads through empty block1, block2 eliminated
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 42int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Loop back edges keep loop blocks reachable.
    #[test]
    fn test_preserve_loop_structure() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    jump b1(v0)
b1(v1: int32):
    v2: int32 = 10int32
    v3: boolean = int.lt.s v1, v2
    branch v3, b2, b3
b2:
    v4: int32 = 1int32
    v5: int32 = int.add v1, v4
    jump b1(v5)
b3:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_unchanged(input);
    }

    /// Single block functions with no branches are unchanged.
    #[test]
    fn test_preserve_single_block() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_unchanged(input);
    }

    /// Nested constant branches all fold, making intermediate blocks unreachable.
    #[test]
    fn test_fold_nested_constant_branches() {
        // v0=true to block1, v1=false to block4, block2 and block3 become unreachable
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: boolean = false
    branch v0, b1, b2
b1:
    branch v1, b3, b4
b2:
    v2: int32 = 2int32
    return v2
b3:
    v3: int32 = 3int32
    return v3
b4:
    v4: int32 = 4int32
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: boolean = false
    v2: int32 = 4int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Diamond CFG with non constant condition is preserved.
    #[test]
    fn test_preserve_diamond_cfg() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1, b2
b1:
    jump b3(v1)
b2:
    jump b3(v2)
b3(v3: int32):
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_unchanged(input);
    }

    /// Multiple disconnected unreachable regions are all eliminated.
    #[test]
    fn test_eliminate_multiple_unreachable_regions() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    return v0
b1:
    v1: int32 = 2int32
    jump b2
b2:
    return v1
b3:
    v2: int32 = 3int32
    jump b4
b4:
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Constant branch with block arguments preserves arguments on folded jump.
    #[test]
    fn test_fold_branch_with_block_arguments() {
        let input = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 42int32
    v2: int32 = 0int32
    branch v0, b1(v1), b1(v2)
b1(v3: int32):
    return v3
}"#;
        // after folding branch to jump, block merging merges b1 into b0
        let expected = r#"
function test(): int32 {
b0:
    v0: boolean = true
    v1: int32 = 42int32
    v2: int32 = 0int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Assume conditions fold branches to the assumed target.
    #[test]
    fn test_fold_assume_branch() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    assume v0
    branch v0, b1, b2
b1:
    return v1
b2:
    return v2
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    assume v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Assume conditions fold checks to the success edge.
    #[test]
    fn test_fold_assume_check() {
        let input = r#"
function test(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]): uint32 {
b0(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]):
    v4: boolean = int.lt.u v1, v2
    assume v4
    check bounds.u v1, v2, v3 -> b1, b2
b1:
    return v1
b2:
    unreachable
}"#;
        let expected = r#"
function test(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]): uint32 {
b0(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]):
    v4: boolean = int.lt.u v1, v2
    assume v4
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Check constraints fold even when condition ranges are unknown.
    #[test]
    fn test_fold_check_constraint_truth() {
        let input = r#"
function test(v0: boolean, v1: uint32[4]): uint32 {
b0(v0: boolean, v1: uint32[4]):
    v2: uint32 = 0uint32
    v3: uint32 = 4uint32
    check bounds.u v2, v3, v1 -> b1, b2
b1:
    return v2
b2:
    unreachable
}"#;
        let expected = r#"
function test(v0: boolean, v1: uint32[4]): uint32 {
b0(v0: boolean, v1: uint32[4]):
    v2: uint32 = 0uint32
    v3: uint32 = 4uint32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Jump through empty block is threaded to final target.
    #[test]
    fn test_thread_simple_jump() {
        // block1 and block2 are both empty threadable blocks
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    jump b1
b1:
    jump b2
b2:
    return v0
}"#;
        // b0's jump threads all the way to return, both intermediates become unreachable
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Chain of empty jump blocks all thread to final target.
    #[test]
    fn test_thread_jump_chain() {
        // all intermediate blocks are empty and threadable
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    jump b1
b1:
    jump b2
b2:
    jump b3
b3:
    return v0
}"#;
        // b0's jump threads through entire chain to return
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Block with instructions is not threaded through, but its successor can be.
    #[test]
    fn test_preserve_block_with_instructions() {
        // b1 has instructions so can't be threaded through
        // but after threading b1's jump to return, b0 and b1 merge
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    jump b1
b1:
    v1: int32 = int.add v0, v0
    jump b2
b2:
    return v1
}"#;
        // b1's jump threads to return, then b0 and b1 merge
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = int.add v0, v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Branch targets through empty blocks are threaded.
    #[test]
    fn test_thread_branch_targets() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b4
b3:
    return v1
b4:
    return v2
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    v3: int32 = select v0, v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Check targets thread through empty jump blocks.
    #[test]
    fn test_thread_check_targets() {
        let input = r#"
function test(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]): void {
b0(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]):
    check bounds.u v1, v2, v3 -> b1, b2
b1:
    jump b3
b2:
    jump b4
b3:
    return
b4:
    return
}"#;
        let expected = r#"
function test(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]): void {
b0(v0: boolean, v1: uint32, v2: uint32, v3: uint32[4]):
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Switch edges thread through empty jump blocks.
    #[test]
    fn test_thread_switch_edges() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    switch v0, b1, 0 => b2
b1:
    jump b3
b2:
    jump b4
b3:
    v1: int32 = 1int32
    return v1
b4:
    v2: int32 = 2int32
    return v2
}"#;
        let expected = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    switch v0, b1, 0 => b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Edge specific ranges thread through a condition only block.
    #[test]
    fn test_thread_edge_condition_with_ranges() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 20uint32
    v3: uint32 = select v0, v1, v2
    v4: uint32 = 10uint32
    v5: uint32 = 15uint32
    v6: boolean = int.lt.u v3, v4
    branch v6, b1, b2
b1:
    v7: boolean = int.lt.u v3, v5
    branch v7, b3, b4
b2:
    v8: int32 = 1int32
    return v8
b3:
    v9: int32 = 2int32
    return v9
b4:
    v10: int32 = 3int32
    return v10
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 20uint32
    v3: uint32 = select v0, v1, v2
    v4: uint32 = 10uint32
    v5: uint32 = 15uint32
    v6: boolean = int.lt.u v3, v4
    branch v6, b2, b1
b1:
    v7: int32 = 1int32
    return v7
b2:
    v8: int32 = 2int32
    return v8
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Block with parameters threads through its empty successor to a return.
    #[test]
    fn test_thread_parameterized_block_successor() {
        // block1 has params so can't be threaded through, but block2 is empty and threadable
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1(v1), b1(v2)
b1(v3: int32):
    jump b2
b2:
    return v3
}"#;
        // block1's jump is threaded directly to the return, block2 becomes unreachable
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    v3: int32 = select v0, v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Jump predecessors duplicate a small tail block into the jump edge.
    #[test]
    fn test_tail_duplicate_jump_predecessor() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    v2: int32 = 1int32
    branch v0, b1, b2(v1)
b1:
    jump b2(v1)
b2(v3: int32):
    v4: int32 = int.add v3, v2
    return v4
}"#;
        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    v2: int32 = 1int32
    branch v0, b1, b2(v1)
b1:
    v3: int32 = int.add v1, v2
    return v3
b2(v4: int32):
    v5: int32 = int.add v4, v2
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Profile guided tail duplication duplicates only the hot edge.
    #[test]
    fn test_tail_duplicate_profile_hot_edge() {
        // base cfg with two jump predecessors into a shared tail block
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1(v1), b2(v2)
b1(v3: int32):
    jump b3(v3)
b2(v4: int32):
    jump b3(v4)
b3(v5: int32):
    v6: int32 = int.mul v5, v5
    return v6
}"#;
        // expected cfg after duplicating the hot predecessor only
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 1int32
    v2: int32 = 2int32
    branch v0, b1(v1), b3(v2)
b1(v3: int32):
    jump b2
b2:
    v4: int32 = int.mul v3, v3
    return v4
b3(v5: int32):
    jump b4(v5)
b4(v6: int32):
    v7: int32 = int.mul v6, v6
    return v7
}"#;

        // parse input test
        let mut test = TestProgram::new(input);

        // gather the hot and cold jump predecessors
        let function_id = test.first_function_id();
        let mut function = test.tree.get(function_id).clone();
        let (hot_pred, cold_pred) = test.entry_branch_targets(&function);
        let tail_block = test.jump_target(hot_pred);

        // build dominance data for tail duplication
        let analyses = test.function_analyses(&function);
        let domtree = analyses.get::<DominatorTree>().clone();

        // build the profile table for jump edges
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_jump_edge_profile(&mut profile, hot_pred, tail_block, 100);
        test.record_jump_edge_profile(&mut profile, cold_pred, tail_block, 1);

        // run tail duplication with the profile data
        function.recompute_next_value_id(&test.tree);
        let mut profiled_targets = HashSet::new();
        let changed = tail_duplicate_blocks(
            &mut function,
            &mut test.tree,
            Some(&profile),
            &domtree,
            &mut profiled_targets,
        );

        // persist changes and assert the snapshot
        assert!(changed);
        *test.tree.get_mut(function_id) = function;
        test.assert_output(expected);
    }

    /// Critical edges are split with a dedicated block.
    #[test]
    fn test_split_critical_edge() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    v2: int32 = 1int32
    branch v0, b1(v2), b2(v1)
b1(v3: int32):
    return v3
b2(v4: int32):
    v5: int32 = int.add v4, v2
    jump b1(v5)
}"#;
        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    v2: int32 = 1int32
    branch v0, b1(v2), b3(v1)
b1(v3: int32):
    jump b2(v3)
b2(v4: int32):
    return v4
b3(v5: int32):
    v6: int32 = int.add v5, v2
    jump b2(v6)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Range based branch folding collapses branches on bounded conditions.
    #[test]
    fn test_fold_range_branch_select() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = select v0, v1, v2
    v4: uint32 = 2uint32
    v5: boolean = int.lt.u v3, v4
    v6: int32 = 10int32
    v7: int32 = 20int32
    branch v5, b1, b2
b1:
    return v6
b2:
    return v7
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = select v0, v1, v2
    v4: uint32 = 2uint32
    v5: boolean = int.lt.u v3, v4
    v6: int32 = 10int32
    v7: int32 = 20int32
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Range based switch folding prunes impossible cases.
    #[test]
    fn test_prune_switch_cases_by_range() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = select v0, v1, v2
    switch v3, b3, 0 => b1, 1 => b2, 2 => b4
b1:
    v4: int32 = 10int32
    return v4
b2:
    v5: int32 = 11int32
    return v5
b3:
    v6: int32 = 12int32
    return v6
b4:
    v7: int32 = 13int32
    return v7
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = select v0, v1, v2
    switch v3, b3, 0 => b1, 1 => b2
b1:
    v4: int32 = 10int32
    return v4
b2:
    v5: int32 = 11int32
    return v5
b3:
    v6: int32 = 12int32
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Switches with identical targets fold into a jump.
    #[test]
    fn test_fold_switch_with_identical_targets() {
        let input = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    switch v0, b1, 0 => b1, 1 => b1
b1:
    v1: int32 = 10int32
    return v1
}"#;
        let expected = r#"
function test(v0: uint32): int32 {
b0(v0: uint32):
    v1: int32 = 10int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Single case switches lower to conditional branches.
    #[test]
    fn test_lower_single_case_switch_to_branch() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = select v0, v1, v2
    switch v3, b1, 1 => b2
b1:
    v4: int32 = 10int32
    return v4
b2:
    v5: int32 = 20int32
    return v5
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = select v0, v1, v2
    v4: uint32 = 1uint32
    v5: boolean = int.eq v3, v4
    branch v5, b2, b1
b1:
    v6: int32 = 10int32
    return v6
b2:
    v7: int32 = 20int32
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Boolean switches lower to branches without new compares.
    #[test]
    fn test_lower_boolean_switch_to_branch() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: boolean = int.eq v0, v1
    switch v2, b1, 1 => b2
b1:
    v3: int32 = 1int32
    return v3
b2:
    v4: int32 = 2int32
    return v4
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: boolean = int.eq v0, v1
    branch v2, b2, b1
b1:
    v3: int32 = 1int32
    return v3
b2:
    v4: int32 = 2int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Boolean switches preserve argument passing on lowering.
    #[test]
    fn test_lower_boolean_switch_with_arguments() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    v3: int32 = 7int32
    v4: int32 = 9int32
    switch v2, b1(v3), 1 => b2(v4)
b1(v5: int32):
    v6: int32 = int.add v5, v5
    return v6
b2(v7: int32):
    v8: int32 = int.add v7, v7
    return v8
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    v3: int32 = 7int32
    v4: int32 = 9int32
    branch v2, b2(v4), b1(v3)
b1(v5: int32):
    v6: int32 = int.add v5, v5
    return v6
b2(v7: int32):
    v8: int32 = int.add v7, v7
    return v8
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Boolean switches with two cases lower to a branch.
    #[test]
    fn test_lower_boolean_switch_two_cases() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: boolean = int.eq v0, v1
    switch v2, b1, 0 => b2, 1 => b3
b1:
    v3: int32 = 10int32
    return v3
b2:
    v4: int32 = 20int32
    return v4
b3:
    v5: int32 = 30int32
    return v5
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: boolean = int.eq v0, v1
    branch v2, b2, b1
b1:
    v3: int32 = 20int32
    return v3
b2:
    v4: int32 = 30int32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Single case switches preserve argument passing on lowering.
    #[test]
    fn test_lower_single_case_switch_with_arguments() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 0int32
    v2: int32 = 1int32
    v3: int32 = select v0, v1, v2
    v4: int32 = 4int32
    v5: int32 = 8int32
    switch v3, b1(v4), 1 => b2(v5)
b1(v6: int32):
    v7: int32 = int.mul v6, v6
    return v7
b2(v8: int32):
    v9: int32 = int.mul v8, v8
    return v9
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 0int32
    v2: int32 = 1int32
    v3: int32 = select v0, v1, v2
    v4: int32 = 4int32
    v5: int32 = 8int32
    v6: int32 = 1int32
    v7: boolean = int.eq v3, v6
    branch v7, b2(v5), b1(v4)
b1(v8: int32):
    v9: int32 = int.mul v8, v8
    return v9
b2(v10: int32):
    v11: int32 = int.mul v10, v10
    return v11
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Switch cases that mirror the default edge are dropped.
    #[test]
    fn test_prune_default_switch_case() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    switch v0, b1, 0 => b1, 1 => b2
b1:
    v1: int32 = 10int32
    return v1
b2:
    v2: int32 = 20int32
    return v2
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    switch v0, b1, 1 => b2
b1:
    v1: int32 = 10int32
    return v1
b2:
    v2: int32 = 20int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    /// Passthrough blocks forward parameters directly to their successor.
    #[test]
    fn test_thread_passthrough_block_parameters() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 5int32
    jump b1(v0)
b1(v1: int32):
    jump b2(v1)
b2(v2: int32):
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 5int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);
        test.assert_output(expected);
    }

    fn collect_argument_mismatches(function: &mir::Function, tree: &mir::Tree) -> Vec<String> {
        let mut mismatches = Vec::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            let check_edge = |target: mir::LocalNodeId<mir::Block>,
                              arguments: &[mir::Value],
                              mismatches: &mut Vec<String>| {
                let target_block = tree.get(target);
                if target_block.parameters.len() != arguments.len() {
                    mismatches.push(format!(
                        "block {:?} -> {:?} expected {} args, got {}",
                        block_id,
                        target,
                        target_block.parameters.len(),
                        arguments.len()
                    ));
                }
            };

            match terminator {
                mir::Terminator::Jump { target } => {
                    check_edge(
                        target
                            .block
                            .block()
                            .expect("jump target should be concrete"),
                        &target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => {
                    check_edge(
                        then_target
                            .block
                            .block()
                            .expect("then target should be concrete"),
                        &then_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                    check_edge(
                        else_target
                            .block
                            .block()
                            .expect("else target should be concrete"),
                        &else_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::Check {
                    success, failure, ..
                } => {
                    check_edge(
                        success
                            .block
                            .block()
                            .expect("check success target should be concrete"),
                        &success
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                    check_edge(
                        failure
                            .block
                            .block()
                            .expect("check failure target should be concrete"),
                        &failure
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::Switch { default, cases, .. } => {
                    check_edge(
                        default
                            .block
                            .block()
                            .expect("switch default target should be concrete"),
                        &default
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                    for case in cases {
                        check_edge(
                            case.target
                                .block
                                .block()
                                .expect("switch case target should be concrete"),
                            &case
                                .target
                                .arguments
                                .iter()
                                .filter_map(|value| value.value())
                                .collect::<Vec<_>>(),
                            &mut mismatches,
                        );
                    }
                }
                mir::Terminator::Yield { resume, .. } => {
                    check_edge(
                        resume
                            .block
                            .block()
                            .expect("yield resume target should be concrete"),
                        &resume
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::Invoke {
                    normal_target,
                    unwind_target,
                    ..
                } => {
                    check_edge(
                        normal_target
                            .block
                            .block()
                            .expect("invoke success target should be concrete"),
                        &normal_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                    check_edge(
                        unwind_target
                            .block
                            .block()
                            .expect("invoke unwind target should be concrete"),
                        &unwind_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::InvokeIndirect {
                    normal_target,
                    unwind_target,
                    ..
                } => {
                    check_edge(
                        normal_target
                            .block
                            .block()
                            .expect("invokeIndirect success target should be concrete"),
                        &normal_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                    check_edge(
                        unwind_target
                            .block
                            .block()
                            .expect("invokeIndirect unwind target should be concrete"),
                        &unwind_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::InvokeClass {
                    normal_target,
                    unwind_target,
                    ..
                } => {
                    check_edge(
                        normal_target
                            .block
                            .block()
                            .expect("invokeClass success target should be concrete"),
                        &normal_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                    check_edge(
                        unwind_target
                            .block
                            .block()
                            .expect("invokeClass unwind target should be concrete"),
                        &unwind_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::InvokeInterface {
                    normal_target,
                    unwind_target,
                    ..
                } => {
                    check_edge(
                        normal_target
                            .block
                            .block()
                            .expect("invokeInterface success target should be concrete"),
                        &normal_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                    check_edge(
                        unwind_target
                            .block
                            .block()
                            .expect("invokeInterface unwind target should be concrete"),
                        &unwind_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                            .collect::<Vec<_>>(),
                        &mut mismatches,
                    );
                }
                mir::Terminator::Throw { value: _ } | mir::Terminator::Error => {}
                mir::Terminator::Return { .. }
                | mir::Terminator::Unreachable
                | mir::Terminator::Trap { .. }
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallClass { .. }
                | mir::Terminator::TailCallInterface { .. }
                | mir::Terminator::TailCallIndirect { .. } => {}
            }
        }

        mismatches
    }

    fn collect_undefined_uses(function: &mir::Function, tree: &mir::Tree) -> Vec<String> {
        let mut defined_values: HashSet<mir::Value> = HashSet::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for param in &block.parameters {
                defined_values.insert(
                    param
                        .value
                        .value()
                        .expect("block parameter should be concrete"),
                );
            }

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    defined_values.insert(
                        destination
                            .value()
                            .expect("instruction destination should be concrete"),
                    );
                }
            }
        }

        let mut undefined = Vec::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                let mut uses = instruction.uses();
                if let Some(arguments) = instruction.argument_slice() {
                    uses.extend(tree.get_arguments(arguments).iter().copied());
                }

                for value in uses {
                    let value = value.value().expect("instruction use should be concrete");
                    if !defined_values.contains(&value) {
                        undefined.push(format!(
                            "block {block_id:?} instruction {instruction_id:?} uses {value:?} without definition: {instruction:?}"
                        ));
                    }
                }
            }

            match terminator {
                mir::Terminator::Jump { target } => {
                    for value in target.arguments.iter().filter_map(|value| value.value()) {
                        if !defined_values.contains(&value) {
                            undefined.push(format!(
                                "block {block_id:?} terminator uses {value:?} without definition: {terminator:?}",
                                terminator = terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Branch {
                    condition,
                    then_target,
                    else_target,
                    ..
                } => {
                    let mut uses = Vec::with_capacity(
                        1 + then_target.arguments.len() + else_target.arguments.len(),
                    );
                    uses.push(
                        condition
                            .value()
                            .expect("branch condition should be concrete"),
                    );
                    uses.extend(
                        then_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value()),
                    );
                    uses.extend(
                        else_target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value()),
                    );
                    for value in uses {
                        if !defined_values.contains(&value) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, value, terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Check {
                    constraint,
                    success,
                    failure,
                } => {
                    let mut uses = Vec::new();
                    uses.extend(constraint.uses().iter().filter_map(|value| value.value()));
                    uses.extend(success.arguments.iter().filter_map(|value| value.value()));
                    uses.extend(failure.arguments.iter().filter_map(|value| value.value()));
                    for value in uses {
                        if !defined_values.contains(&value) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, value, terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Switch {
                    value,
                    default,
                    cases,
                    ..
                } => {
                    let value = value.value().expect("switch value should be concrete");
                    if !defined_values.contains(&value) {
                        undefined.push(format!(
                            "block {:?} terminator uses {:?} without definition: {:?}",
                            block_id, value, terminator
                        ));
                    }
                    for argument in default.arguments.iter().filter_map(|value| value.value()) {
                        if !defined_values.contains(&argument) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, argument, terminator
                            ));
                        }
                    }
                    for case in cases {
                        for argument in case
                            .target
                            .arguments
                            .iter()
                            .filter_map(|value| value.value())
                        {
                            if !defined_values.contains(&argument) {
                                undefined.push(format!(
                                    "block {:?} terminator uses {:?} without definition: {:?}",
                                    block_id, argument, terminator
                                ));
                            }
                        }
                    }
                }
                mir::Terminator::Yield { value, resume } => {
                    let value = value.value().expect("yield value should be concrete");
                    if !defined_values.contains(&value) {
                        undefined.push(format!(
                            "block {:?} terminator uses {:?} without definition: {:?}",
                            block_id, value, terminator
                        ));
                    }
                    for argument in resume.arguments.iter().filter_map(|value| value.value()) {
                        if !defined_values.contains(&argument) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, argument, terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Return { value } => {
                    if let Some(value) = value.and_then(|value| value.value())
                        && !defined_values.contains(&value)
                    {
                        undefined.push(format!(
                            "block {:?} terminator uses {:?} without definition: {:?}",
                            block_id, value, terminator
                        ));
                    }
                }
                mir::Terminator::Invoke { .. }
                | mir::Terminator::InvokeIndirect { .. }
                | mir::Terminator::InvokeClass { .. }
                | mir::Terminator::InvokeInterface { .. }
                | mir::Terminator::Throw { value: _ } => {
                    for value in terminator.uses().iter().filter_map(|value| value.value()) {
                        if !defined_values.contains(&value) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, value, terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Unreachable
                | mir::Terminator::Trap { .. }
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallClass { .. }
                | mir::Terminator::TailCallInterface { .. }
                | mir::Terminator::TailCallIndirect { .. }
                | mir::Terminator::Error => {}
            }
        }

        undefined
    }

    /// SimplifyCfg preserves argument counts and definitions.
    #[test]
    fn test_simplify_cfg_preserves_argument_counts() {
        let input = r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1(v1), b2(v2)
b1(v3: int32):
    v4: int32 = int.add v3, v2
    jump b3(v4)
b2(v5: int32):
    v6: int32 = int.add v5, v1
    jump b3(v6)
b3(v7: int32):
    branch v0, b4(v7), b5(v7)
b4(v8: int32):
    return v8
b5(v9: int32):
    return v9
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&SimplifyCfg);

        let function_id = test.function_id_by_name("test");
        let function = test.tree.get(function_id);
        let mismatches = collect_argument_mismatches(function, &test.tree);
        let undefined = collect_undefined_uses(function, &test.tree);
        let output = test.format();

        assert!(
            mismatches.is_empty(),
            "argument mismatches:\n{}\n{output}",
            mismatches.join("\n")
        );
        assert!(
            undefined.is_empty(),
            "undefined values:\n{}\n{output}",
            undefined.join("\n")
        );
    }
}
