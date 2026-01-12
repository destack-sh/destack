use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ConstantPropagation, RangeAnalysis, RangeMap, ValueRange};
use crate::optimize::{
    AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext, function_thread_jumps,
    instruction_substitute_uses, substitute_values, terminator_substitute_uses,
};

declare_pass! {
    /// Simplify the control flow graph.
    ///
    /// This pass performs several CFG simplifications:
    /// 1. Branch folding: converts `branch cond, A, B` to `jump` when cond is constant or range proven
    /// 2. Path sensitive threading: threads edges using edge specific range facts
    /// 3. Jump threading: threads jumps through empty or passthrough blocks
    /// 4. Block merging: merges blocks with single predecessor/successor
    /// 5. Unreachable block elimination: removes blocks not reachable from entry
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
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get constant propagation and range analyses
        let (constants, ranges) = {
            let analyses = FunctionAnalyses::new(function, tree);
            (
                analyses.get::<ConstantPropagation>().clone(),
                analyses.get::<RangeAnalysis>().clone(),
            )
        };

        // run simplify CFG
        let changed = run_simplify_cfg(function, tree, &constants, &ranges);

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
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
) -> bool {
    let mut changed = false;

    // phase 1: branch folding
    // converts `branch always_true, A, B` -> `jump A`
    changed |= fold_branches(function, tree, constants, ranges);

    // phase 2: path sensitive jump threading
    // threads edges using edge specific range facts
    changed |= thread_edge_conditions(function, tree, constants, ranges);

    // phase 3: jump threading
    // threads jumps through empty blocks
    changed |= function_thread_jumps(function, tree);

    // phase 4: block merging
    // merges blocks with single predecessor/successor
    if let Some(entry) = function.entry {
        changed |= merge_blocks(function, tree, entry);
    }

    // phase 5: eliminate unreachable blocks
    if let Some(entry) = function.entry {
        changed |= eliminate_unreachable_blocks(function, tree, entry);
    }

    changed
}

/// Fold branches on constant conditions into unconditional jumps.
/// Returns true if any branches were folded.
fn fold_branches(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
) -> bool {
    let mut changed = false;

    // fold branches with constant or range proven conditions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let exit_ranges = ranges.exit(block_id);
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
                let condition_value =
                    condition_value.or_else(|| bool_from_range(exit_ranges.get(*condition)));

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
                let condition_value =
                    condition_value.or_else(|| bool_from_range(exit_ranges.get(*condition)));

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
                let constant_value = constants.constant_at_exit(block_id, *value);
                let range_value = exit_ranges.get(*value);
                if let Some(new_terminator) = fold_switch(
                    *value,
                    *default,
                    default_arguments,
                    cases,
                    constant_value,
                    range_value,
                ) {
                    let mut new_block = block.clone();
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
) -> bool {
    // build value definition and use maps
    let value_definitions = build_value_definitions(function, tree);
    let value_use_counts = collect_value_use_counts(function, tree);

    let mut changed = false;

    // scan blocks for edge threading opportunities
    for &block_id in &function.blocks {
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
                let then_edge = resolve_edge_target(
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
                );
                let else_edge = resolve_edge_target(
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
                let success_edge = resolve_edge_target(
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
                );
                let failure_edge = resolve_edge_target(
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

/// Build a mapping from SSA values to their defining instructions.
fn build_value_definitions(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> {
    // collect destination values for every instruction
    let mut definitions = HashMap::new();

    // scan blocks in order
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // record instruction destinations for this block
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // track each destination value
            if let Some(destination) = instruction.destination() {
                definitions.insert(destination, instruction_id);
            }
        }
    }

    definitions
}

/// Count value uses across instructions and terminators.
fn collect_value_use_counts(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, usize> {
    // initialize per value use counters
    let mut counts: HashMap<mir::Value, usize> = HashMap::new();

    // scan blocks for uses
    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        // scan instruction uses
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);

            // count inline operands
            for value in instruction.uses() {
                *counts.entry(value).or_insert(0) += 1;
            }

            // count external argument slices
            if let Some(args_slice) = instruction.argument_slice() {
                for &arg in tree.get_arguments(args_slice) {
                    *counts.entry(arg).or_insert(0) += 1;
                }
            }
        }

        // count terminator operands
        for value in block.terminator.uses() {
            *counts.entry(value).or_insert(0) += 1;
        }
    }

    counts
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
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    value_use_counts: &HashMap<mir::Value, usize>,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    // fetch the edge target block
    let block = tree.get(target);

    // require an empty or condition only block
    if !is_threadable_condition_block(block, &block.terminator, tree, value_use_counts) {
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
        tree,
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
                tree,
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
                tree,
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
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
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
        tree,
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
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
    edge_ranges: &mut RangeMap,
) {
    // look up the condition definition
    let Some(&definition_id) = value_definitions.get(&condition) else {
        return;
    };
    let instruction = tree.get(definition_id);
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
    value_definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
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
    let &definition_id = value_definitions.get(&condition)?;
    let instruction = tree.get(definition_id);
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

    evaluate_integer_comparison(*operator, *left, *right, ranges)
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

/// Evaluate integer comparisons using range information.
fn evaluate_integer_comparison(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    ranges: &RangeMap,
) -> Option<bool> {
    // extract integer ranges for both operands
    let ValueRange::Integer {
        min: left_min,
        max: left_max,
        width: left_width,
        is_signed: left_signed,
    } = ranges.get(left)?
    else {
        return None;
    };
    let ValueRange::Integer {
        min: right_min,
        max: right_max,
        width: right_width,
        is_signed: right_signed,
    } = ranges.get(right)?
    else {
        return None;
    };
    // reject mismatched integer widths or signedness
    if left_width != right_width || left_signed != right_signed {
        return None;
    }

    // reject comparisons that do not match operand signedness
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
    if expects_signed && !*left_signed {
        return None;
    }
    if expects_unsigned && *left_signed {
        return None;
    }

    // evaluate comparison from range relationships
    match operator {
        mir::BinaryOperator::Equal => {
            let is_single = left_min == left_max && right_min == right_max;
            if is_single && left_min == right_min {
                Some(true)
            } else if left_max < right_min || left_min > right_max {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::NotEqual => {
            let is_single = left_min == left_max && right_min == right_max;
            if left_max < right_min || left_min > right_max {
                Some(true)
            } else if is_single && left_min == right_min {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::UnsignedLessThan => {
            if left_max < right_min {
                Some(true)
            } else if left_min >= right_max {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedLessEqual | mir::BinaryOperator::UnsignedLessEqual => {
            if left_max <= right_min {
                Some(true)
            } else if left_min > right_max {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedGreaterThan | mir::BinaryOperator::UnsignedGreaterThan => {
            if left_min > right_max {
                Some(true)
            } else if left_max <= right_min {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedGreaterEqual | mir::BinaryOperator::UnsignedGreaterEqual => {
            if left_min >= right_max {
                Some(true)
            } else if left_max < right_min {
                Some(false)
            } else {
                None
            }
        }
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

/// Swap a comparison operator by exchanging operands.
fn swap_comparison_operator(operator: mir::BinaryOperator) -> Option<mir::BinaryOperator> {
    match operator {
        mir::BinaryOperator::Equal | mir::BinaryOperator::NotEqual => Some(operator),
        mir::BinaryOperator::SignedLessThan => Some(mir::BinaryOperator::SignedGreaterThan),
        mir::BinaryOperator::SignedLessEqual => Some(mir::BinaryOperator::SignedGreaterEqual),
        mir::BinaryOperator::SignedGreaterThan => Some(mir::BinaryOperator::SignedLessThan),
        mir::BinaryOperator::SignedGreaterEqual => Some(mir::BinaryOperator::SignedLessEqual),
        mir::BinaryOperator::UnsignedLessThan => Some(mir::BinaryOperator::UnsignedGreaterThan),
        mir::BinaryOperator::UnsignedLessEqual => Some(mir::BinaryOperator::UnsignedGreaterEqual),
        mir::BinaryOperator::UnsignedGreaterThan => Some(mir::BinaryOperator::UnsignedLessThan),
        mir::BinaryOperator::UnsignedGreaterEqual => Some(mir::BinaryOperator::UnsignedLessEqual),
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

/// Convert a boolean range into a constant when possible.
fn bool_from_range(range: Option<&ValueRange>) -> Option<bool> {
    let ValueRange::Boolean {
        can_be_true,
        can_be_false,
    } = range?
    else {
        return None;
    };

    match (*can_be_true, *can_be_false) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
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
) -> bool {
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
                let new_instruction = instruction_substitute_uses(&instruction, &param_to_arg);
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
    return v1
block2:
    return v2
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
    branch v0, block1(v1), block1(v1)
block1(v4: bool):
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
    jump block1
block1:
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
    branch v0, block1, block2
block1:
    return v1
block2:
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
    branch v0, block1(v1), block1(v2)
block1(v3: i32):
    return v3
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
    jump block1(v0)
block1(v2: i32):
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&SimplifyCfg);
        program.assert_output(expected);
    }
}
