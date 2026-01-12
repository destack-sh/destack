use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, RangeAnalysis, RangeMap, ValueRange};
use crate::optimize::common::{
    SuccessorArguments, build_value_instruction_map, terminator_arguments_for_successor_checked,
};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

declare_pass! {
    /// Eliminate redundant guard checks when conditions are proven.
    ///
    /// Uses control flow facts, assume instructions, and range analysis to
    /// remove checks that are guaranteed to take one edge.
    ///
    /// ```mir
    /// function @before(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
    /// block0(v0: u32, v1: u32, v2: [u32; 4]):
    ///     v3 = icmp_eq v0, v1
    ///     branch v3, block1, block2
    /// block1:
    ///     check v3, bounds.unsigned v0, v1, v2, block3, block4
    /// block3:
    ///     return v0
    /// block4:
    ///     unreachable
    /// block2:
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
    /// block0(v0: u32, v1: u32, v2: [u32; 4]):
    ///     v3 = icmp_eq v0, v1
    ///     branch v3, block1, block2
    /// block1:
    ///     jump block3
    /// block3:
    ///     return v0
    /// block4:
    ///     unreachable
    /// block2:
    ///     return v1
    /// }
    /// ```
    #[pass(id = "guard-eliminate")]
    pub GuardEliminate,
    "Eliminate redundant guard checks"
}

impl FunctionPass for GuardEliminate {
    /// Run guard elimination on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        let Some(entry) = function.entry else {
            return AnalysisPreservation::all();
        };

        // gather analyses
        let analyses = FunctionAnalyses::new(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let ranges = analyses.get::<RangeAnalysis>().clone();

        // build condition definition map
        let definitions = build_value_instruction_map(function, tree);

        // compute condition facts per block
        let fact_maps = build_condition_fact_maps(entry, tree, &cfg, &definitions);

        // scan blocks for eliminable checks
        let mut changed = false;
        for &block_id in &function.blocks {
            // read the terminator
            let terminator = tree.get(block_id).terminator.clone();
            let mir::Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } = terminator
            else {
                continue;
            };

            // resolve condition truth using ranges and facts
            let facts = fact_maps.exit(block_id);
            let block_ranges = ranges.exit(block_id);
            let condition_truth =
                condition_truth_value(condition, block_ranges, &facts, &definitions);

            // resolve semantic constraints using ranges
            let constraint_truth = constraint_truth_value(&constraint, block_ranges);
            let check_outcome = resolve_check_outcome(condition_truth, constraint_truth);

            // rewrite the terminator when the outcome is known
            if let Some(is_true) = check_outcome {
                let target = if is_true { &success } else { &failure };
                replace_check_with_jump(tree, block_id, target.target, &target.arguments);
                changed = true;
            }
        }

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "GuardEliminate"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "guard-eliminate"
    }
}

/// Known boolean facts at a program point.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConditionFacts {
    /// Known truth values keyed by value.
    truths: HashMap<mir::Value, bool>,
}

impl ConditionFacts {
    /// Create an empty fact set.
    fn new() -> Self {
        Self {
            truths: HashMap::new(),
        }
    }

    /// Get the known truth value for a condition.
    fn truth_for(&self, value: mir::Value) -> Option<bool> {
        self.truths.get(&value).copied()
    }

    /// Record a truth value for a condition.
    fn insert(&mut self, value: mir::Value, truth: bool) {
        // update or clear the recorded truth
        match self.truths.get(&value) {
            None => {
                self.truths.insert(value, truth);
            }
            Some(existing) if *existing == truth => {}
            Some(_) => {
                self.truths.remove(&value);
            }
        }
    }

    /// Intersect facts that agree along all paths.
    fn meet(&self, other: &Self) -> Self {
        let mut truths = HashMap::new();

        // retain facts that agree in both sets
        for (value, truth) in &self.truths {
            if let Some(other_truth) = other.truths.get(value)
                && other_truth == truth
            {
                truths.insert(*value, *truth);
            }
        }

        Self { truths }
    }
}

impl Default for ConditionFacts {
    fn default() -> Self {
        Self::new()
    }
}

/// Block-local maps of condition facts.
#[derive(Debug, Clone)]
struct ConditionFactMaps {
    /// Facts available at block exit.
    exit: HashMap<mir::LocalNodeId<mir::Block>, ConditionFacts>,
}

impl ConditionFactMaps {
    /// Get exit facts for a block.
    fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> ConditionFacts {
        self.exit.get(&block).cloned().unwrap_or_default()
    }
}

/// Snapshot of an integer range with type metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntegerRangeSnapshot {
    /// Minimum possible value.
    min: i128,
    /// Maximum possible value.
    max: i128,
    /// Bit width of the integer.
    width: u8,
    /// Whether the integer is signed.
    is_signed: bool,
}

/// Build condition facts using a forward dataflow walk.
fn build_condition_fact_maps(
    entry: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    cfg: &ControlFlowGraph,
    definitions: &HashMap<mir::Value, mir::Instruction>,
) -> ConditionFactMaps {
    // initialize worklist from entry
    let mut entry_facts = HashMap::new();
    let mut exit_facts = HashMap::new();
    entry_facts.insert(entry, ConditionFacts::new());

    let mut worklist = VecDeque::new();
    let mut in_worklist = HashSet::new();
    worklist.push_back(entry);
    in_worklist.insert(entry);

    // process blocks until fixpoint
    while let Some(block_id) = worklist.pop_front() {
        // drop the block from the pending set
        in_worklist.remove(&block_id);

        // compute entry facts from predecessors
        let entry_state = if block_id == entry {
            entry_facts.get(&block_id).cloned().unwrap_or_default()
        } else {
            compute_entry_facts(block_id, cfg, tree, &exit_facts, definitions)
        };

        // update entry facts when they change
        let entry_changed = entry_facts
            .get(&block_id)
            .map(|facts| facts != &entry_state)
            .unwrap_or(true);
        if entry_changed {
            entry_facts.insert(block_id, entry_state.clone());
        }

        // apply assume instructions to build exit facts
        let exit_state = apply_assumes(block_id, tree, entry_state, definitions);
        // update exit facts when they change
        let exit_changed = exit_facts
            .get(&block_id)
            .map(|facts| facts != &exit_state)
            .unwrap_or(true);
        if exit_changed {
            exit_facts.insert(block_id, exit_state);

            // enqueue successors after exit changes
            let block = tree.get(block_id);
            for successor in block.terminator.successors() {
                if in_worklist.insert(successor) {
                    worklist.push_back(successor);
                }
            }
        }
    }

    ConditionFactMaps { exit: exit_facts }
}

/// Merge predecessor facts and edge conditions into entry facts.
fn compute_entry_facts(
    block_id: mir::LocalNodeId<mir::Block>,
    cfg: &ControlFlowGraph,
    tree: &mir::NodeTree,
    exit_facts: &HashMap<mir::LocalNodeId<mir::Block>, ConditionFacts>,
    definitions: &HashMap<mir::Value, mir::Instruction>,
) -> ConditionFacts {
    // intersect all incoming edge facts
    let mut merged: Option<ConditionFacts> = None;

    for &pred in cfg.predecessors(block_id) {
        let base_facts = exit_facts.get(&pred).cloned().unwrap_or_default();
        let edge_facts = apply_edge_condition(base_facts, pred, block_id, tree, definitions);
        merged = match merged {
            None => Some(edge_facts),
            Some(existing) => Some(existing.meet(&edge_facts)),
        };
    }

    merged.unwrap_or_default()
}

/// Apply branch or check facts for an edge.
fn apply_edge_condition(
    mut facts: ConditionFacts,
    pred: mir::LocalNodeId<mir::Block>,
    succ: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    definitions: &HashMap<mir::Value, mir::Instruction>,
) -> ConditionFacts {
    let terminator = &tree.get(pred).terminator;

    // apply branch or check condition for the chosen successor
    match terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
            ..
        } => {
            if succ == *then_target {
                apply_condition_fact(&mut facts, *condition, true, definitions);
            } else if succ == *else_target {
                apply_condition_fact(&mut facts, *condition, false, definitions);
            }
        }
        mir::Terminator::Check {
            condition,
            success,
            failure,
            ..
        } => {
            if succ == success.target {
                apply_condition_fact(&mut facts, *condition, true, definitions);
            } else if succ == failure.target {
                apply_condition_fact(&mut facts, *condition, false, definitions);
            }
        }
        _ => {}
    }

    // propagate boolean facts into successor parameters
    let successor_args = terminator_arguments_for_successor_checked(terminator, succ);
    if let SuccessorArguments::Consistent(args) = successor_args {
        let successor_block = tree.get(succ);
        if args.len() == successor_block.parameters.len() {
            for (param, arg) in successor_block.parameters.iter().zip(args.iter()) {
                if let Some(truth) = facts.truth_for(*arg) {
                    facts.insert(param.value, truth);
                }
            }
        }
    }

    facts
}

/// Apply assume instructions within a block to condition facts.
fn apply_assumes(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    mut facts: ConditionFacts,
    definitions: &HashMap<mir::Value, mir::Instruction>,
) -> ConditionFacts {
    let block = tree.get(block_id);

    // apply each assume condition in order
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        let mir::Instruction::Assume { condition } = instruction else {
            continue;
        };

        apply_condition_fact(&mut facts, *condition, true, definitions);
    }

    facts
}

/// Record condition facts, propagating through simple boolean forms.
fn apply_condition_fact(
    facts: &mut ConditionFacts,
    condition: mir::Value,
    truth: bool,
    definitions: &HashMap<mir::Value, mir::Instruction>,
) {
    // track visited conditions to avoid cycles
    let mut visited = HashSet::new();

    apply_condition_fact_inner(facts, condition, truth, definitions, &mut visited);
}

/// Recursive implementation of condition fact propagation.
fn apply_condition_fact_inner(
    facts: &mut ConditionFacts,
    condition: mir::Value,
    truth: bool,
    definitions: &HashMap<mir::Value, mir::Instruction>,
    visited: &mut HashSet<mir::Value>,
) {
    // stop on cycles
    if !visited.insert(condition) {
        return;
    }

    // record the direct condition truth
    facts.insert(condition, truth);

    let Some(instruction) = definitions.get(&condition) else {
        return;
    };

    match instruction {
        mir::Instruction::Unary {
            operator: mir::UnaryOperator::Not,
            argument,
            ..
        } => {
            apply_condition_fact_inner(facts, *argument, !truth, definitions, visited);
        }
        mir::Instruction::Binary {
            operator: mir::BinaryOperator::And,
            left,
            right,
            ..
        } if truth => {
            apply_condition_fact_inner(facts, *left, true, definitions, visited);
            apply_condition_fact_inner(facts, *right, true, definitions, visited);
        }
        mir::Instruction::Binary {
            operator: mir::BinaryOperator::Or,
            left,
            right,
            ..
        } if !truth => {
            apply_condition_fact_inner(facts, *left, false, definitions, visited);
            apply_condition_fact_inner(facts, *right, false, definitions, visited);
        }
        _ => {}
    }
}

/// Evaluate a condition to a constant truth value when possible.
fn condition_truth_value(
    condition: mir::Value,
    ranges: &RangeMap,
    facts: &ConditionFacts,
    definitions: &HashMap<mir::Value, mir::Instruction>,
) -> Option<bool> {
    // track visited conditions to avoid cycles
    let mut visited = HashSet::new();

    condition_truth_value_inner(condition, ranges, facts, definitions, &mut visited)
}

/// Resolve the check outcome from condition and constraint truths.
fn resolve_check_outcome(
    condition_truth: Option<bool>,
    constraint_truth: Option<bool>,
) -> Option<bool> {
    // prefer explicit condition knowledge when it agrees
    if let Some(condition_truth) = condition_truth {
        if let Some(constraint_truth) = constraint_truth
            && constraint_truth != condition_truth
        {
            return None;
        }

        return Some(condition_truth);
    }

    // fall back to constraint truth when the condition is unknown
    constraint_truth
}

/// Recursive evaluation of condition truth values.
fn condition_truth_value_inner(
    condition: mir::Value,
    ranges: &RangeMap,
    facts: &ConditionFacts,
    definitions: &HashMap<mir::Value, mir::Instruction>,
    visited: &mut HashSet<mir::Value>,
) -> Option<bool> {
    // stop on cycles
    if !visited.insert(condition) {
        return None;
    }

    // resolve the condition using facts, ranges, and instruction structure
    let result = if let Some(truth) = facts.truth_for(condition) {
        Some(truth)
    } else if let Some(range) = ranges.get(condition)
        && let Some(mir::Constant::Boolean { value }) = range.as_constant()
    {
        Some(value)
    } else if let Some(instruction) = definitions.get(&condition) {
        match instruction {
            mir::Instruction::Const { value, .. } => match value {
                mir::Constant::Boolean { value } => Some(*value),
                _ => None,
            },
            mir::Instruction::Unary {
                operator: mir::UnaryOperator::Not,
                argument,
                ..
            } => condition_truth_value_inner(*argument, ranges, facts, definitions, visited)
                .map(|value| !value),
            mir::Instruction::Binary {
                operator,
                left,
                right,
                ..
            } => {
                let left_value =
                    condition_truth_value_inner(*left, ranges, facts, definitions, visited);
                let right_value =
                    condition_truth_value_inner(*right, ranges, facts, definitions, visited);
                match operator {
                    mir::BinaryOperator::And => match (left_value, right_value) {
                        (Some(false), _) | (_, Some(false)) => Some(false),
                        (Some(true), Some(true)) => Some(true),
                        _ => None,
                    },
                    mir::BinaryOperator::Or => match (left_value, right_value) {
                        (Some(true), _) | (_, Some(true)) => Some(true),
                        (Some(false), Some(false)) => Some(false),
                        _ => None,
                    },
                    mir::BinaryOperator::Xor => match (left_value, right_value) {
                        (Some(left), Some(right)) => Some(left ^ right),
                        _ => None,
                    },
                    mir::BinaryOperator::Equal => match (left_value, right_value) {
                        (Some(left), Some(right)) => Some(left == right),
                        _ => None,
                    },
                    mir::BinaryOperator::NotEqual => match (left_value, right_value) {
                        (Some(left), Some(right)) => Some(left != right),
                        _ => None,
                    },
                    _ => None,
                }
            }
            mir::Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => {
                let selector =
                    condition_truth_value_inner(*condition, ranges, facts, definitions, visited)?;
                if selector {
                    condition_truth_value_inner(*then_value, ranges, facts, definitions, visited)
                } else {
                    condition_truth_value_inner(*else_value, ranges, facts, definitions, visited)
                }
            }
            _ => None,
        }
    } else {
        None
    };

    // pop the condition from the recursion stack
    visited.remove(&condition);

    result
}

/// Evaluate a check constraint to a constant truth value when possible.
fn constraint_truth_value(constraint: &mir::CheckConstraint, ranges: &RangeMap) -> Option<bool> {
    match constraint {
        mir::CheckConstraint::Bounds {
            index,
            length,
            is_signed,
            ..
        } => bounds_constraint_truth(*index, *length, *is_signed, ranges),
        mir::CheckConstraint::Null { .. } => None,
        mir::CheckConstraint::DivZero { divisor } => div_zero_constraint_truth(*divisor, ranges),
        mir::CheckConstraint::ShiftRange {
            value,
            bit_width,
            is_signed,
        } => shift_constraint_truth(*value, *bit_width, *is_signed, ranges),
        mir::CheckConstraint::Narrow {
            value,
            to_width,
            is_signed,
        } => narrow_constraint_truth(*value, *to_width, *is_signed, ranges),
        mir::CheckConstraint::Overflow {
            operator,
            left,
            right,
            is_signed,
        } => overflow_constraint_truth(*operator, *left, *right, *is_signed, ranges),
    }
}

/// Evaluate a bounds check constraint using range information.
fn bounds_constraint_truth(
    index: mir::Value,
    length: mir::Value,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // read index and length ranges
    let index_range = integer_range_snapshot(index, ranges)?;
    let length_range = integer_range_snapshot(length, ranges)?;

    // require signedness alignment
    if index_range.is_signed != is_signed || length_range.is_signed != is_signed {
        return None;
    }

    // check for always failing ranges
    if length_range.max <= 0 {
        return Some(false);
    }
    if is_signed && index_range.max < 0 {
        return Some(false);
    }
    if index_range.min >= length_range.max {
        return Some(false);
    }

    // check for always successful ranges
    if index_range.min >= 0 && length_range.min >= 0 && index_range.max < length_range.min {
        return Some(true);
    }

    None
}

/// Evaluate a division by zero constraint using range information.
fn div_zero_constraint_truth(divisor: mir::Value, ranges: &RangeMap) -> Option<bool> {
    // read the divisor range
    let divisor_range = integer_range_snapshot(divisor, ranges)?;

    // return when the divisor is always zero or always non zero
    if divisor_range.min == 0 && divisor_range.max == 0 {
        return Some(false);
    }

    if divisor_range.max < 0 || divisor_range.min > 0 {
        return Some(true);
    }

    None
}

/// Evaluate a shift range constraint using range information.
fn shift_constraint_truth(
    value: mir::Value,
    bit_width: u8,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // reject invalid widths
    let max_shift = i128::from(bit_width).checked_sub(1)?;

    // read the shift amount range
    let shift_range = integer_range_snapshot(value, ranges)?;
    if shift_range.is_signed != is_signed {
        return None;
    }

    // check for always failing ranges
    if shift_range.max < 0 || shift_range.min > max_shift {
        return Some(false);
    }

    // check for always successful ranges
    if shift_range.min >= 0 && shift_range.max <= max_shift {
        return Some(true);
    }

    None
}

/// Evaluate a narrowing constraint using range information.
fn narrow_constraint_truth(
    value: mir::Value,
    to_width: u8,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // compute target bounds
    let (target_min, target_max) = integer_bounds(to_width, is_signed)?;

    // read the value range
    let value_range = integer_range_snapshot(value, ranges)?;
    if value_range.is_signed != is_signed {
        return None;
    }

    // check for always failing ranges
    if value_range.max < target_min || value_range.min > target_max {
        return Some(false);
    }

    // check for always successful ranges
    if value_range.min >= target_min && value_range.max <= target_max {
        return Some(true);
    }

    None
}

/// Evaluate an overflow constraint using range information.
fn overflow_constraint_truth(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    is_signed: bool,
    ranges: &RangeMap,
) -> Option<bool> {
    // read operand ranges
    let left_range = integer_range_snapshot(left, ranges)?;
    let right_range = integer_range_snapshot(right, ranges)?;

    // require matching signedness and width
    if left_range.is_signed != is_signed
        || right_range.is_signed != is_signed
        || left_range.width != right_range.width
    {
        return None;
    }

    // compute the target bounds
    let (min_bound, max_bound) = integer_bounds(left_range.width, is_signed)?;

    // compute the result bounds
    let (result_min, result_max) = match operator {
        mir::BinaryOperator::Add => (
            left_range.min.checked_add(right_range.min)?,
            left_range.max.checked_add(right_range.max)?,
        ),
        mir::BinaryOperator::Subtract => (
            left_range.min.checked_sub(right_range.max)?,
            left_range.max.checked_sub(right_range.min)?,
        ),
        mir::BinaryOperator::Multiply => {
            let candidates = [
                left_range.min.checked_mul(right_range.min)?,
                left_range.min.checked_mul(right_range.max)?,
                left_range.max.checked_mul(right_range.min)?,
                left_range.max.checked_mul(right_range.max)?,
            ];
            let min = *candidates.iter().min()?;
            let max = *candidates.iter().max()?;
            (min, max)
        }
        mir::BinaryOperator::SignedDivide | mir::BinaryOperator::SignedRemainder => {
            // require signed overflow semantics
            if !is_signed {
                return None;
            }

            // detect the min divided by negative one overflow case
            let min_value = min_bound;
            let neg_one = -1;
            let left_has_min = range_includes_value(&left_range, min_value);
            let right_has_neg_one = range_includes_value(&right_range, neg_one);

            // reject when overflow remains possible
            if left_has_min && right_has_neg_one {
                if left_range.min == min_value
                    && left_range.max == min_value
                    && right_range.min == neg_one
                    && right_range.max == neg_one
                {
                    return Some(false);
                }

                return None;
            }

            // remaining cases cannot overflow
            return Some(true);
        }
        _ => return None,
    };

    // check for always failing ranges
    if result_max < min_bound || result_min > max_bound {
        return Some(false);
    }

    // check for always successful ranges
    if result_min >= min_bound && result_max <= max_bound {
        return Some(true);
    }

    None
}

/// Extract an integer range snapshot for a value.
fn integer_range_snapshot(value: mir::Value, ranges: &RangeMap) -> Option<IntegerRangeSnapshot> {
    // fetch the range and require integer bounds
    let range = ranges.get(value)?;
    let ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    } = range
    else {
        return None;
    };

    Some(IntegerRangeSnapshot {
        min: *min,
        max: *max,
        width: *width,
        is_signed: *is_signed,
    })
}

/// Compute the full bounds for an integer width and signedness.
fn integer_bounds(width: u8, is_signed: bool) -> Option<(i128, i128)> {
    // reject nonsensical widths
    if width == 0 {
        return None;
    }

    // handle full width signed bounds
    if is_signed && width == 128 {
        return Some((i128::MIN, i128::MAX));
    }

    // compute signed or unsigned bounds
    if is_signed {
        let shift = u32::from(width).checked_sub(1)?;
        let magnitude = (1_i128).checked_shl(shift)?;
        let min = -magnitude;
        let max = magnitude.checked_sub(1)?;
        Some((min, max))
    } else {
        let magnitude = (1_i128).checked_shl(u32::from(width))?;
        let max = magnitude.checked_sub(1)?;
        Some((0, max))
    }
}

/// Return true when a range includes a specific value.
fn range_includes_value(range: &IntegerRangeSnapshot, value: i128) -> bool {
    range.min <= value && value <= range.max
}

/// Replace a check terminator with a direct jump.
fn replace_check_with_jump(
    tree: &mut mir::NodeTree,
    block_id: mir::LocalNodeId<mir::Block>,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
) {
    // build a jump terminator replacement
    let block = tree.get(block_id);
    let mut new_block = block.clone();
    new_block.terminator = mir::Terminator::Jump {
        target,
        arguments: arguments.to_vec(),
    };
    tree.replace(block_id, new_block);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Dominating branch conditions eliminate redundant checks.
    #[test]
    fn test_guard_eliminate_branch_facts() {
        let input = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    branch v3, block1, block2
block1:
    check v3, bounds.unsigned v0, v1, v2, block3, block4
block2:
    return v1
block3:
    return v0
block4:
    unreachable
}"#;
        let expected = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    branch v3, block1, block2
block1:
    jump block3
block2:
    return v1
block3:
    return v0
block4:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Assume instructions feed redundant checks.
    #[test]
    fn test_guard_eliminate_assume_fact() {
        let input = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    assume v3
    check v3, bounds.unsigned v0, v1, v2, block1, block2
block1:
    return v0
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    assume v3
    jump block1
block1:
    return v0
block2:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Constant conditions eliminate checks.
    #[test]
    fn test_guard_eliminate_constant_condition() {
        let input = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = iconst false
    check v3, bounds.unsigned v0, v1, v2, block1, block2
block1:
    return v0
block2:
    return v1
}"#;
        let expected = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = iconst false
    jump block2
block1:
    return v0
block2:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Negated conditions are resolved using edge facts.
    #[test]
    fn test_guard_eliminate_negated_condition() {
        let input = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    v4 = bnot v3
    branch v3, block1, block2
block1:
    return v0
block2:
    check v4, bounds.unsigned v0, v1, v2, block3, block4
block3:
    return v1
block4:
    unreachable
}"#;
        let expected = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    v4 = bnot v3
    branch v3, block1, block2
block1:
    return v0
block2:
    jump block3
block3:
    return v1
block4:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Condition facts transfer through block parameters.
    #[test]
    fn test_guard_eliminate_block_param_condition() {
        let input = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    branch v3, block1(v3), block2(v3)
block1(v4: bool):
    check v4, bounds.unsigned v0, v1, v2, block3, block4
block2(v5: bool):
    return v1
block3:
    return v0
block4:
    unreachable
}"#;
        let expected = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    branch v3, block1(v3), block2(v3)
block1(v4: bool):
    jump block3
block2(v5: bool):
    return v1
block3:
    return v0
block4:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Check edges propagate condition facts to successors.
    #[test]
    fn test_guard_eliminate_check_edge_fact() {
        let input = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> u32 {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    check v0, bounds.unsigned v1, v2, v3, block1, block2
block1:
    check v0, bounds.unsigned v1, v2, v3, block3, block4
block2:
    return v2
block3:
    return v1
block4:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool, v1: u32, v2: u32, v3: [u32; 4]) -> u32 {
block0(v0: bool, v1: u32, v2: u32, v3: [u32; 4]):
    check v0, bounds.unsigned v1, v2, v3, block1, block2
block1:
    jump block3
block2:
    return v2
block3:
    return v1
block4:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Bounds constraints eliminate checks when always in range.
    #[test]
    fn test_guard_eliminate_bounds_constraint_success() {
        let input = r#"function @test(v0: bool, v1: [u32; 4]) -> u32 {
block0(v0: bool, v1: [u32; 4]):
    v2 = iconst 2u32
    v3 = iconst 4u32
    check v0, bounds.unsigned v2, v3, v1, block1, block2
block1:
    return v2
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool, v1: [u32; 4]) -> u32 {
block0(v0: bool, v1: [u32; 4]):
    v2 = iconst 2u32
    v3 = iconst 4u32
    jump block1
block1:
    return v2
block2:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Bounds constraints jump to failure when always out of range.
    #[test]
    fn test_guard_eliminate_bounds_constraint_failure() {
        let input = r#"function @test(v0: bool, v1: [u32; 0]) -> u32 {
block0(v0: bool, v1: [u32; 0]):
    v2 = iconst 0u32
    v3 = iconst 0u32
    check v0, bounds.unsigned v2, v3, v1, block1, block2
block1:
    unreachable
block2:
    return v2
}"#;
        let expected = r#"function @test(v0: bool, v1: [u32; 0]) -> u32 {
block0(v0: bool, v1: [u32; 0]):
    v2 = iconst 0u32
    v3 = iconst 0u32
    jump block2
block1:
    unreachable
block2:
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Div zero constraints eliminate checks with non zero divisors.
    #[test]
    fn test_guard_eliminate_div_zero_constraint_success() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 4i32
    check v0, div_zero v1, block1, block2
block1:
    return v1
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 4i32
    jump block1
block1:
    return v1
block2:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Div zero constraints eliminate checks with zero divisors.
    #[test]
    fn test_guard_eliminate_div_zero_constraint_failure() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0i32
    check v0, div_zero v1, block1, block2
block1:
    unreachable
block2:
    return v1
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0i32
    jump block2
block1:
    unreachable
block2:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Shift range constraints eliminate checks with in range shifts.
    #[test]
    fn test_guard_eliminate_shift_constraint_success() {
        let input = r#"function @test(v0: bool) -> u8 {
block0(v0: bool):
    v1 = iconst 3u8
    check v0, shift.unsigned v1, 8, block1, block2
block1:
    return v1
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool) -> u8 {
block0(v0: bool):
    v1 = iconst 3u8
    jump block1
block1:
    return v1
block2:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Shift range constraints jump to failure on out of range shifts.
    #[test]
    fn test_guard_eliminate_shift_constraint_failure() {
        let input = r#"function @test(v0: bool) -> u8 {
block0(v0: bool):
    v1 = iconst 8u8
    check v0, shift.unsigned v1, 8, block1, block2
block1:
    unreachable
block2:
    return v1
}"#;
        let expected = r#"function @test(v0: bool) -> u8 {
block0(v0: bool):
    v1 = iconst 8u8
    jump block2
block1:
    unreachable
block2:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Narrow constraints eliminate checks for values in range.
    #[test]
    fn test_guard_eliminate_narrow_constraint_success() {
        let input = r#"function @test(v0: bool) -> u16 {
block0(v0: bool):
    v1 = iconst 12u16
    check v0, narrow.unsigned v1, 8, block1, block2
block1:
    return v1
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool) -> u16 {
block0(v0: bool):
    v1 = iconst 12u16
    jump block1
block1:
    return v1
block2:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Narrow constraints jump to failure for out of range values.
    #[test]
    fn test_guard_eliminate_narrow_constraint_failure() {
        let input = r#"function @test(v0: bool) -> u16 {
block0(v0: bool):
    v1 = iconst 300u16
    check v0, narrow.unsigned v1, 8, block1, block2
block1:
    unreachable
block2:
    return v1
}"#;
        let expected = r#"function @test(v0: bool) -> u16 {
block0(v0: bool):
    v1 = iconst 300u16
    jump block2
block1:
    unreachable
block2:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Overflow constraints eliminate checks when no overflow is possible.
    #[test]
    fn test_guard_eliminate_overflow_constraint_success() {
        let input = r#"function @test(v0: bool) -> i8 {
block0(v0: bool):
    v1 = iconst 1i8
    v2 = iconst 2i8
    check v0, overflow.signed.iadd v1, v2, block1, block2
block1:
    return v1
block2:
    unreachable
}"#;
        let expected = r#"function @test(v0: bool) -> i8 {
block0(v0: bool):
    v1 = iconst 1i8
    v2 = iconst 2i8
    jump block1
block1:
    return v1
block2:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }

    /// Overflow constraints jump to failure when overflow is guaranteed.
    #[test]
    fn test_guard_eliminate_overflow_constraint_failure() {
        let input = r#"function @test(v0: bool) -> i8 {
block0(v0: bool):
    v1 = iconst 120i8
    v2 = iconst 120i8
    check v0, overflow.signed.iadd v1, v2, block1, block2
block1:
    unreachable
block2:
    return v1
}"#;
        let expected = r#"function @test(v0: bool) -> i8 {
block0(v0: bool):
    v1 = iconst 120i8
    v2 = iconst 120i8
    jump block2
block1:
    unreachable
block2:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GuardEliminate);
        program.assert_output(expected);
    }
}
