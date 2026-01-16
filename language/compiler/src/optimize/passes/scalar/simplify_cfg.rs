use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ConstantPropagation, DominatorTree, LoopAnalysis, RangeAnalysis, RangeMap, ValueRange,
};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, bool_from_range,
    build_use_def_maps, build_value_instruction_map, build_value_use_counts,
    constraint_truth_value, evaluate_integer_range_comparison, function_thread_jumps,
    instruction_is_speculatable, instruction_map, instruction_substitute_uses_in_tree,
    is_comparison_operator, substitute_values, swap_comparison_operator, terminator_remap,
    terminator_substitute_uses,
};

/// Return block metadata for canonicalization.
#[derive(Debug, Clone)]
struct ReturnBlockInfo {
    /// Block parameters for the return block.
    params: Vec<mir::TypedValue>,
    /// The returned value, if any.
    return_value: Option<mir::Value>,
}

/// Maximum instructions to duplicate during tail duplication.
const MAX_TAIL_DUP_INSTRUCTIONS: usize = 6;
/// Maximum predecessors to duplicate per block.
const MAX_TAIL_DUP_PREDECESSORS: usize = 4;
/// Ratio of total edge count required to duplicate all hot edges.
const TAIL_DUP_HOT_EDGE_RATIO: f64 = 0.70;
/// Ratio of total edge count required to duplicate the hottest edge.
const TAIL_DUP_MIN_EDGE_RATIO: f64 = 0.20;
/// Maximum rounds of CFG simplification before re-analysis.
const MAX_SIMPLIFY_CFG_ITERATIONS: usize = 3;

declare_pass! {
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
    /// 10. Critical edge splitting: splits edges from multi-successor blocks into multi-predecessor blocks
    ///
    /// The pass iterates to a bounded fixed point.
    ///
    /// ```mir
    /// function @before(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iconst true
    ///     branch v1, block1, block2
    /// block1:
    ///     return v0
    /// block2:
    ///     v2 = iconst 0i32
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     return v0
    /// }
    /// ```
    /// ```mir
    /// function @before_select(v0: bool, v1: i32, v2: i32) -> i32 {
    /// block0(v0: bool, v1: i32, v2: i32):
    ///     branch v0, block1(v1), block1(v2)
    /// block1(v3: i32):
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after_select(v0: bool, v1: i32, v2: i32) -> i32 {
    /// block0(v0: bool, v1: i32, v2: i32):
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
        tree: &mut mir::NodeTree,
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
    tree: &mut mir::NodeTree,
    profile: Option<&mir::ProfileTable>,
    ctx: &PipelineContext<'_>,
) -> bool {
    // track whether any changes were made
    let mut changed = false;

    // keep track of profile-guided tail duplication targets
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

        // track changes in this iteration
        let mut changed_this_round = false;

        // phase 1: branch folding
        // converts `branch always_true, A, B` -> `jump A`
        changed_this_round |= fold_branches(function, tree, &constants, &ranges, &loop_blocks);

        // phase 2: path sensitive jump threading
        // threads edges using edge specific range facts
        changed_this_round |= thread_edge_conditions(
            function,
            tree,
            &constants,
            &ranges,
            &domtree,
            &loop_blocks,
        );

        // phase 3: jump threading
        // threads jumps through empty blocks
        changed_this_round |= function_thread_jumps(function, tree);

        // phase 4: canonicalize return blocks
        changed_this_round |= canonicalize_return_blocks(function, tree);

        // phase 5: fold branches and checks with identical edges
        changed_this_round |= fold_redundant_edges(function, tree);

        // phase 6: fold branches that share the same target
        changed_this_round |= fold_same_target_branches(function, tree);

        // phase 7: block merging
        // merges blocks with single predecessor/successor
        if let Some(entry) = function.entry {
            changed_this_round |= merge_blocks(function, tree, entry, &domtree);
        }

        // phase 8: eliminate unreachable blocks
        if let Some(entry) = function.entry {
            changed_this_round |= eliminate_unreachable_blocks(function, tree, entry);
        }

        // phase 9: tail duplicate small jump targets
        changed_this_round |=
            tail_duplicate_blocks(
                function,
                tree,
                profile,
                &domtree,
                &mut profiled_tail_dup_targets,
            );

        // stop when no changes are made
        if !changed_this_round {
            break;
        }

        // record that we changed in this iteration
        changed = true;

        // stop when the iteration cap is reached
        iteration += 1;
        if iteration >= MAX_SIMPLIFY_CFG_ITERATIONS {
            break;
        }
    }

    // split critical edges after simplification converges
    changed |= split_critical_edges(function, tree);

    changed
}

/// Fold branches on constant conditions into unconditional jumps.
/// Returns true if any branches were folded.
fn fold_branches(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
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
        let exit_ranges = ranges.exit(block_id);
        let is_range_allowed = !loop_blocks.contains(&block_id);
        match &block.terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                // resolve condition constant
                let condition_constant = constants.constant_at_exit(block_id, *condition);
                let condition_value = match condition_constant {
                    Some(mir::Constant::Boolean { value }) => Some(*value),
                    _ => None,
                };
                let condition_value = condition_value
                    .or_else(|| {
                        if is_range_allowed {
                            bool_from_range(exit_ranges.get(*condition))
                        } else {
                            None
                        }
                    })
                    .or_else(|| assume_truth_value(&block, tree, *condition));

                if let Some(is_true) = condition_value {
                    let (target, arguments) = if is_true {
                        (*then_target, then_arguments.clone())
                    } else {
                        (*else_target, else_arguments.clone())
                    };

                    let mut new_block = block.clone();
                    new_block.terminator = mir::Terminator::Jump { target, arguments };
                    tree.replace(block_id, new_block);
                    changed = true;
                }
            }
            mir::Terminator::Check {
                condition,
                constraint,
                success,
                failure,
                ..
            } => {
                // resolve condition constant
                let condition_constant = constants.constant_at_exit(block_id, *condition);
                let condition_value = match condition_constant {
                    Some(mir::Constant::Boolean { value }) => Some(*value),
                    _ => None,
                };
                let condition_value = condition_value
                    .or_else(|| {
                        if is_range_allowed {
                            bool_from_range(exit_ranges.get(*condition))
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        if is_range_allowed {
                            constraint_truth_value(constraint, exit_ranges)
                        } else {
                            None
                        }
                    })
                    .or_else(|| assume_truth_value(&block, tree, *condition));

                if let Some(is_true) = condition_value {
                    let (target, arguments) = if is_true {
                        (success.target, success.arguments.clone())
                    } else {
                        (failure.target, failure.arguments.clone())
                    };

                    let mut new_block = block.clone();
                    new_block.terminator = mir::Terminator::Jump { target, arguments };
                    tree.replace(block_id, new_block);
                    changed = true;
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                // fold switches when value is constant or range restricted
                let constant_value = constants.constant_at_exit(block_id, *value);
                let range_value = if is_range_allowed {
                    exit_ranges.get(*value)
                } else {
                    None
                };
                let is_boolean_value = value_is_boolean(*value, range_value, &value_definitions);
                if let Some(new_terminator) = fold_switch(
                    *value,
                    *default,
                    default_arguments,
                    cases,
                    constant_value,
                    range_value,
                ) {
                    let mut new_block = block.clone();
                    if let mir::Terminator::Switch {
                        value,
                        default,
                        default_arguments,
                        cases,
                    } = &new_terminator
                    {
                        // lower boolean switches into branches
                        if let Some(lowered) = lower_boolean_switch(
                            *value,
                            *default,
                            default_arguments,
                            cases,
                            is_boolean_value,
                        ) {
                            new_block.terminator = lowered;
                            tree.replace(block_id, new_block);
                            changed = true;
                            continue;
                        }

                        // lower to a branch when a single case remains
                        if let Some((new_instructions, lowered)) = lower_single_case_switch(
                            function,
                            tree,
                            *value,
                            *default,
                            default_arguments,
                            cases,
                            range_value,
                        ) {
                            new_block.instructions.extend(new_instructions);
                            new_block.terminator = lowered;
                        } else {
                            new_block.terminator = new_terminator;
                        }
                    } else {
                        new_block.terminator = new_terminator;
                    }
                    tree.replace(block_id, new_block);
                    changed = true;
                    continue;
                }

                // lower boolean switches into branches
                if let Some(new_terminator) = lower_boolean_switch(
                    *value,
                    *default,
                    default_arguments,
                    cases,
                    is_boolean_value,
                ) {
                    let mut new_block = block.clone();
                    new_block.terminator = new_terminator;
                    tree.replace(block_id, new_block);
                    changed = true;
                    continue;
                }

                // lower single case switches into branches when safe
                if let Some((new_instructions, new_terminator)) = lower_single_case_switch(
                    function,
                    tree,
                    *value,
                    *default,
                    default_arguments,
                    cases,
                    range_value,
                ) {
                    let mut new_block = block.clone();
                    new_block.instructions.extend(new_instructions);
                    new_block.terminator = new_terminator;
                    tree.replace(block_id, new_block);
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
    tree: &mut mir::NodeTree,
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
        let new_terminator = match &block.terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                // resolve then and else edges using edge specific ranges
                let then_edge = resolve_edge_if_available(
                    block_id,
                    *condition,
                    true,
                    *then_target,
                    then_arguments,
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
                    *else_target,
                    else_arguments,
                    tree,
                    constants,
                    ranges,
                    &value_definitions,
                    &value_use_counts,
                    &value_def_blocks,
                    domtree,
                );

                // select rewritten or original edges
                let (new_then_target, new_then_args) =
                    then_edge.unwrap_or((*then_target, then_arguments.clone()));
                let (new_else_target, new_else_args) =
                    else_edge.unwrap_or((*else_target, else_arguments.clone()));

                // update terminator when edges change
                let changed_edge = new_then_target != *then_target
                    || new_else_target != *else_target
                    || new_then_args != *then_arguments
                    || new_else_args != *else_arguments;
                if changed_edge {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: new_then_target,
                        then_arguments: new_then_args,
                        else_target: new_else_target,
                        else_arguments: new_else_args,
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } => {
                // resolve success and failure edges using edge specific ranges
                let success_edge = resolve_edge_if_available(
                    block_id,
                    *condition,
                    true,
                    success.target,
                    &success.arguments,
                    tree,
                    constants,
                    ranges,
                    &value_definitions,
                    &value_use_counts,
                    &value_def_blocks,
                    domtree,
                );
                let failure_edge = resolve_edge_if_available(
                    block_id,
                    *condition,
                    false,
                    failure.target,
                    &failure.arguments,
                    tree,
                    constants,
                    ranges,
                    &value_definitions,
                    &value_use_counts,
                    &value_def_blocks,
                    domtree,
                );

                // select rewritten or original edges
                let (new_success_target, new_success_args) =
                    success_edge.unwrap_or((success.target, success.arguments.clone()));
                let (new_failure_target, new_failure_args) =
                    failure_edge.unwrap_or((failure.target, failure.arguments.clone()));

                // update terminator when edges change
                let changed_edge = new_success_target != success.target
                    || new_failure_target != failure.target
                    || new_success_args != success.arguments
                    || new_failure_args != failure.arguments;
                if changed_edge {
                    Some(mir::Terminator::Check {
                        condition: *condition,
                        constraint: constraint.clone(),
                        success: mir::CheckTarget {
                            target: new_success_target,
                            arguments: new_success_args,
                        },
                        failure: mir::CheckTarget {
                            target: new_failure_target,
                            arguments: new_failure_args,
                        },
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        // update block terminator when changes were made
        if let Some(terminator) = new_terminator {
            let mut new_block = block.clone();
            new_block.terminator = terminator;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Resolve a single edge and ensure its arguments are available.
#[allow(clippy::too_many_arguments)]
fn resolve_edge_if_available(
    source_block: mir::LocalNodeId<mir::Block>,
    condition: mir::Value,
    is_true: bool,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
    tree: &mir::NodeTree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
    value_use_counts: &HashMap<mir::Value, usize>,
    value_def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    // resolve the edge target
    let resolved = resolve_edge_target(
        source_block,
        condition,
        is_true,
        target,
        arguments,
        tree,
        constants,
        ranges,
        value_definitions,
        value_use_counts,
    )?;

    // require values to be available at the source block
    if !values_available_in_block(source_block, &resolved.1, value_def_blocks, domtree) {
        return None;
    }

    Some(resolved)
}

/// Resolve a single edge to a threaded target using edge specific facts.
#[allow(clippy::too_many_arguments)]
fn resolve_edge_target(
    source_block: mir::LocalNodeId<mir::Block>,
    condition: mir::Value,
    is_true: bool,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
    tree: &mir::NodeTree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
    value_use_counts: &HashMap<mir::Value, usize>,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    // fetch the edge target block
    let block = tree.get(target);

    // require an empty or condition only block
    if !is_threadable_condition_block(block, &block.terminator, tree, value_use_counts) {
        return None;
    }

    // require argument counts to match parameters
    if block.parameters.len() != arguments.len() {
        return None;
    }

    // build parameter substitutions for this edge
    let mut param_substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        param_substitutions.insert(param.value, *arg);
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
    apply_block_param_ranges_for_edge(block, arguments, &edge_ranges, &mut target_ranges);

    // resolve the target terminator
    let resolved = match &block.terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let condition_value = resolve_condition_value(
                *condition,
                target,
                &target_ranges,
                constants,
                value_definitions,
            )?;
            // choose the resolved branch target
            let (target, args) = if condition_value {
                (*then_target, then_arguments.as_slice())
            } else {
                (*else_target, else_arguments.as_slice())
            };
            let resolved_args = substitute_values(args, &param_substitutions);
            Some((target, resolved_args))
        }
        mir::Terminator::Check {
            condition,
            success,
            failure,
            ..
        } => {
            let condition_value = resolve_condition_value(
                *condition,
                target,
                &target_ranges,
                constants,
                value_definitions,
            )?;
            // choose the resolved check target
            let (target, args) = if condition_value {
                (success.target, success.arguments.as_slice())
            } else {
                (failure.target, failure.arguments.as_slice())
            };
            let resolved_args = substitute_values(args, &param_substitutions);
            Some((target, resolved_args))
        }
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let resolved = resolve_switch_target(
                *value,
                target,
                *default,
                default_arguments,
                cases,
                constants,
                &target_ranges,
            )?;
            // forward the resolved switch edge
            let (target, args) = resolved;
            let resolved_args = substitute_values(&args, &param_substitutions);
            Some((target, resolved_args))
        }
        _ => None,
    }?;

    // require arguments to match the resolved target parameters
    let resolved_block = tree.get(resolved.0);
    if resolved_block.parameters.len() != resolved.1.len() {
        return None;
    }

    Some(resolved)
}

/// Return true when a block is safe to bypass for edge threading.
fn is_threadable_condition_block(
    block: &mir::Block,
    terminator: &mir::Terminator,
    tree: &mir::NodeTree,
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
        mir::Terminator::Branch { condition, .. } => Some(*condition),
        mir::Terminator::Check { condition, .. } => Some(*condition),
        mir::Terminator::Switch { value, .. } => Some(*value),
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
    if destination != condition_value {
        return false;
    }

    // require that the condition is used only by the terminator
    let use_count = value_use_counts.get(&destination).copied().unwrap_or(0);
    use_count == 1
}

/// Build edge specific ranges for a branch condition.
fn edge_ranges_for_condition(
    block_id: mir::LocalNodeId<mir::Block>,
    condition: mir::Value,
    is_true: bool,
    ranges: &RangeAnalysis,
    constants: &ConstantPropagation,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
) -> RangeMap {
    // seed ranges from the source block exit
    let mut edge_ranges = ranges.exit(block_id).clone();

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
    let left_const = resolve_integer_constant(*left, edge_ranges, constants, block_id);
    let right_const = resolve_integer_constant(*right, edge_ranges, constants, block_id);

    // refine the left operand when the right is constant
    if let (None, Some(right_const)) = (left_const, right_const) {
        refine_range_for_comparison(edge_ranges, *left, *operator, is_true, right_const);
    }

    // refine the right operand when the left is constant
    if let (Some(left_const), None) = (left_const, right_const) {
        let Some(swapped) = swap_comparison_operator(*operator) else {
            return;
        };
        refine_range_for_comparison(edge_ranges, *right, swapped, is_true, left_const);
    }
}

/// Apply block parameter ranges for a single edge.
fn apply_block_param_ranges_for_edge(
    block: &mir::Block,
    arguments: &[mir::Value],
    source_ranges: &RangeMap,
    target_ranges: &mut RangeMap,
) {
    // map each parameter to the range of its incoming argument
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        if let Some(range) = source_ranges.get(*arg) {
            target_ranges.insert(param.value, range.clone());
        } else {
            target_ranges.remove(param.value);
        }
    }
}

/// Resolve a boolean condition using ranges or comparison evaluation.
fn resolve_condition_value(
    condition: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeMap,
    constants: &ConstantPropagation,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
) -> Option<bool> {
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
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    default: mir::LocalNodeId<mir::Block>,
    default_arguments: &[mir::Value],
    cases: &[mir::SwitchCase],
    constants: &ConstantPropagation,
    ranges: &RangeMap,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
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
        let terminator = resolve_switch_case(
            *constant_value as i128,
            default,
            default_arguments,
            cases,
            *is_signed,
        );
        return extract_switch_target(terminator);
    }
    if let Some(mir::Constant::UInt {
        value: constant_value,
        ..
    }) = constant
    {
        let terminator = resolve_switch_case(
            *constant_value as i128,
            default,
            default_arguments,
            cases,
            false,
        );
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

    let terminator = resolve_switch_case(*min, default, default_arguments, cases, *is_signed);
    extract_switch_target(terminator)
}

/// Extract the jump target from a switch resolution terminator.
fn extract_switch_target(
    terminator: mir::Terminator,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    match terminator {
        mir::Terminator::Jump { target, arguments } => Some((target, arguments)),
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
        mir::Constant::Int { value, .. } => Some(*value as i128),
        mir::Constant::UInt { value, .. } => Some(*value as i128),
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
    tree: &mir::NodeTree,
    condition: mir::Value,
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
    value: mir::Value,
    default: mir::LocalNodeId<mir::Block>,
    default_arguments: &[mir::Value],
    cases: &[mir::SwitchCase],
    constant_value: Option<&mir::Constant>,
    range_value: Option<&ValueRange>,
) -> Option<mir::Terminator> {
    // select a single known constant case
    if let Some(mir::Constant::Int {
        value, is_signed, ..
    }) = constant_value
    {
        return Some(resolve_switch_case(
            *value as i128,
            default,
            default_arguments,
            cases,
            *is_signed,
        ));
    }
    if let Some(mir::Constant::UInt { value, .. }) = constant_value {
        return Some(resolve_switch_case(
            *value as i128,
            default,
            default_arguments,
            cases,
            false,
        ));
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
        return Some(resolve_switch_case(
            *min,
            default,
            default_arguments,
            cases,
            *is_signed,
        ));
    }

    let mut filtered_cases: Vec<mir::SwitchCase> = Vec::new();
    for case in cases {
        let case_value = if *is_signed {
            case.value as i128
        } else {
            case.value as u64 as i128
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
            target: default,
            arguments: default_arguments.to_vec(),
        });
    }

    Some(mir::Terminator::Switch {
        value,
        default,
        default_arguments: default_arguments.to_vec(),
        cases: filtered_cases,
    })
}

/// Resolve a switch into a jump when the value is known.
fn resolve_switch_case(
    value: i128,
    default: mir::LocalNodeId<mir::Block>,
    default_arguments: &[mir::Value],
    cases: &[mir::SwitchCase],
    is_signed: bool,
) -> mir::Terminator {
    // scan cases for a matching value
    for case in cases {
        // normalize the case value for comparison
        let case_value = if is_signed {
            case.value as i128
        } else {
            case.value as u64 as i128
        };

        // return when the case matches
        if value == case_value {
            return mir::Terminator::Jump {
                target: case.target,
                arguments: case.arguments.clone(),
            };
        }
    }

    // fall back to the default target
    mir::Terminator::Jump {
        target: default,
        arguments: default_arguments.to_vec(),
    }
}

/// Return true when a value is known to be boolean.
fn value_is_boolean(
    value: mir::Value,
    range_value: Option<&ValueRange>,
    value_definitions: &HashMap<mir::Value, mir::Instruction>,
) -> bool {
    // prefer range information when available
    if matches!(range_value, Some(ValueRange::Boolean { .. })) {
        return true;
    }

    // fall back to instruction based detection
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
    value: mir::Value,
    default: mir::LocalNodeId<mir::Block>,
    default_arguments: &[mir::Value],
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
        match case.value {
            0 => case_zero = Some(case),
            1 => case_one = Some(case),
            _ => return None,
        }
    }

    // require at least one boolean case
    if case_zero.is_none() && case_one.is_none() {
        return None;
    }

    // map cases to branch edges
    let (then_target, then_arguments) = match case_one {
        Some(case) => (case.target, case.arguments.clone()),
        None => (default, default_arguments.to_vec()),
    };
    let (else_target, else_arguments) = match case_zero {
        Some(case) => (case.target, case.arguments.clone()),
        None => (default, default_arguments.to_vec()),
    };

    Some(mir::Terminator::Branch {
        condition: value,
        then_target,
        then_arguments,
        else_target,
        else_arguments,
    })
}

/// Lower a single case switch into a conditional branch when possible.
fn lower_single_case_switch(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    value: mir::Value,
    default: mir::LocalNodeId<mir::Block>,
    default_arguments: &[mir::Value],
    cases: &[mir::SwitchCase],
    range_value: Option<&ValueRange>,
) -> Option<(Vec<mir::LocalNodeId<mir::Instruction>>, mir::Terminator)> {
    // require exactly one case
    let [case] = cases else {
        return None;
    };

    // skip when the case is identical to the default
    if case.target == default && case.arguments == default_arguments {
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
    let constant_value = function.next_value();
    let constant = if *is_signed {
        mir::Constant::Int {
            value: case.value,
            width: *width,
            is_signed: true,
        }
    } else {
        mir::Constant::UInt {
            value: case.value as u64,
            width: *width,
        }
    };
    let constant_id = tree.insert(mir::Instruction::Const {
        destination: constant_value,
        value: constant,
    });

    // compare the switch value against the case
    let condition_value = function.next_value();
    let compare_id = tree.insert(mir::Instruction::Binary {
        destination: condition_value,
        operator: mir::BinaryOperator::Equal,
        left: value,
        right: constant_value,
    });

    // build the conditional branch
    let terminator = mir::Terminator::Branch {
        condition: condition_value,
        then_target: case.target,
        then_arguments: case.arguments.clone(),
        else_target: default,
        else_arguments: default_arguments.to_vec(),
    };

    Some((vec![constant_id, compare_id], terminator))
}

/// Compute canonical return arguments for an edge into a return block.
fn remap_return_edge_arguments(
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
    return_blocks: &HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo>,
    is_void_return: bool,
) -> Option<Vec<mir::Value>> {
    // lookup return block metadata
    let info = return_blocks.get(&target)?;

    // ensure the argument count matches the block parameters
    if info.params.len() != arguments.len() {
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
    for (param, arg) in info.params.iter().zip(arguments.iter()) {
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
    target: mir::LocalNodeId<mir::Block>,
) {
    // record the target when it is a return block
    if return_blocks.contains_key(&target) {
        kept_returns.insert(target);
    }
}

/// Check whether any return edge can be remapped into a canonical return block.
fn function_has_remappable_return_edges(
    function: &mir::Function,
    tree: &mir::NodeTree,
    return_blocks: &HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo>,
    is_void_return: bool,
) -> bool {
    // scan blocks for return targets that can be remapped
    for &block_id in &function.blocks {
        // read the terminator for the current block
        let terminator = &tree.get(block_id).terminator;

        // inspect terminator targets
        match terminator {
            mir::Terminator::Jump { target, arguments } => {
                // check jump targets
                if remap_return_edge_arguments(*target, arguments, return_blocks, is_void_return)
                    .is_some()
                {
                    return true;
                }
            }
            mir::Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                // check branch targets
                if remap_return_edge_arguments(
                    *then_target,
                    then_arguments,
                    return_blocks,
                    is_void_return,
                )
                .is_some()
                    || remap_return_edge_arguments(
                        *else_target,
                        else_arguments,
                        return_blocks,
                        is_void_return,
                    )
                    .is_some()
                {
                    return true;
                }
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                // check check targets
                if remap_return_edge_arguments(
                    success.target,
                    &success.arguments,
                    return_blocks,
                    is_void_return,
                )
                .is_some()
                    || remap_return_edge_arguments(
                        failure.target,
                        &failure.arguments,
                        return_blocks,
                        is_void_return,
                    )
                    .is_some()
                {
                    return true;
                }
            }
            mir::Terminator::Switch {
                default,
                default_arguments,
                cases,
                ..
            } => {
                // check default switch edge
                if remap_return_edge_arguments(
                    *default,
                    default_arguments,
                    return_blocks,
                    is_void_return,
                )
                .is_some()
                {
                    return true;
                }

                // check each switch case edge
                for case in cases {
                    if remap_return_edge_arguments(
                        case.target,
                        &case.arguments,
                        return_blocks,
                        is_void_return,
                    )
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
fn canonicalize_return_blocks(function: &mut mir::Function, tree: &mut mir::NodeTree) -> bool {
    // collect candidate return and unreachable blocks
    let mut return_blocks: HashMap<mir::LocalNodeId<mir::Block>, ReturnBlockInfo> = HashMap::new();
    let mut unreachable_blocks: Vec<mir::LocalNodeId<mir::Block>> = Vec::new();

    // determine return type expectations
    let return_type = tree.get(function.return_type);
    let is_void_return = matches!(return_type, mir::Type::Void);

    // scan blocks for empty return and unreachable terminators
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // skip blocks with instructions
        if !block.instructions.is_empty() {
            continue;
        }

        // classify empty terminators
        match &block.terminator {
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
    let mut changed = false;
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
        let block = mir::Block::new();
        let mut block = block;
        block.terminator = mir::Terminator::Return { value: None };
        let canonical_id = tree.insert(block);
        function.blocks.push(canonical_id);
        canonical_id
    } else {
        let return_value = function.next_value();
        let param = mir::TypedValue::new(return_value, function.return_type);
        let mut block = mir::Block::with_parameters(vec![param]);
        block.terminator = mir::Terminator::Return {
            value: Some(return_value),
        };
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
        let mut new_block = block.clone();

        let new_terminator = rewrite_return_targets(
            &block.terminator,
            &return_blocks,
            canonical_return,
            is_void_return,
            &mut kept_returns,
        );

        if new_terminator != block.terminator {
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    // recompute referenced return blocks
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for successor in block.terminator.successors() {
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
        mir::Terminator::Jump { target, arguments } => {
            // remap jump targets that point at return blocks
            let remapped =
                remap_return_edge_arguments(*target, arguments, return_blocks, is_void_return);

            // build the remapped jump when possible
            if let Some(arguments) = remapped {
                return mir::Terminator::Jump {
                    target: canonical_return,
                    arguments,
                };
            }

            // keep the return block when remapping is not possible
            record_kept_return(return_blocks, kept_returns, *target);

            terminator.clone()
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            // remap then and else edges into the canonical return block
            let then_remap = remap_return_edge_arguments(
                *then_target,
                then_arguments,
                return_blocks,
                is_void_return,
            );
            let else_remap = remap_return_edge_arguments(
                *else_target,
                else_arguments,
                return_blocks,
                is_void_return,
            );

            // apply remapped then edge when available
            let mut remapped = false;
            let (new_then_target, new_then_args) = if let Some(arguments) = then_remap {
                remapped = true;
                (canonical_return, arguments)
            } else {
                record_kept_return(return_blocks, kept_returns, *then_target);
                (*then_target, then_arguments.clone())
            };

            // apply remapped else edge when available
            let (new_else_target, new_else_args) = if let Some(arguments) = else_remap {
                remapped = true;
                (canonical_return, arguments)
            } else {
                record_kept_return(return_blocks, kept_returns, *else_target);
                (*else_target, else_arguments.clone())
            };

            // rebuild the branch when any edge was remapped
            if remapped {
                return mir::Terminator::Branch {
                    condition: *condition,
                    then_target: new_then_target,
                    then_arguments: new_then_args,
                    else_target: new_else_target,
                    else_arguments: new_else_args,
                };
            }

            terminator.clone()
        }
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            // remap check edges into the canonical return block
            let success_remap = remap_return_edge_arguments(
                success.target,
                &success.arguments,
                return_blocks,
                is_void_return,
            );
            let failure_remap = remap_return_edge_arguments(
                failure.target,
                &failure.arguments,
                return_blocks,
                is_void_return,
            );

            // apply remapped success edge when available
            let mut remapped = false;
            let (new_success_target, new_success_args) = if let Some(arguments) = success_remap {
                remapped = true;
                (canonical_return, arguments)
            } else {
                record_kept_return(return_blocks, kept_returns, success.target);
                (success.target, success.arguments.clone())
            };

            // apply remapped failure edge when available
            let (new_failure_target, new_failure_args) = if let Some(arguments) = failure_remap {
                remapped = true;
                (canonical_return, arguments)
            } else {
                record_kept_return(return_blocks, kept_returns, failure.target);
                (failure.target, failure.arguments.clone())
            };

            // rebuild the check when any edge was remapped
            if remapped {
                return mir::Terminator::Check {
                    condition: *condition,
                    constraint: constraint.clone(),
                    success: mir::CheckTarget {
                        target: new_success_target,
                        arguments: new_success_args,
                    },
                    failure: mir::CheckTarget {
                        target: new_failure_target,
                        arguments: new_failure_args,
                    },
                };
            }

            terminator.clone()
        }
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            // remap the default edge into the canonical return block
            let default_remap = remap_return_edge_arguments(
                *default,
                default_arguments,
                return_blocks,
                is_void_return,
            );

            // apply the default remap when available
            let mut remapped = false;
            let (new_default, new_default_args) = if let Some(arguments) = default_remap {
                remapped = true;
                (canonical_return, arguments)
            } else {
                record_kept_return(return_blocks, kept_returns, *default);
                (*default, default_arguments.clone())
            };

            // remap switch cases into the canonical return block
            let mut new_cases = Vec::with_capacity(cases.len());
            for case in cases {
                let case_remap = remap_return_edge_arguments(
                    case.target,
                    &case.arguments,
                    return_blocks,
                    is_void_return,
                );

                // apply the case remap when available
                if let Some(arguments) = case_remap {
                    remapped = true;
                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: canonical_return,
                        arguments,
                    });
                } else {
                    record_kept_return(return_blocks, kept_returns, case.target);
                    new_cases.push(case.clone());
                }
            }

            // rebuild the switch when any edge was remapped
            if remapped {
                return mir::Terminator::Switch {
                    value: *value,
                    default: new_default,
                    default_arguments: new_default_args,
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
    tree: &mut mir::NodeTree,
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
        let mut new_block = block.clone();
        let mut new_terminator = new_block.terminator.clone();
        terminator_remap(&mut new_terminator, redirects, &value_map);

        if new_terminator != new_block.terminator {
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Fold branches and checks that target identical edges.
fn fold_redundant_edges(function: &mir::Function, tree: &mut mir::NodeTree) -> bool {
    // track whether any changes were made
    let mut changed = false;

    // simplify terminators with identical targets
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let new_terminator = match &block.terminator {
            mir::Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } if then_target == else_target && then_arguments == else_arguments => {
                Some(mir::Terminator::Jump {
                    target: *then_target,
                    arguments: then_arguments.clone(),
                })
            }
            mir::Terminator::Check {
                success, failure, ..
            } if success.target == failure.target && success.arguments == failure.arguments => {
                Some(mir::Terminator::Jump {
                    target: success.target,
                    arguments: success.arguments.clone(),
                })
            }
            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
                ..
            } => {
                // drop cases that match the default edge
                let mut filtered_cases: Vec<mir::SwitchCase> = Vec::new();
                let mut changed_cases = false;
                for case in cases {
                    if case.target == *default && case.arguments == *default_arguments {
                        changed_cases = true;
                        continue;
                    }
                    filtered_cases.push(case.clone());
                }

                // replace with a jump when all edges are identical
                if filtered_cases.is_empty() {
                    Some(mir::Terminator::Jump {
                        target: *default,
                        arguments: default_arguments.clone(),
                    })
                } else if changed_cases {
                    Some(mir::Terminator::Switch {
                        value: *value,
                        default: *default,
                        default_arguments: default_arguments.clone(),
                        cases: filtered_cases,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(new_terminator) = new_terminator {
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Fold branches that target the same block into a jump with selects.
fn fold_same_target_branches(function: &mut mir::Function, tree: &mut mir::NodeTree) -> bool {
    // track whether any changes were made
    let mut changed = false;

    // snapshot blocks to avoid borrowing conflicts with value allocation
    let block_ids = function.blocks.clone();

    for block_id in block_ids {
        // read the block
        let block = tree.get(block_id).clone();
        let mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } = &block.terminator
        else {
            continue;
        };

        // skip branches that do not target the same block
        if then_target != else_target {
            continue;
        }

        // skip malformed branches
        if then_arguments.len() != else_arguments.len() {
            continue;
        }

        // build new arguments using selects when needed
        let mut new_arguments = Vec::with_capacity(then_arguments.len());
        let mut new_block = block.clone();
        let mut inserted_select = false;

        for (then_arg, else_arg) in then_arguments.iter().zip(else_arguments.iter()) {
            // keep identical arguments unchanged
            if then_arg == else_arg {
                new_arguments.push(*then_arg);
                continue;
            }

            // materialize a select for differing arguments
            let destination = function.next_value();
            let instruction = mir::Instruction::Select {
                destination,
                condition: *condition,
                then_value: *then_arg,
                else_value: *else_arg,
            };
            let instruction_id = tree.insert(instruction);
            new_block.instructions.push(instruction_id);
            new_arguments.push(destination);
            inserted_select = true;
        }

        // skip when nothing changed
        if !inserted_select && then_arguments == else_arguments {
            continue;
        }

        // replace the branch with a jump to the shared target
        new_block.terminator = mir::Terminator::Jump {
            target: *then_target,
            arguments: new_arguments,
        };
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
    arguments: Vec<mir::Value>,
}

/// Duplicate small jump targets into jump predecessors.
fn tail_duplicate_blocks(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    profile: Option<&mir::ProfileTable>,
    domtree: &DominatorTree,
    profiled_targets: &mut HashSet<mir::LocalNodeId<mir::Block>>,
) -> bool {
    // build definition metadata
    let value_def_blocks = build_use_def_maps(function, tree).def_block;

    // collect predecessor counts and jump predecessors
    let mut predecessor_counts: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    let mut jump_predecessors: HashMap<mir::LocalNodeId<mir::Block>, Vec<JumpPredecessor>> =
        HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for successor in block.terminator.successors() {
            *predecessor_counts.entry(successor).or_insert(0) += 1;
        }

        if let mir::Terminator::Jump { target, arguments } = &block.terminator {
            jump_predecessors
                .entry(*target)
                .or_default()
                .push(JumpPredecessor {
                    pred: block_id,
                    arguments: arguments.clone(),
                });
        }
    }

    // snapshot block ids to avoid mutation during iteration
    let block_ids = function.blocks.clone();
    let mut changed = false;

    for block_id in block_ids {
        // skip blocks already handled by profile-guided duplication
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

        // require speculatable instructions
        let mut all_speculatable = true;
        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id);
            if !instruction_is_speculatable(instruction) {
                all_speculatable = false;
                break;
            }
        }
        if !all_speculatable {
            continue;
        }

        // only duplicate simple terminators
        if !matches!(
            block.terminator,
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
                &value_def_blocks,
                domtree,
            ) {
                continue;
            }
            safe_candidates.push(pred);
        }

        if safe_candidates.is_empty() {
            continue;
        }

        if safe_candidates.len() > MAX_TAIL_DUP_PREDECESSORS {
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
                value_map.insert(param.value, *arg);
            }

            // clone instructions with remapped values
            let mut new_instructions = Vec::with_capacity(block.instructions.len());
            for instruction_id in &block.instructions {
                let instruction = tree.get(*instruction_id).clone();

                if let Some(destination) = instruction.destination() {
                    let new_destination = function.next_value();
                    value_map.insert(destination, new_destination);
                }

                let cloned = instruction_map(&instruction, &value_map, tree);
                let new_id = tree.insert(cloned);
                new_instructions.push(new_id);
            }

            // clone the terminator with remapped values
            let new_terminator = terminator_substitute_uses(&block.terminator, &value_map);

            // create the duplicated block
            let mut new_block = mir::Block::new();
            new_block.instructions = new_instructions;
            new_block.terminator = new_terminator;

            // insert the duplicated block
            let new_block_id = tree.insert(new_block);
            insert_block_after(function, pred.pred, new_block_id);

            // rewrite the predecessor jump to target the duplicated block
            let pred_block = tree.get(pred.pred).clone();
            let mut updated_pred = pred_block.clone();
            updated_pred.terminator = mir::Terminator::Jump {
                target: new_block_id,
                arguments: Vec::new(),
            };
            tree.replace(pred.pred, updated_pred);
            changed = true;
        }
    }

    changed
}

/// Return true when all block uses are available at a predecessor.
fn block_uses_available_in_predecessor(
    block_id: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    tree: &mir::NodeTree,
    predecessor: mir::LocalNodeId<mir::Block>,
    value_def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> bool {
    // collect block parameter values
    let mut param_values: HashSet<mir::Value> = HashSet::new();

    for param in &block.parameters {
        param_values.insert(param.value);
    }

    // ensure all uses are defined before the predecessor
    let uses = collect_block_uses(block, tree);
    for value in uses {
        // skip values provided by block parameters
        if param_values.contains(&value) {
            continue;
        }

        // require a known definition
        let Some(def_block) = value_def_blocks.get(&value) else {
            return false;
        };

        // skip values defined inside the block
        if *def_block == block_id {
            continue;
        }

        // require dominance at the predecessor
        if !domtree.dominates(*def_block, predecessor) {
            return false;
        }
    }

    true
}

/// Return true when all values are available in the given block.
fn values_available_in_block(
    block_id: mir::LocalNodeId<mir::Block>,
    values: &[mir::Value],
    value_def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> bool {
    // ensure each value definition dominates the block
    for value in values {
        let Some(def_block) = value_def_blocks.get(value) else {
            return false;
        };

        if !domtree.dominates(*def_block, block_id) {
            return false;
        }
    }

    true
}

/// Collect all SSA values used by a block.
fn collect_block_uses(block: &mir::Block, tree: &mir::NodeTree) -> Vec<mir::Value> {
    // prepare the use list
    let mut uses = Vec::new();

    // collect uses from instructions
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        uses.extend(instruction.uses());
        if let Some(arguments) = instruction.argument_slice() {
            uses.extend(tree.get_arguments(arguments).iter().copied());
        }
    }

    // collect uses from the terminator
    match &block.terminator {
        mir::Terminator::Jump { arguments, .. } => {
            // record jump arguments
            uses.extend(arguments.iter().copied());
        }
        mir::Terminator::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            // record branch condition and arguments
            uses.push(*condition);
            uses.extend(then_arguments.iter().copied());
            uses.extend(else_arguments.iter().copied());
        }
        mir::Terminator::Check {
            condition,
            success,
            failure,
            ..
        } => {
            // record check condition and arguments
            uses.push(*condition);
            uses.extend(success.arguments.iter().copied());
            uses.extend(failure.arguments.iter().copied());
        }
        mir::Terminator::Switch {
            value,
            default_arguments,
            cases,
            ..
        } => {
            // record switch condition and arguments
            uses.push(*value);
            uses.extend(default_arguments.iter().copied());
            for case in cases {
                uses.extend(case.arguments.iter().copied());
            }
        }
        mir::Terminator::Yield {
            value,
            resume_arguments,
            ..
        } => {
            // record yield value and resume arguments
            uses.push(*value);
            uses.extend(resume_arguments.iter().copied());
        }
        mir::Terminator::Return { value } => {
            // record return value
            if let Some(value) = value {
                uses.push(*value);
            }
        }
        mir::Terminator::Unreachable
        | mir::Terminator::TailCall { .. }
        | mir::Terminator::TailCallIndirect { .. } => {}
    }

    uses
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
        if ratio >= TAIL_DUP_HOT_EDGE_RATIO {
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
    if ratio < TAIL_DUP_MIN_EDGE_RATIO {
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
fn split_critical_edges(function: &mut mir::Function, tree: &mut mir::NodeTree) -> bool {
    // collect predecessor sets for each block
    let mut predecessors: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Block>>,
    > = HashMap::new();
    let mut successor_counts: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // record unique successors for the block
        let mut unique_successors = HashSet::new();
        for successor in block.terminator.successors() {
            unique_successors.insert(successor);
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
        let terminator = block.terminator.clone();

        // rewrite terminator edges when they are critical
        let new_terminator = match &terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                // split the then edge when critical
                let new_then_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    *then_target,
                    then_arguments,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or(*then_target);

                // split the else edge when critical
                let new_else_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    *else_target,
                    else_arguments,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or(*else_target);

                if new_then_target != *then_target || new_else_target != *else_target {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: new_then_target,
                        then_arguments: then_arguments.clone(),
                        else_target: new_else_target,
                        else_arguments: else_arguments.clone(),
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } => {
                // split the success edge when critical
                let new_success_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    success.target,
                    &success.arguments,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or(success.target);

                // split the failure edge when critical
                let new_failure_target = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    failure.target,
                    &failure.arguments,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or(failure.target);

                if new_success_target != success.target || new_failure_target != failure.target {
                    Some(mir::Terminator::Check {
                        condition: *condition,
                        constraint: constraint.clone(),
                        success: mir::CheckTarget {
                            target: new_success_target,
                            arguments: success.arguments.clone(),
                        },
                        failure: mir::CheckTarget {
                            target: new_failure_target,
                            arguments: failure.arguments.clone(),
                        },
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                default_arguments,
                cases,
            } => {
                // split the default edge when critical
                let new_default = split_critical_edge_target(
                    function,
                    tree,
                    block_id,
                    *default,
                    default_arguments,
                    &predecessors,
                    &mut split_cache,
                )
                .unwrap_or(*default);

                // split each case edge when critical
                let mut updated_cases = Vec::with_capacity(cases.len());
                let mut remapped = new_default != *default;

                for case in cases {
                    let new_target = split_critical_edge_target(
                        function,
                        tree,
                        block_id,
                        case.target,
                        &case.arguments,
                        &predecessors,
                        &mut split_cache,
                    )
                    .unwrap_or(case.target);

                    if new_target != case.target {
                        remapped = true;
                    }

                    updated_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: new_target,
                        arguments: case.arguments.clone(),
                    });
                }

                if remapped {
                    Some(mir::Terminator::Switch {
                        value: *value,
                        default: new_default,
                        default_arguments: default_arguments.clone(),
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
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
            changed = true;
        }
    }

    changed
}

/// Split a critical edge target and return the new block id when needed.
fn split_critical_edge_target(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    source: mir::LocalNodeId<mir::Block>,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
    predecessors: &HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Block>>>,
    split_cache: &mut HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    >,
) -> Option<mir::LocalNodeId<mir::Block>> {
    // require multiple predecessors to be critical
    let target_preds = predecessors.get(&target).map_or(0, HashSet::len);
    if target_preds <= 1 {
        return None;
    }

    // reuse previously split edges for the same source and target
    if let Some(existing) = split_cache.get(&(source, target)) {
        return Some(*existing);
    }

    // read the target block parameters
    let target_block = tree.get(target);
    if target_block.parameters.len() != arguments.len() {
        return None;
    }

    // build new parameters that mirror the target parameter types
    let mut new_parameters = Vec::with_capacity(target_block.parameters.len());
    let mut new_arguments = Vec::with_capacity(target_block.parameters.len());
    for param in &target_block.parameters {
        let value = function.next_value();
        new_parameters.push(mir::TypedValue::new(value, param.ty));
        new_arguments.push(value);
    }

    // build the split block
    let mut new_block = mir::Block::with_parameters(new_parameters);
    new_block.terminator = mir::Terminator::Jump {
        target,
        arguments: new_arguments,
    };

    // insert the block and record it for reuse
    let new_block_id = tree.insert(new_block);
    insert_block_after(function, source, new_block_id);
    split_cache.insert((source, target), new_block_id);

    Some(new_block_id)
}

/// Merge blocks where predecessor has single successor and successor has single predecessor.
///
/// If block A unconditionally jumps to block B, and B has no other predecessors,
/// we can merge B's instructions and terminator into A.
///
/// Returns true if any blocks were merged.
fn merge_blocks(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    entry: mir::LocalNodeId<mir::Block>,
    domtree: &DominatorTree,
) -> bool {
    let value_def_blocks = build_use_def_maps(function, tree).def_block;

    // build predecessor count for each block
    let mut predecessor_count: HashMap<mir::LocalNodeId<mir::Block>, usize> = HashMap::new();
    for &block_id in &function.blocks {
        predecessor_count.entry(block_id).or_insert(0);
        let block = tree.get(block_id);
        for successor in block.terminator.successors() {
            *predecessor_count.entry(successor).or_insert(0) += 1;
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
                let mir::Terminator::Jump { target, arguments } = &block.terminator else {
                    continue;
                };
                (*target, arguments.clone(), block.clone())
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
            let (param_to_arg, target_instructions, target_terminator) = {
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
                    .map(|(param, arg)| (param.value, *arg))
                    .collect();
                (
                    param_to_arg,
                    target_block.instructions.clone(),
                    target_block.terminator.clone(),
                )
            };

            // merge: append target's instructions to our block, take target's terminator
            let mut new_block = block_clone;

            // copy and substitute instructions from target
            for instruction_id in target_instructions {
                let instruction = tree.get(instruction_id).clone();
                let new_instruction =
                    instruction_substitute_uses_in_tree(&instruction, &param_to_arg, tree);
                let new_id = tree.insert(new_instruction);
                new_block.instructions.push(new_id);
            }

            // substitute and take target's terminator
            new_block.terminator = terminator_substitute_uses(&target_terminator, &param_to_arg);

            tree.replace(block_id, new_block);

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

/// Eliminate blocks not reachable from the entry block.
/// Returns true if any blocks were removed.
fn eliminate_unreachable_blocks(
    function: &mut mir::Function,
    tree: &mir::NodeTree,
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
        for successor in block.terminator.successors() {
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
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    jump block1
block1:
    return v0
block2:
    v1 = iconst 2i32
    return v1
}"#;
        // block0's jump threads to return, block1 and block2 become unreachable
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Chains of unreachable blocks are all eliminated.
    #[test]
    fn test_eliminate_unreachable_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
block1:
    jump block2
block2:
    v1 = iconst 2i32
    return v1
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Branch on constant true folds to unconditional jump to then target.
    #[test]
    fn test_fold_constant_true_branch() {
        // branch on true folds to jump, then threads through empty return block
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        // branch folds to jump, then threads to return
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 1i32
    v2 = iconst 2i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Branch on constant false folds to unconditional jump to else target.
    #[test]
    fn test_fold_constant_false_branch() {
        // branch on false folds to jump, then threads through empty return block
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        // branch folds to jump to block2, then threads to return v2
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst false
    v1 = iconst 1i32
    v2 = iconst 2i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Identical void return blocks are merged.
    #[test]
    fn test_merge_identical_return_blocks() {
        let input = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    branch v0, block1, block2
block1:
    return
block2:
    return
}"#;
        let expected = r#"function @test(v0: bool) -> void {
block0(v0: bool):
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Non void return blocks are routed through a canonical return block.
    #[test]
    fn test_canonicalize_non_void_return_blocks() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    branch v0, block1(v1), block2(v2)
block1(v3: i32):
    return v3
block2(v4: i32):
    return v4
}"#;
        let expected = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    v6 = select v0, v1, v2
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Return values from parent blocks are forwarded through the canonical return block.
    #[test]
    fn test_canonicalize_return_with_outer_value() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    v4 = select v0, v1, v2
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Branch on a global const folds to the selected target.
    #[test]
    fn test_fold_global_const_branch() {
        let input = r#"global @flag: bool = true ; const
function @test() -> i32 {
block0:
    v0 = global.const @flag
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        let expected = r#"global @flag: bool = true ; const
function @test() -> i32 {
block0:
    v0 = global.const @flag
    v1 = iconst 1i32
    v2 = iconst 2i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Branch on non-constant condition is preserved.
    #[test]
    fn test_preserve_non_constant_branch() {
        // v0 is a parameter, not a constant
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    v3 = iadd v1, v2
    return v3
block2:
    v4 = isub v2, v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Branch on a block parameter constant folds to the selected target.
    #[test]
    fn test_fold_block_param_constant_branch() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst true
    branch v0, block1(v1), block2(v1)
block1(v2: bool):
    jump block3(v2)
block2(v3: bool):
    jump block3(v3)
block3(v4: bool):
    branch v4, block4, block5
block4:
    v5 = iconst 1i32
    return v5
block5:
    v6 = iconst 2i32
    return v6
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst true
    v5 = iconst 1i32
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Constant branch folding makes the else target unreachable.
    #[test]
    fn test_fold_and_eliminate_combined() {
        // branch on true folds to jump to block1, then threads to return
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    branch v0, block1, block2
block1:
    return v1
block2:
    v2 = iconst 0i32
    return v2
}"#;
        // branch folds, jump threads through empty block1, block2 eliminated
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Loop back-edges keep loop blocks reachable.
    #[test]
    fn test_preserve_loop_structure() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    jump block1(v0)
block1(v1: i32):
    v2 = iconst 10i32
    v3 = icmp_slt v1, v2
    branch v3, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v1, v4
    jump block1(v5)
block3:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Single-block functions with no branches are unchanged.
    #[test]
    fn test_preserve_single_block() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Nested constant branches all fold, making intermediate blocks unreachable.
    #[test]
    fn test_fold_nested_constant_branches() {
        // v0=true -> block1, v1=false -> block4, block2 and block3 become unreachable
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst false
    branch v0, block1, block2
block1:
    branch v1, block3, block4
block2:
    v2 = iconst 2i32
    return v2
block3:
    v3 = iconst 3i32
    return v3
block4:
    v4 = iconst 4i32
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst false
    v4 = iconst 4i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Diamond CFG with non-constant condition is preserved.
    #[test]
    fn test_preserve_diamond_cfg() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    jump block3(v1)
block2:
    jump block3(v2)
block3(v3: i32):
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_unchanged(input);
    }

    /// Multiple disconnected unreachable regions are all eliminated.
    #[test]
    fn test_eliminate_multiple_unreachable_regions() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
block1:
    v1 = iconst 2i32
    jump block2
block2:
    return v1
block3:
    v2 = iconst 3i32
    jump block4
block4:
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Constant branch with block arguments preserves arguments on folded jump.
    #[test]
    fn test_fold_branch_with_block_arguments() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    v2 = iconst 0i32
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    return v3
}"#;
        // after folding branch to jump, block merging merges block1 into block0
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst true
    v1 = iconst 42i32
    v2 = iconst 0i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Assume conditions fold branches to the assumed target.
    #[test]
    fn test_fold_assume_branch() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    assume v0
    branch v0, block1, block2
block1:
    return v1
block2:
    return v2
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    assume v0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Assume conditions fold checks to the success edge.
    #[test]
    fn test_fold_assume_check() {
        let input = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> u32 {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    v4 = icmp_ult v1, v2
    assume v4
    check v4, bounds.unsigned v1, v2, v3, block1, block2
block1:
    return v1
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> u32 {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    v4 = icmp_ult v1, v2
    assume v4
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Check constraints fold even when condition ranges are unknown.
    #[test]
    fn test_fold_check_constraint_truth() {
        let input = r#"function @test(v0: bool, v1: [u32; 4]) -> u32 {
block0(v0: bool, v1: [u32; 4]):
    v2 = iconst 0u32
    v3 = iconst 4u32
    check v0, bounds.unsigned v2, v3, v1, block1, block2
block1:
    return v2
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool, v1: [u32; 4]) -> u32 {
block0(v0: bool, v1: [u32; 4]):
    v2 = iconst 0u32
    v3 = iconst 4u32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Jump through empty block is threaded to final target.
    #[test]
    fn test_thread_simple_jump() {
        // block1 and block2 are both empty threadable blocks
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    jump block1
block1:
    jump block2
block2:
    return v0
}"#;
        // block0's jump threads all the way to return, both intermediates become unreachable
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Chain of empty jump blocks all thread to final target.
    #[test]
    fn test_thread_jump_chain() {
        // all intermediate blocks are empty and threadable
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    return v0
}"#;
        // block0's jump threads through entire chain to return
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Block with instructions is not threaded through, but its successor can be.
    #[test]
    fn test_preserve_block_with_instructions() {
        // block1 has instructions so can't be threaded through
        // but after threading block1's jump to return, block0 and block1 merge
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    jump block1
block1:
    v1 = iadd v0, v0
    jump block2
block2:
    return v1
}"#;
        // block1's jump threads to return, then block0 and block1 merge
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 1i32
    v1 = iadd v0, v0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Branch targets through empty blocks are threaded.
    #[test]
    fn test_thread_branch_targets() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block4
block3:
    return v1
block4:
    return v2
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    v4 = select v0, v1, v2
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Check targets thread through empty jump blocks.
    #[test]
    fn test_thread_check_targets() {
        let input = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> void {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    check v0, bounds.unsigned v1, v2, v3, block1, block2
block1:
    jump block3
block2:
    jump block4
block3:
    return
block4:
    return
}"#;
        let expected = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> void {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Switch edges thread through empty jump blocks.
    #[test]
    fn test_thread_switch_edges() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    switch v0, block1, 0 => block2
block1:
    jump block3
block2:
    jump block4
block3:
    v1 = iconst 1i32
    return v1
block4:
    v2 = iconst 2i32
    return v2
}"#;
        let expected = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    switch v0, block1, 0 => block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Edge specific ranges thread through a condition only block.
    #[test]
    fn test_thread_edge_condition_with_ranges() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 20u32
    v3 = select v0, v1, v2
    v4 = iconst 10u32
    v5 = iconst 15u32
    v6 = icmp_ult v3, v4
    branch v6, block1, block2
block1:
    v7 = icmp_ult v3, v5
    branch v7, block3, block4
block2:
    v8 = iconst 1i32
    return v8
block3:
    v9 = iconst 2i32
    return v9
block4:
    v10 = iconst 3i32
    return v10
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 20u32
    v3 = select v0, v1, v2
    v4 = iconst 10u32
    v5 = iconst 15u32
    v6 = icmp_ult v3, v4
    branch v6, block2, block1
block1:
    v8 = iconst 1i32
    return v8
block2:
    v9 = iconst 2i32
    return v9
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Block with parameters threads through its empty successor to a return.
    #[test]
    fn test_thread_parameterized_block_successor() {
        // block1 has params so can't be threaded through, but block2 is empty and threadable
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    jump block2
block2:
    return v3
}"#;
        // block1's jump is threaded directly to the return, block2 becomes unreachable
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    v5 = select v0, v1, v2
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Jump predecessors duplicate a small tail block into the jump edge.
    #[test]
    fn test_tail_duplicate_jump_predecessor() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 1i32
    branch v0, block1, block2(v1)
block1:
    jump block2(v1)
block2(v3: i32):
    v4 = iadd v3, v2
    return v4
}"#;
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 1i32
    branch v0, block1, block2(v1)
block1:
    v5 = iadd v1, v2
    return v5
block2(v3: i32):
    v4 = iadd v3, v2
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Profile-guided tail duplication duplicates only the hot edge.
    #[test]
    fn test_tail_duplicate_profile_hot_edge() {
        // base cfg with two jump predecessors into a shared tail block
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block2(v2)
block1(v3: i32):
    jump block3(v3)
block2(v4: i32):
    jump block3(v4)
block3(v5: i32):
    v6 = imul v5, v5
    return v6
}"#;
        // expected cfg after duplicating the hot predecessor only
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 1i32
    v2 = iconst 2i32
    branch v0, block1(v1), block3(v2)
block1(v3: i32):
    jump block2
block2:
    v7 = imul v3, v3
    return v7
block3(v4: i32):
    jump block4(v4)
block4(v5: i32):
    v6 = imul v5, v5
    return v6
}"#;

        // parse input program
        let mut program = TestProgram::new(input);

        // gather the hot and cold jump predecessors
        let function_id = program.first_function_id();
        let mut function = program.tree.get(function_id).clone();
        let (hot_pred, cold_pred) = program.entry_branch_targets(&function);
        let tail_block = program.jump_target(hot_pred);

        // build dominance data for tail duplication
        let analyses = program.function_analyses(&function);
        let domtree = analyses.get::<DominatorTree>().clone();

        // build the profile table for jump edges
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        program.record_jump_edge_profile(&mut profile, hot_pred, tail_block, 100);
        program.record_jump_edge_profile(&mut profile, cold_pred, tail_block, 1);

        // run tail duplication with the profile data
        function.recompute_next_value_id(&program.tree);
        let mut profiled_targets = HashSet::new();
        let changed = tail_duplicate_blocks(
            &mut function,
            &mut program.tree,
            Some(&profile),
            &domtree,
            &mut profiled_targets,
        );

        // persist changes and assert the snapshot
        assert!(changed);
        *program.tree.get_mut(function_id) = function;
        program.assert_output(expected);
    }

    /// Critical edges are split with a dedicated block.
    #[test]
    fn test_split_critical_edge() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 1i32
    branch v0, block1(v2), block2(v1)
block1(v3: i32):
    return v3
block2(v4: i32):
    v5 = iadd v4, v2
    jump block1(v5)
}"#;
        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2 = iconst 1i32
    branch v0, block1(v2), block3(v1)
block1(v6: i32):
    jump block2(v6)
block2(v3: i32):
    return v3
block3(v4: i32):
    v5 = iadd v4, v2
    jump block2(v5)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Range-based branch folding collapses branches on bounded conditions.
    #[test]
    fn test_fold_range_branch_select() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    v4 = iconst 2u32
    v5 = icmp_ult v3, v4
    v6 = iconst 10i32
    v7 = iconst 20i32
    branch v5, block1, block2
block1:
    return v6
block2:
    return v7
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    v4 = iconst 2u32
    v5 = icmp_ult v3, v4
    v6 = iconst 10i32
    v7 = iconst 20i32
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Range-based switch folding prunes impossible cases.
    #[test]
    fn test_prune_switch_cases_by_range() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    switch v3, block3, 0 => block1, 1 => block2, 2 => block4
block1:
    v4 = iconst 10i32
    return v4
block2:
    v5 = iconst 11i32
    return v5
block3:
    v6 = iconst 12i32
    return v6
block4:
    v7 = iconst 13i32
    return v7
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    switch v3, block3, 0 => block1, 1 => block2
block1:
    v4 = iconst 10i32
    return v4
block2:
    v5 = iconst 11i32
    return v5
block3:
    v6 = iconst 12i32
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Switches with identical targets fold into a jump.
    #[test]
    fn test_fold_switch_with_identical_targets() {
        let input = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    switch v0, block1, 0 => block1, 1 => block1
block1:
    v1 = iconst 10i32
    return v1
}"#;
        let expected = r#"function @test(v0: u32) -> i32 {
block0(v0: u32):
    v1 = iconst 10i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Single case switches lower to conditional branches.
    #[test]
    fn test_lower_single_case_switch_to_branch() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    switch v3, block1, 1 => block2
block1:
    v4 = iconst 10i32
    return v4
block2:
    v5 = iconst 20i32
    return v5
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = select v0, v1, v2
    v6 = iconst 1u32
    v7 = icmp_eq v3, v6
    branch v7, block2, block1
block1:
    v4 = iconst 10i32
    return v4
block2:
    v5 = iconst 20i32
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Boolean switches lower to branches without new compares.
    #[test]
    fn test_lower_boolean_switch_to_branch() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    switch v2, block1, 1 => block2
block1:
    v3 = iconst 1i32
    return v3
block2:
    v4 = iconst 2i32
    return v4
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block2, block1
block1:
    v3 = iconst 1i32
    return v3
block2:
    v4 = iconst 2i32
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Boolean switches preserve argument passing on lowering.
    #[test]
    fn test_lower_boolean_switch_with_arguments() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    v3 = iconst 7i32
    v4 = iconst 9i32
    switch v2, block1(v3), 1 => block2(v4)
block1(v5: i32):
    v6 = iadd v5, v5
    return v6
block2(v7: i32):
    v8 = iadd v7, v7
    return v8
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    v3 = iconst 7i32
    v4 = iconst 9i32
    branch v2, block2(v4), block1(v3)
block1(v5: i32):
    v6 = iadd v5, v5
    return v6
block2(v7: i32):
    v8 = iadd v7, v7
    return v8
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Boolean switches with two cases lower to a branch.
    #[test]
    fn test_lower_boolean_switch_two_cases() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    switch v2, block1, 0 => block2, 1 => block3
block1:
    v3 = iconst 10i32
    return v3
block2:
    v4 = iconst 20i32
    return v4
block3:
    v5 = iconst 30i32
    return v5
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = icmp_eq v0, v1
    branch v2, block2, block1
block1:
    v4 = iconst 20i32
    return v4
block2:
    v5 = iconst 30i32
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Single case switches preserve argument passing on lowering.
    #[test]
    fn test_lower_single_case_switch_with_arguments() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0i32
    v2 = iconst 1i32
    v3 = select v0, v1, v2
    v4 = iconst 4i32
    v5 = iconst 8i32
    switch v3, block1(v4), 1 => block2(v5)
block1(v6: i32):
    v7 = imul v6, v6
    return v7
block2(v8: i32):
    v9 = imul v8, v8
    return v9
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0i32
    v2 = iconst 1i32
    v3 = select v0, v1, v2
    v4 = iconst 4i32
    v5 = iconst 8i32
    v10 = iconst 1i32
    v11 = icmp_eq v3, v10
    branch v11, block2(v5), block1(v4)
block1(v6: i32):
    v7 = imul v6, v6
    return v7
block2(v8: i32):
    v9 = imul v8, v8
    return v9
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Switch cases that mirror the default edge are dropped.
    #[test]
    fn test_prune_default_switch_case() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block1, 0 => block1, 1 => block2
block1:
    v1 = iconst 10i32
    return v1
block2:
    v2 = iconst 20i32
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block1, 1 => block2
block1:
    v1 = iconst 10i32
    return v1
block2:
    v2 = iconst 20i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    /// Passthrough blocks forward parameters directly to their successor.
    #[test]
    fn test_thread_passthrough_block_parameters() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 5i32
    jump block1(v0)
block1(v1: i32):
    jump block2(v1)
block2(v2: i32):
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 5i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }

    fn collect_argument_mismatches(function: &mir::Function, tree: &mir::NodeTree) -> Vec<String> {
        let mut mismatches = Vec::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
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

            match &block.terminator {
                mir::Terminator::Jump { target, arguments } => {
                    check_edge(*target, arguments, &mut mismatches);
                }
                mir::Terminator::Branch {
                    then_target,
                    then_arguments,
                    else_target,
                    else_arguments,
                    ..
                } => {
                    check_edge(*then_target, then_arguments, &mut mismatches);
                    check_edge(*else_target, else_arguments, &mut mismatches);
                }
                mir::Terminator::Check {
                    success, failure, ..
                } => {
                    check_edge(success.target, &success.arguments, &mut mismatches);
                    check_edge(failure.target, &failure.arguments, &mut mismatches);
                }
                mir::Terminator::Switch {
                    default,
                    default_arguments,
                    cases,
                    ..
                } => {
                    check_edge(*default, default_arguments, &mut mismatches);
                    for case in cases {
                        check_edge(case.target, &case.arguments, &mut mismatches);
                    }
                }
                mir::Terminator::Yield {
                    resume,
                    resume_arguments,
                    ..
                } => {
                    check_edge(*resume, resume_arguments, &mut mismatches);
                }
                mir::Terminator::Return { .. }
                | mir::Terminator::Unreachable
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallIndirect { .. } => {}
            }
        }

        mismatches
    }

    fn collect_undefined_uses(function: &mir::Function, tree: &mir::NodeTree) -> Vec<String> {
        let mut defined_values: HashSet<mir::Value> = HashSet::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for param in &block.parameters {
                defined_values.insert(param.value);
            }

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    defined_values.insert(destination);
                }
            }
        }

        let mut undefined = Vec::new();

        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                let mut uses = instruction.uses();
                if let Some(arguments) = instruction.argument_slice() {
                    uses.extend(tree.get_arguments(arguments).iter().copied());
                }

                for value in uses {
                    if !defined_values.contains(&value) {
                        undefined.push(format!(
                            "block {:?} instruction {:?} uses {:?} without definition: {:?}",
                            block_id, instruction_id, value, instruction
                        ));
                    }
                }
            }

            match &block.terminator {
                mir::Terminator::Jump { arguments, .. } => {
                    for value in arguments {
                        if !defined_values.contains(value) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, value, block.terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Branch {
                    condition,
                    then_arguments,
                    else_arguments,
                    ..
                } => {
                    let mut uses =
                        Vec::with_capacity(1 + then_arguments.len() + else_arguments.len());
                    uses.push(*condition);
                    uses.extend(then_arguments.iter().copied());
                    uses.extend(else_arguments.iter().copied());
                    for value in uses {
                        if !defined_values.contains(&value) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, value, block.terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Check {
                    condition,
                    success,
                    failure,
                    ..
                } => {
                    let mut uses =
                        Vec::with_capacity(1 + success.arguments.len() + failure.arguments.len());
                    uses.push(*condition);
                    uses.extend(success.arguments.iter().copied());
                    uses.extend(failure.arguments.iter().copied());
                    for value in uses {
                        if !defined_values.contains(&value) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, value, block.terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Switch {
                    value,
                    default_arguments,
                    cases,
                    ..
                } => {
                    if !defined_values.contains(value) {
                        undefined.push(format!(
                            "block {:?} terminator uses {:?} without definition: {:?}",
                            block_id, value, block.terminator
                        ));
                    }
                    for argument in default_arguments {
                        if !defined_values.contains(argument) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, argument, block.terminator
                            ));
                        }
                    }
                    for case in cases {
                        for argument in &case.arguments {
                            if !defined_values.contains(argument) {
                                undefined.push(format!(
                                    "block {:?} terminator uses {:?} without definition: {:?}",
                                    block_id, argument, block.terminator
                                ));
                            }
                        }
                    }
                }
                mir::Terminator::Yield {
                    value,
                    resume_arguments,
                    ..
                } => {
                    if !defined_values.contains(value) {
                        undefined.push(format!(
                            "block {:?} terminator uses {:?} without definition: {:?}",
                            block_id, value, block.terminator
                        ));
                    }
                    for argument in resume_arguments {
                        if !defined_values.contains(argument) {
                            undefined.push(format!(
                                "block {:?} terminator uses {:?} without definition: {:?}",
                                block_id, argument, block.terminator
                            ));
                        }
                    }
                }
                mir::Terminator::Return { value } => {
                    if let Some(value) = value
                        && !defined_values.contains(value)
                    {
                        undefined.push(format!(
                            "block {:?} terminator uses {:?} without definition: {:?}",
                            block_id, value, block.terminator
                        ));
                    }
                }
                mir::Terminator::Unreachable
                | mir::Terminator::TailCall { .. }
                | mir::Terminator::TailCallIndirect { .. } => {}
            }
        }

        undefined
    }

    /// SimplifyCfg preserves argument counts and definitions.
    #[test]
    fn test_simplify_cfg_preserves_argument_counts() {
        let input = r#"function @test(v0: bool, v1: i32, v2: i32) -> i32 {
block0(v0: bool, v1: i32, v2: i32):
    branch v0, block1(v1), block2(v2)
block1(v3: i32):
    v4 = iadd v3, v2
    jump block3(v4)
block2(v5: i32):
    v6 = iadd v5, v1
    jump block3(v6)
block3(v7: i32):
    branch v0, block4(v7), block5(v7)
block4(v8: i32):
    return v8
block5(v9: i32):
    return v9
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);

        let function_id = program.function_id_by_name("test");
        let function = program.tree.get(function_id);
        let mismatches = collect_argument_mismatches(function, &program.tree);
        let undefined = collect_undefined_uses(function, &program.tree);
        let output = program.format();

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
