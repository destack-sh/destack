use std::collections::HashMap;

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{
    ConstantPropagation, ControlFlowGraph, DominatorTree, ValueRange,
};
use crate::common::mir::{
    apply_substitutions_in_dominated_blocks, build_use_def_maps, build_value_instruction_map,
    swap_comparison_operator,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Propagate equalities implied by dominating conditions.
    ///
    /// When a branch condition proves two values are equal, this pass replaces
    /// uses of one value with the other within the dominated region.
    ///
    /// ```mir
    /// function before(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = int.eq v0, v1
    ///     branch v2, b1, b2
    /// b1:
    ///     v3 = int.add v0, v1
    ///     jump b3(v3)
    /// b2:
    ///     v4 = int.sub v0, v1
    ///     jump b3(v4)
    /// b3(v5: int32):
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32, v1: int32): int32 {
    /// b0(v0: int32, v1: int32):
    ///     v2 = int.eq v0, v1
    ///     branch v2, b1, b2
    /// b1:
    ///     v3 = int.add v0, v0
    ///     jump b3(v3)
    /// b2:
    ///     v4 = int.sub v0, v1
    ///     jump b3(v4)
    /// b3(v5: int32):
    ///     return v5
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only propagates integer and pointer equality
    /// - Does not propagate float equality due to NaN and signed zero
    /// - Folds dominated integer comparisons using branch range constraints
    #[pass(id = "correlated-value-prop")]
    pub CorrelatedValueProp,
    "Propagate correlated values from dominating conditions"
}

impl FunctionPass for CorrelatedValueProp {
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

        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let domtree = analyses.get::<DominatorTree>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let constants = analyses.get::<ConstantPropagation>().clone();

        // run correlated propagation
        let changed = run_correlated_value_prop(function, tree, &domtree, &cfg, &constants);

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "CorrelatedValueProp"
    }

    fn id(&self) -> &'static str {
        "correlated-value-prop"
    }
}

/// Propagate equalities implied by conditional branches.
fn run_correlated_value_prop(
    function: &mir::Function,
    tree: &mut mir::Tree,
    domtree: &DominatorTree,
    cfg: &ControlFlowGraph,
    constants: &ConstantPropagation,
) -> bool {
    // build definition map for dominance checks
    let use_def = build_use_def_maps(function, tree);

    // build a lookup for condition instructions
    let value_to_instruction = build_value_instruction_map(function, tree);

    // track changes across the function
    let mut changed = false;

    // scan each block for equality conditions
    for &block_id in &function.blocks {
        // read the terminator to find a branch or check
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let (condition, then_target, else_target) = match terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => {
                let Some(condition) = condition.value() else {
                    continue;
                };
                let Some(then_target) = then_target.block.block() else {
                    continue;
                };
                let Some(else_target) = else_target.block.block() else {
                    continue;
                };

                (condition, then_target, else_target)
            }
            _ => continue,
        };

        // extract an equality condition
        if let Some(equality) = equality_condition(condition, &value_to_instruction) {
            // decide which successor is the equality path
            let equality_block = if equality.is_equal_on_then {
                then_target
            } else {
                else_target
            };

            // require the equality block to be reached only from this branch
            if is_single_predecessor(cfg, equality_block, block_id) {
                // use constant operands when available
                let constant_substitution =
                    constant_substitution(constants, block_id, equality.left, equality.right);

                // pick a canonical replacement value
                let replacement = if let Some((canonical, replace)) = constant_substitution {
                    Some((canonical, replace))
                } else {
                    choose_replacement(
                        function.entry,
                        equality.left,
                        equality.right,
                        equality_block,
                        &use_def.def_block,
                        domtree,
                    )
                };

                // apply the substitution when a distinct replacement exists
                if let Some((canonical, replace)) = replacement
                    && canonical != replace
                {
                    // apply substitutions in dominated blocks
                    let mut substitutions = HashMap::new();
                    substitutions.insert(replace, canonical);
                    let applied = apply_substitutions_in_dominated_blocks(
                        function,
                        tree,
                        domtree,
                        equality_block,
                        &substitutions,
                    );
                    changed |= applied;
                }
            }
        }

        // apply range constraints derived from the branch condition
        let range_constraints =
            range_constraints_for_condition(condition, &value_to_instruction, tree);
        let mut range_changed = false;

        // apply the constraint on the then edge
        if let Some(constraint) = range_constraints.then_constraint {
            // require a single predecessor on the constrained block
            if is_single_predecessor(cfg, then_target, block_id) {
                range_changed |= apply_range_constraint(
                    function,
                    tree,
                    domtree,
                    then_target,
                    &constraint,
                    &value_to_instruction,
                );
            }
        }

        // apply the constraint on the else edge
        if let Some(constraint) = range_constraints.else_constraint {
            // require a single predecessor on the constrained block
            if is_single_predecessor(cfg, else_target, block_id) {
                range_changed |= apply_range_constraint(
                    function,
                    tree,
                    domtree,
                    else_target,
                    &constraint,
                    &value_to_instruction,
                );
            }
        }

        // record whether range updates changed anything
        changed |= range_changed;
    }

    changed
}

/// A branch condition that implies equality along one successor.
struct EqualityCondition {
    /// Left hand value of the comparison.
    left: mir::Value,
    /// Right hand value of the comparison.
    right: mir::Value,
    /// True when equality holds on the then edge.
    is_equal_on_then: bool,
}

/// Constraint derived from a comparison branch.
struct RangeConstraint {
    /// The constrained value.
    value: mir::Value,
    /// The constrained range.
    range: ValueRange,
}

/// Pair of constraints for true and false edges.
struct RangeConstraintPair {
    /// Constraint that holds on the true edge.
    then_constraint: Option<RangeConstraint>,
    /// Constraint that holds on the false edge.
    else_constraint: Option<RangeConstraint>,
}

/// Extract equality information from a condition value.
fn equality_condition(
    condition: mir::Value,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
) -> Option<EqualityCondition> {
    // look up the defining instruction
    let instruction = value_to_instruction.get(&condition)?;

    // handle direct comparisons
    if let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = instruction
    {
        // map equality operators to the condition
        let Some(left) = left.value() else {
            return None;
        };
        let Some(right) = right.value() else {
            return None;
        };

        return match operator {
            mir::BinaryOperator::Equal => Some(EqualityCondition {
                left,
                right,
                is_equal_on_then: true,
            }),
            mir::BinaryOperator::NotEqual => Some(EqualityCondition {
                left,
                right,
                is_equal_on_then: false,
            }),
            _ => None,
        };
    }

    // handle logical negation of comparisons
    if let mir::Instruction::Unary {
        operator: mir::UnaryOperator::Not,
        argument,
        ..
    } = instruction
        && let Some(argument) = argument.value()
        && let Some(nested) = value_to_instruction.get(&argument)
        && let mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } = nested
    {
        // invert equality operators for the negated condition
        let Some(left) = left.value() else {
            return None;
        };
        let Some(right) = right.value() else {
            return None;
        };

        return match operator {
            mir::BinaryOperator::Equal => Some(EqualityCondition {
                left,
                right,
                is_equal_on_then: false,
            }),
            mir::BinaryOperator::NotEqual => Some(EqualityCondition {
                left,
                right,
                is_equal_on_then: true,
            }),
            _ => None,
        };
    }

    None
}

/// Extract range constraints from a comparison condition.
fn range_constraints_for_condition(
    condition: mir::Value,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
    tree: &mir::Tree,
) -> RangeConstraintPair {
    // default to no constraints
    let mut constraints = RangeConstraintPair {
        then_constraint: None,
        else_constraint: None,
    };

    // find the defining instruction
    let Some(instruction) = value_to_instruction.get(&condition) else {
        return constraints;
    };

    // unwrap the comparison
    let (operator, left, right) = match instruction {
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let Some(left) = left.value() else {
                return constraints;
            };
            let Some(right) = right.value() else {
                return constraints;
            };

            (*operator, left, right)
        }
        _ => return constraints,
    };

    // detect a constant operand
    let left_constant = constant_from_value(left, value_to_instruction, tree);
    let right_constant = constant_from_value(right, value_to_instruction, tree);

    // pick the non constant value to constrain
    let (value, constant, is_swapped) = match (left_constant, right_constant) {
        (Some(constant), None) => (right, constant, true),
        (None, Some(constant)) => (left, constant, false),
        _ => return constraints,
    };

    // derive a range constraint for integer comparisons
    let Some((then_range, else_range)) = integer_range_constraints(operator, &constant, is_swapped)
    else {
        return constraints;
    };

    // populate the constraint pair
    constraints.then_constraint = Some(RangeConstraint {
        value,
        range: then_range,
    });
    constraints.else_constraint = Some(RangeConstraint {
        value,
        range: else_range,
    });

    constraints
}

/// Extract a constant value for a SSA value if it is defined by a constant.
fn constant_from_value(
    value: mir::Value,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
    _tree: &mir::Tree,
) -> Option<mir::Constant> {
    // look up the defining instruction
    let instruction = value_to_instruction.get(&value)?;

    // map constants to their values
    match instruction {
        mir::Instruction::Const { value, .. } => Some(value.clone()),
        _ => None,
    }
}

/// Derive integer range constraints for a comparison.
fn integer_range_constraints(
    operator: mir::BinaryOperator,
    constant: &mir::Constant,
    is_swapped: bool,
) -> Option<(ValueRange, ValueRange)> {
    // decode integer constant
    let (constant_value, width, is_signed) = match constant {
        mir::Constant::Int {
            value,
            width,
            is_signed,
        } => (*value, *width, *is_signed),
        mir::Constant::UInt { value, width } => {
            let value = i128::try_from(*value).ok()?;

            (value, *width, false)
        }
        _ => return None,
    };

    // resolve full type bounds
    let (full_min, full_max) = integer_full_bounds(width, is_signed)?;

    // flip operator when constant is on the left
    let operator = if is_swapped {
        swap_comparison_operator(operator)?
    } else {
        operator
    };

    // derive range for the true edge
    let (then_min, then_max, else_min, else_max) = match operator {
        mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::UnsignedLessThan => (
            full_min,
            constant_value.saturating_sub(1),
            constant_value,
            full_max,
        ),
        mir::BinaryOperator::SignedLessEqual | mir::BinaryOperator::UnsignedLessEqual => (
            full_min,
            constant_value,
            constant_value.saturating_add(1),
            full_max,
        ),
        mir::BinaryOperator::SignedGreaterThan | mir::BinaryOperator::UnsignedGreaterThan => (
            constant_value.saturating_add(1),
            full_max,
            full_min,
            constant_value,
        ),
        mir::BinaryOperator::SignedGreaterEqual | mir::BinaryOperator::UnsignedGreaterEqual => (
            constant_value,
            full_max,
            full_min,
            constant_value.saturating_sub(1),
        ),
        _ => return None,
    };

    // clamp ranges to the type bounds
    let then_range = integer_range_from_bounds(then_min, then_max, width, is_signed)?;
    let else_range = integer_range_from_bounds(else_min, else_max, width, is_signed)?;

    Some((then_range, else_range))
}

/// Return the full integer bounds for a type.
fn integer_full_bounds(width: u16, is_signed: bool) -> Option<(i128, i128)> {
    // reject unsupported widths
    if width == 0 || width > 127 {
        return None;
    }

    // compute the maximum bound
    let max = if is_signed {
        (1i128 << (width - 1)) - 1
    } else {
        (1i128 << width) - 1
    };

    // compute the minimum bound
    let min = if is_signed {
        -(1i128 << (width - 1))
    } else {
        0
    };

    Some((min, max))
}

/// Clamp integer bounds into a ValueRange.
fn integer_range_from_bounds(
    min: i128,
    max: i128,
    width: u16,
    is_signed: bool,
) -> Option<ValueRange> {
    // reject empty ranges
    if min > max {
        return None;
    }

    // clamp to the full type bounds
    let (full_min, full_max) = integer_full_bounds(width, is_signed)?;
    let min = min.max(full_min);
    let max = max.min(full_max);

    // reject empty ranges after clamping
    if min > max {
        return None;
    }

    Some(ValueRange::Integer {
        min,
        max,
        width,
        is_signed,
    })
}

/// Apply a range constraint by folding dominated comparisons.
fn apply_range_constraint(
    function: &mir::Function,
    tree: &mut mir::Tree,
    domtree: &DominatorTree,
    root: mir::LocalNodeId<mir::Block>,
    constraint: &RangeConstraint,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
) -> bool {
    // collect dominated blocks
    let mut blocks = Vec::new();
    for &block_id in &function.blocks {
        // record blocks dominated by the root
        if domtree.dominates(root, block_id) {
            blocks.push(block_id);
        }
    }

    // fold comparisons using the constraint
    let mut changed = false;
    for block_id in blocks {
        // read the block instruction list
        let block = tree.get(block_id);
        let instruction_ids: Vec<_> = block.instructions.clone();

        // scan instructions for comparisons
        for instruction_id in instruction_ids {
            // skip non comparison instructions
            let instruction = tree.get(instruction_id);
            let mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } = instruction
            else {
                continue;
            };
            let Some(destination) = destination.value() else {
                continue;
            };
            let Some(left) = left.value() else {
                continue;
            };
            let Some(right) = right.value() else {
                continue;
            };

            // fold the comparison when constrained
            let comparison = comparison_from_range(
                *operator,
                left,
                right,
                constraint,
                value_to_instruction,
                tree,
            );

            // replace with a constant when the outcome is known
            if let Some(result) = comparison {
                let new_instruction = mir::Instruction::Const {
                    destination: destination.into(),
                    value: mir::Constant::Boolean { value: result },
                };
                tree.set(instruction_id, new_instruction);
                changed = true;
            }
        }
    }

    changed
}

/// Determine if a comparison is constant given a range constraint.
fn comparison_from_range(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    constraint: &RangeConstraint,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
    tree: &mir::Tree,
) -> Option<bool> {
    // identify the constrained operand
    let (is_left, constant_value) = if left == constraint.value {
        (
            true,
            constant_to_i128(constant_from_value(right, value_to_instruction, tree)?)?,
        )
    } else if right == constraint.value {
        (
            false,
            constant_to_i128(constant_from_value(left, value_to_instruction, tree)?)?,
        )
    } else {
        return None;
    };

    // apply the constraint for integer ranges only
    let ValueRange::Integer { min, max, .. } = constraint.range else {
        return None;
    };

    // adjust operator if the constrained value is on the right
    let operator = if is_left {
        operator
    } else {
        swap_comparison_operator(operator)?
    };

    // evaluate comparison outcome
    match operator {
        mir::BinaryOperator::SignedLessThan | mir::BinaryOperator::UnsignedLessThan => {
            if max < constant_value {
                Some(true)
            } else if min >= constant_value {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedLessEqual | mir::BinaryOperator::UnsignedLessEqual => {
            if max <= constant_value {
                Some(true)
            } else if min > constant_value {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedGreaterThan | mir::BinaryOperator::UnsignedGreaterThan => {
            if min > constant_value {
                Some(true)
            } else if max <= constant_value {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::SignedGreaterEqual | mir::BinaryOperator::UnsignedGreaterEqual => {
            if min >= constant_value {
                Some(true)
            } else if max < constant_value {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::Equal => {
            if min == max && min == constant_value {
                Some(true)
            } else if constant_value < min || constant_value > max {
                Some(false)
            } else {
                None
            }
        }
        mir::BinaryOperator::NotEqual => {
            if min == max && min == constant_value {
                Some(false)
            } else if constant_value < min || constant_value > max {
                Some(true)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Convert an integer constant to i128.
fn constant_to_i128(constant: mir::Constant) -> Option<i128> {
    // map integer constants to i128
    match constant {
        mir::Constant::Int { value, .. } => Some(value),
        mir::Constant::UInt { value, .. } => i128::try_from(value).ok(),
        _ => None,
    }
}

/// Choose the value to substitute within a dominated block.
fn choose_replacement(
    entry: Option<mir::LocalNodeId<mir::Block>>,
    left: mir::Value,
    right: mir::Value,
    block: mir::LocalNodeId<mir::Block>,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> Option<(mir::Value, mir::Value)> {
    // resolve definition blocks with entry fallback
    let entry_block = entry?;
    let left_def = def_blocks.get(&left).copied().unwrap_or(entry_block);
    let right_def = def_blocks.get(&right).copied().unwrap_or(entry_block);

    // determine availability in the dominated region
    let left_available = domtree.dominates(left_def, block);
    let right_available = domtree.dominates(right_def, block);

    // select a replacement value when available
    match (left_available, right_available) {
        (true, true) => {
            // prefer the smaller value id for determinism
            if left.0 <= right.0 {
                Some((left, right))
            } else {
                Some((right, left))
            }
        }
        (true, false) => Some((left, right)),
        (false, true) => Some((right, left)),
        (false, false) => None,
    }
}

/// Pick a constant operand as the canonical value when safe.
fn constant_substitution(
    constants: &ConstantPropagation,
    block_id: mir::LocalNodeId<mir::Block>,
    left: mir::Value,
    right: mir::Value,
) -> Option<(mir::Value, mir::Value)> {
    // read the constant map for this block
    let block_constants = constants.exit(block_id);
    let left_constant = block_constants.get(left);
    let right_constant = block_constants.get(right);

    // prefer the constant operand when only one side is constant
    match (left_constant, right_constant) {
        (Some(constant), None) if constant_is_integer_like(constant) => Some((left, right)),
        (None, Some(constant)) if constant_is_integer_like(constant) => Some((right, left)),
        _ => None,
    }
}

/// Check whether a constant is safe to propagate as an equality operand.
fn constant_is_integer_like(constant: &mir::Constant) -> bool {
    // restrict to integer like scalars for equality propagation
    matches!(
        constant,
        mir::Constant::Boolean { .. }
            | mir::Constant::Int { .. }
            | mir::Constant::UInt { .. }
            | mir::Constant::Char { .. }
    )
}

/// Check whether a block has a single predecessor and it matches the expected block.
fn is_single_predecessor(
    cfg: &ControlFlowGraph,
    block: mir::LocalNodeId<mir::Block>,
    expected: mir::LocalNodeId<mir::Block>,
) -> bool {
    // read predecessor list
    let predecessors = cfg.predecessors(block);

    // require all predecessors to be the expected block
    !predecessors.is_empty() && predecessors.iter().all(|pred| *pred == expected)
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::CorrelatedValueProp;

    /// Equality branches substitute the dominated value.
    #[test]
    fn test_cvp_substitutes_equal_values() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
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

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v0
    jump b3(v3)
b2:
    v4: int32 = int.sub v0, v1
    jump b3(v4)
b3(v5: int32):
    return v5
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }

    /// Not equal conditions propagate equality on the false edge.
    #[test]
    fn test_cvp_inverts_not_equal() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.ne v0, v1
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    return v3
b2:
    v4: int32 = int.sub v0, v1
    return v4
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.ne v0, v1
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    return v3
b2:
    v4: int32 = int.sub v0, v0
    return v4
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }

    /// Constant equalities substitute the non constant operand.
    #[test]
    fn test_cvp_prefers_constant_operand() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 7int32
    v3: boolean = int.eq v0, v2
    branch v3, b1, b2
b1:
    v4: int32 = int.add v0, v1
    return v4
b2:
    return v0
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = 7int32
    v3: boolean = int.eq v0, v2
    branch v3, b1, b2
b1:
    v4: int32 = int.add v2, v1
    return v4
b2:
    return v0
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }

    /// Float equality is not substituted.
    #[test]
    fn test_cvp_skips_float_equal() {
        // source test
        let input = r#"
function test(v0: float64, v1: float64): float64 {
b0(v0: float64, v1: float64):
    v2: boolean = float.eq v0, v1
    branch v2, b1, b2
b1:
    v3: float64 = float.add v0, v1
    return v3
b2:
    v4: float64 = float.sub v0, v1
    return v4
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(input);
    }

    /// Substitution flows into blocks dominated by the equality edge.
    #[test]
    fn test_cvp_propagates_into_dominated_blocks() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2
b1:
    jump b3
b2:
    return v0
b3:
    v3: int32 = int.add v0, v1
    return v3
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2
b1:
    jump b3
b2:
    return v0
b3:
    v3: int32 = int.add v0, v0
    return v3
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }

    /// Negated equality conditions substitute on the else edge.
    #[test]
    fn test_cvp_handles_negated_equal() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    v3: boolean = int.not v2
    branch v3, b1, b2
b1:
    v4: int32 = int.sub v0, v1
    return v4
b2:
    v5: int32 = int.add v0, v1
    return v5
}"#;

        // expected output
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    v3: boolean = int.not v2
    branch v3, b1, b2
b1:
    v4: int32 = int.sub v0, v1
    return v4
b2:
    v5: int32 = int.add v0, v0
    return v5
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }

    /// Check terminators propagate equality on the success edge.
    #[test]
    fn test_cvp_handles_check_terminator() {
        // source test
        let input = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
b0(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    check bounds.u v0, v1, v2 -> b1, b2
b1:
    v4: uint32 = int.add v0, v1
    return v4
b2:
    return v0
}"#;

        // expected output
        let expected = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
b0(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    check bounds.u v0, v1, v2 -> b1, b2
b1:
    v4: uint32 = int.add v0, v0
    return v4
b2:
    return v0
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }

    /// Equality blocks with extra predecessors are not substituted.
    #[test]
    fn test_cvp_requires_single_predecessor() {
        // source test
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: boolean = int.eq v0, v1
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    return v3
b2:
    jump b1
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(input);
    }

    /// Range constraints fold comparisons on dominated paths.
    #[test]
    fn test_cvp_range_constraint_then_edge() {
        // source test
        let input = r#"
function test(v0: int32): boolean {
b0(v0: int32):
    v1: int32 = 5int32
    v2: boolean = int.lt.s v0, v1
    branch v2, b1, b2
b1:
    v3: boolean = int.lt.s v0, v1
    return v3
b2:
    return v2
}"#;

        // expected output
        let expected = r#"
function test(v0: int32): boolean {
b0(v0: int32):
    v1: int32 = 5int32
    v2: boolean = int.lt.s v0, v1
    branch v2, b1, b2
b1:
    v3: boolean = true
    return v3
b2:
    return v2
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }

    /// Range constraints fold comparisons on the false edge.
    #[test]
    fn test_cvp_range_constraint_else_edge() {
        // source test
        let input = r#"
function test(v0: int32): boolean {
b0(v0: int32):
    v1: int32 = 5int32
    v2: boolean = int.lt.s v0, v1
    branch v2, b1, b2
b1:
    return v2
b2:
    v3: boolean = int.lt.s v0, v1
    return v3
}"#;

        // expected output
        let expected = r#"
function test(v0: int32): boolean {
b0(v0: int32):
    v1: int32 = 5int32
    v2: boolean = int.lt.s v0, v1
    branch v2, b1, b2
b1:
    return v2
b2:
    v3: boolean = false
    return v3
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&CorrelatedValueProp);
        test.assert_output(expected);
    }
}
