use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, RangeAnalysis, RangeMap, ScalarEvolution,
    Scev, ValueRange,
};
use crate::optimize::common::{BlockParamForwarding, constant_zero_like};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Eliminate bounds checks dominated by loop guards.
    ///
    /// Uses loop guard comparisons to remove redundant bounds checks inside
    /// loop bodies when the guard implies the bounds check is satisfied.
    ///
    /// ```mir
    /// function before(v0: int32[4]): void {
    /// b0(v0: int32[4]):
    ///     v1 = 0uint32
    ///     v2 = 1uint32
    ///     v3 = 4uint32
    ///     jump b1(v1)
    /// b1(v4: uint32):
    ///     v5 = int.lt.u v4, v3
    ///     branch v5, b2, b3
    /// b2:
    ///     v6 = int.lt.u v4, v3
    ///     check bounds.u v4, v3, v0 -> b4, b5
    /// b4:
    ///     v7 = int.add v4, v2
    ///     jump b1(v7)
    /// b5:
    ///     unreachable
    /// b3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32[4]): void {
    /// b0(v0: int32[4]):
    ///     v1 = 0uint32
    ///     v2 = 1uint32
    ///     v3 = 4uint32
    ///     jump b1(v1)
    /// b1(v4: uint32):
    ///     v5 = int.lt.u v4, v3
    ///     branch v5, b2, b3
    /// b2:
    ///     v6 = int.lt.u v4, v3
    ///     jump b4
    /// b4:
    ///     v7 = int.add v4, v2
    ///     jump b1(v7)
    /// b5:
    ///     unreachable
    /// b3:
    ///     return
    /// }
    /// ```
    #[pass(id = "loop-bounds-check-eliminate")]
    pub LoopBoundsCheckEliminate,
    "Eliminate redundant loop bounds checks"
}

impl FunctionPass for LoopBoundsCheckEliminate {
    /// Run the loop bounds check elimination pass.
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

        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let loops = analyses.get::<LoopAnalysis>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let ranges = analyses.get::<RangeAnalysis>().clone();
        let scev = analyses.get::<ScalarEvolution>().clone();

        // skip when no loops are present
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }

        // build helper state
        let definitions = ValueDefinitions::build(function, tree);
        let forwarding = BlockParamForwarding::build(function, tree, &cfg);

        // run the elimination pass
        let changed = run_loop_bounds_check_eliminate(
            tree,
            &loops,
            &domtree,
            &ranges,
            &scev,
            &definitions,
            &forwarding,
        );
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "LoopBoundsCheckEliminate"
    }

    /// Return the stable id for this pass.
    fn id(&self) -> &'static str {
        "loop-bounds-check-eliminate"
    }
}

/// Definition kind for a value.
#[derive(Debug, Clone, Copy)]
enum ValueDefinitionKind {
    /// Block parameter definition.
    Parameter,
    /// Instruction definition.
    Instruction {
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

/// Definition metadata for a value.
#[derive(Debug, Clone, Copy)]
struct ValueDefinition {
    /// Definition kind.
    kind: ValueDefinitionKind,
}

/// Map of values to definitions.
#[derive(Debug)]
struct ValueDefinitions {
    /// Definitions keyed by value.
    definitions: HashMap<mir::Value, ValueDefinition>,
}

impl ValueDefinitions {
    /// Build a definition map for a function.
    fn build(function: &mir::Function, tree: &mir::NodeTree) -> Self {
        // collect parameter and instruction definitions
        let mut definitions = HashMap::new();

        // scan blocks for definitions
        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            // record block parameters
            for param in block.parameters.iter() {
                let Some(value) = param.value.value() else {
                    continue;
                };

                definitions.insert(
                    value,
                    ValueDefinition {
                        kind: ValueDefinitionKind::Parameter,
                    },
                );
            }

            // record instruction destinations
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination().and_then(|value| value.value())
                {
                    definitions.insert(
                        destination,
                        ValueDefinition {
                            kind: ValueDefinitionKind::Instruction {
                                instruction: instruction_id,
                            },
                        },
                    );
                }
            }
        }

        Self { definitions }
    }

    /// Get the definition metadata for a value.
    fn definition_for(&self, value: mir::Value) -> Option<ValueDefinition> {
        self.definitions.get(&value).copied()
    }
}

/// Affine representation for a loop value.
#[derive(Debug, Clone)]
struct AffineValue {
    /// Base symbolic expression.
    base: Scev,
    /// Constant offset applied to the base.
    offset: i128,
}

impl AffineValue {
    /// Return true when the base expression matches another.
    fn base_matches(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

/// Loop guard constraint derived from a comparison.
#[derive(Debug, Clone, Copy)]
struct LoopGuard {
    /// Guard block that dominates the loop body.
    block: mir::LocalNodeId<mir::Block>,
    /// Index value compared against length.
    index: mir::Value,
    /// Length value compared against index.
    length: mir::Value,
    /// Whether the comparison is signed.
    is_signed: bool,
}

/// Guard information for a loop.
#[derive(Debug, Clone)]
struct LoopGuards {
    /// Bounds guards derived from loop exits.
    bounds: Vec<LoopGuard>,
    /// Values proven non negative on the loop path.
    non_negative: Vec<NonNegativeGuard>,
}

/// Guard metadata for non negative proofs.
#[derive(Debug, Clone, Copy)]
struct NonNegativeGuard {
    /// Block that establishes the guard.
    block: mir::LocalNodeId<mir::Block>,
    /// Value proven non negative.
    value: mir::Value,
}

/// Eliminate redundant bounds checks inside loops.
fn run_loop_bounds_check_eliminate(
    tree: &mut mir::NodeTree,
    loops: &LoopAnalysis,
    domtree: &DominatorTree,
    ranges: &RangeAnalysis,
    scev: &ScalarEvolution,
    definitions: &ValueDefinitions,
    forwarding: &BlockParamForwarding,
) -> bool {
    // collect guards for each loop
    let mut loop_guards: HashMap<usize, LoopGuards> = HashMap::new();

    for (loop_index, lp) in loops.loops().iter().enumerate() {
        // derive guards from exiting blocks
        let guards = collect_loop_guards(lp, tree, definitions, ranges);

        // record non empty guard sets
        if !guards.bounds.is_empty() || !guards.non_negative.is_empty() {
            loop_guards.insert(loop_index, guards);
        }
    }

    // remove redundant checks
    let mut changed = false;

    for (loop_index, lp) in loops.loops().iter().enumerate() {
        // skip loops without guards
        let Some(guards) = loop_guards.get(&loop_index) else {
            continue;
        };

        // scan loop blocks for check terminators
        for &block_id in &lp.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator).clone();
            let (constraint, success) = match terminator {
                mir::Terminator::Check {
                    constraint,
                    success,
                    failure: _,
                } => (constraint, success),
                _ => continue,
            };

            // require bounds check constraints
            let mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } = &constraint
            else {
                continue;
            };
            let Some(index) = index.value() else {
                continue;
            };
            let Some(length) = length.value() else {
                continue;
            };

            // find a guard that implies the bounds check
            let mut guard_implies = false;

            for guard in &guards.bounds {
                // skip guards that cannot imply the check
                if guard.block == block_id
                    || !domtree.dominates(guard.block, block_id)
                    || guard.is_signed != *is_signed
                {
                    continue;
                }

                // compare affine forms for guard and check values
                let guard_index = affine_value_for_loop(guard.index, loop_index, scev, forwarding);
                let guard_length =
                    affine_value_for_loop(guard.length, loop_index, scev, forwarding);
                let check_index = affine_value_for_loop(index, loop_index, scev, forwarding);
                let check_length = affine_value_for_loop(length, loop_index, scev, forwarding);

                // require guard to imply the bounds
                if !guard_implies_bounds(&guard_index, &guard_length, &check_index, &check_length) {
                    continue;
                }

                // require non negative signed indices
                if *is_signed
                    && !index_non_negative(index, block_id, ranges, forwarding)
                    && !index_non_negative_from_guards(
                        index,
                        block_id,
                        loop_index,
                        scev,
                        forwarding,
                        domtree,
                        &guards.non_negative,
                    )
                {
                    continue;
                }

                guard_implies = true;
                break;
            }

            // skip checks that are not implied
            if !guard_implies {
                continue;
            }

            // replace the check with an unconditional jump
            replace_terminator_with_jump(tree, block_id, success);
            changed = true;
        }
    }

    changed
}

/// Collect guard comparisons that imply index less than length.
fn collect_loop_guards(
    lp: &Loop,
    tree: &mir::NodeTree,
    definitions: &ValueDefinitions,
    ranges: &RangeAnalysis,
) -> LoopGuards {
    // collect guard comparisons
    let mut bounds = Vec::new();
    let mut non_negative = Vec::new();

    // scan exiting blocks for guards
    for &block_id in &lp.exiting_blocks {
        // read the exiting block
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // collect bounds from check terminators when available
        if let mir::Terminator::Check {
            constraint,
            success,
            failure,
            ..
        } = terminator
            && let mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } = constraint
        {
            let Some(success) = success.block.block() else {
                continue;
            };
            let Some(failure) = failure.block.block() else {
                continue;
            };
            let Some(index) = index.value() else {
                continue;
            };
            let Some(length) = length.value() else {
                continue;
            };

            // ensure the success edge stays inside the loop
            let success_in_loop = lp.blocks.contains(&success);
            let failure_in_loop = lp.blocks.contains(&failure);
            if success_in_loop != failure_in_loop && success_in_loop {
                bounds.push(LoopGuard {
                    block: block_id,
                    index,
                    length,
                    is_signed: *is_signed,
                });

                // record signed guards as non negative proofs
                if *is_signed {
                    non_negative.push(NonNegativeGuard {
                        block: block_id,
                        value: index,
                    });
                }

                continue;
            }
        }

        // extract the in loop guard condition
        let Some((condition, guard_is_true)) = guard_condition(terminator.clone(), lp) else {
            continue;
        };

        // require a comparison instruction for the guard
        let Some((operator, left, right, guard_is_true)) =
            guard_comparison(condition, guard_is_true, definitions, tree)
        else {
            continue;
        };

        // record non negative guards when possible
        let ranges_at_guard = ranges.exit(block_id);
        if let Some(value) =
            guard_non_negative(operator, left, right, guard_is_true, ranges_at_guard)
        {
            non_negative.push(NonNegativeGuard {
                block: block_id,
                value,
            });
        }

        // normalize to a strict less than guard
        let Some(guard_info) = guard_less_than(operator, left, right, guard_is_true) else {
            continue;
        };

        // record the guard info
        bounds.push(LoopGuard {
            block: block_id,
            index: guard_info.index,
            length: guard_info.length,
            is_signed: guard_info.is_signed,
        });
    }

    LoopGuards {
        bounds,
        non_negative,
    }
}

/// Extract a guard comparison from a condition value.
fn guard_comparison(
    condition: mir::Value,
    guard_is_true: bool,
    definitions: &ValueDefinitions,
    tree: &mir::NodeTree,
) -> Option<(mir::BinaryOperator, mir::Value, mir::Value, bool)> {
    // resolve the condition instruction
    let definition = definitions.definition_for(condition)?;
    let ValueDefinitionKind::Instruction { instruction } = definition.kind else {
        return None;
    };
    let instruction = tree.get(instruction);

    // handle direct comparisons
    if let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = instruction
    {
        let Some(left) = left.value() else {
            return None;
        };
        let Some(right) = right.value() else {
            return None;
        };

        return Some((*operator, left, right, guard_is_true));
    }

    // handle negated comparisons
    if let mir::Instruction::Unary {
        operator: mir::UnaryOperator::Not,
        argument,
        ..
    } = instruction
    {
        let argument = argument.value()?;
        let nested_definition = definitions.definition_for(argument)?;
        let ValueDefinitionKind::Instruction { instruction } = nested_definition.kind else {
            return None;
        };
        let nested = tree.get(instruction);
        let mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } = nested
        else {
            return None;
        };

        let Some(left) = left.value() else {
            return None;
        };
        let Some(right) = right.value() else {
            return None;
        };

        return Some((*operator, left, right, !guard_is_true));
    }

    None
}

/// Guard comparison info for an in loop less than condition.
#[derive(Debug, Clone, Copy)]
struct GuardLessThan {
    /// Index on the less than side.
    index: mir::Value,
    /// Length on the greater side.
    length: mir::Value,
    /// Signedness of the comparison.
    is_signed: bool,
}

/// Extract the condition and truth value for the in loop edge.
fn guard_condition(terminator: mir::Terminator, lp: &Loop) -> Option<(mir::Value, bool)> {
    // match terminators that select loop exits
    match terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
            ..
        } => {
            let Some(condition) = condition.value() else {
                return None;
            };
            let Some(then_target) = then_target.block.block() else {
                return None;
            };
            let Some(else_target) = else_target.block.block() else {
                return None;
            };

            // determine which branch stays inside the loop
            let then_in_loop = lp.blocks.contains(&then_target);
            let else_in_loop = lp.blocks.contains(&else_target);
            if then_in_loop == else_in_loop {
                return None;
            }

            // return the guard value for the in loop edge
            let guard_is_true = then_in_loop;
            Some((condition, guard_is_true))
        }
        mir::Terminator::Check { .. } => None,
        // no guard for other terminators
        _ => None,
    }
}

/// Convert a comparison into a strict less than guard when possible.
fn guard_less_than(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    guard_is_true: bool,
) -> Option<GuardLessThan> {
    // map comparisons to strict less than forms
    match (operator, guard_is_true) {
        (mir::BinaryOperator::SignedLessThan, true) => Some(GuardLessThan {
            index: left,
            length: right,
            is_signed: true,
        }),
        (mir::BinaryOperator::UnsignedLessThan, true) => Some(GuardLessThan {
            index: left,
            length: right,
            is_signed: false,
        }),
        (mir::BinaryOperator::SignedGreaterThan, true) => Some(GuardLessThan {
            index: right,
            length: left,
            is_signed: true,
        }),
        (mir::BinaryOperator::UnsignedGreaterThan, true) => Some(GuardLessThan {
            index: right,
            length: left,
            is_signed: false,
        }),
        (mir::BinaryOperator::SignedGreaterEqual, false) => Some(GuardLessThan {
            index: left,
            length: right,
            is_signed: true,
        }),
        (mir::BinaryOperator::UnsignedGreaterEqual, false) => Some(GuardLessThan {
            index: left,
            length: right,
            is_signed: false,
        }),
        _ => None,
    }
}

/// Identify values that are proven non negative by a guard comparison.
fn guard_non_negative(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    guard_is_true: bool,
    ranges: &RangeMap,
) -> Option<mir::Value> {
    // map comparisons into value >= bound when possible
    let right_constant = signed_constant_from_range(right, ranges);
    let left_constant = signed_constant_from_range(left, ranges);

    // prefer variable on the left side
    let (value, constant, bound_value, operator) = if let Some(constant) = right_constant {
        (left, Some(constant), right, operator)
    } else if let Some(constant) = left_constant {
        let flipped = flip_signed_comparison(operator)?;
        (right, Some(constant), left, flipped)
    } else {
        (left, None, right, operator)
    };

    // derive a lower bound from the guard
    let (inclusive, bound) = match (operator, guard_is_true) {
        (mir::BinaryOperator::SignedGreaterEqual, true) => (true, constant),
        (mir::BinaryOperator::SignedGreaterThan, true) => (false, constant),
        (mir::BinaryOperator::SignedLessThan, false) => (true, constant),
        (mir::BinaryOperator::SignedLessEqual, false) => (false, constant),
        _ => return None,
    };

    // confirm the lower bound implies non negativity
    if let Some(bound) = bound {
        if inclusive {
            if bound >= 0 {
                return Some(value);
            }
        } else if bound >= -1 {
            return Some(value);
        }

        return None;
    }

    // fall back to range analysis for the bound value
    let bound_min = signed_range_min(bound_value, ranges)?;

    // evaluate the range based lower bound
    if inclusive {
        if bound_min >= 0 {
            return Some(value);
        }
    } else if bound_min >= -1 {
        return Some(value);
    }

    None
}

/// Flip a signed comparison when swapping operands.
fn flip_signed_comparison(operator: mir::BinaryOperator) -> Option<mir::BinaryOperator> {
    // map signed comparisons to flipped operators
    match operator {
        mir::BinaryOperator::SignedLessThan => Some(mir::BinaryOperator::SignedGreaterThan),
        mir::BinaryOperator::SignedLessEqual => Some(mir::BinaryOperator::SignedGreaterEqual),
        mir::BinaryOperator::SignedGreaterThan => Some(mir::BinaryOperator::SignedLessThan),
        mir::BinaryOperator::SignedGreaterEqual => Some(mir::BinaryOperator::SignedLessEqual),
        _ => None,
    }
}

/// Build an affine representation for a loop value.
fn affine_value_for_loop(
    value: mir::Value,
    loop_index: usize,
    scev: &ScalarEvolution,
    forwarding: &BlockParamForwarding,
) -> AffineValue {
    // resolve forwarded values
    let value = forwarding.resolve(value);

    // prefer scalar evolution when available
    let scev = scev
        .scev_for_value_in_loop(loop_index, value)
        .cloned()
        .unwrap_or(Scev::Unknown(value));

    // split constant offsets when possible
    affine_from_scev(&scev)
}

/// Build an affine value from a scalar evolution expression.
fn affine_from_scev(scev: &Scev) -> AffineValue {
    // use constant splitting when available
    if let Some((base, offset)) = split_scev_offset(scev) {
        return AffineValue { base, offset };
    }

    // fall back to the original expression
    AffineValue {
        base: scev.clone(),
        offset: 0,
    }
}

/// Return true when a guard implies a bounds check.
fn guard_implies_bounds(
    guard_index: &AffineValue,
    guard_length: &AffineValue,
    check_index: &AffineValue,
    check_length: &AffineValue,
) -> bool {
    // require matching base expressions
    if !guard_index.base_matches(check_index) || !guard_length.base_matches(check_length) {
        return false;
    }

    // require check index offset no larger than guard index offset
    if check_index.offset > guard_index.offset {
        return false;
    }

    // require check length offset no smaller than guard length offset
    if check_length.offset < guard_length.offset {
        return false;
    }

    true
}

/// Split constant offsets from a scalar evolution expression when possible.
fn split_scev_offset(scev: &Scev) -> Option<(Scev, i128)> {
    // extract constants directly
    if let Some(constant) = scev_constant(scev) {
        let offset = constant_to_i128(constant)?;
        let base = Scev::Constant(constant_zero_like(constant));
        return Some((base, offset));
    }

    // fold constants on additive expressions
    if let Scev::Add(left, right) = scev {
        // handle constants on the right side
        if let Some(constant) = scev_constant(right) {
            let offset = constant_to_i128(constant)?;
            let (base, base_offset) =
                split_scev_offset(left).unwrap_or_else(|| (left.as_ref().clone(), 0));
            return Some((base, base_offset + offset));
        }

        // handle constants on the left side
        if let Some(constant) = scev_constant(left) {
            let offset = constant_to_i128(constant)?;
            let (base, base_offset) =
                split_scev_offset(right).unwrap_or_else(|| (right.as_ref().clone(), 0));
            return Some((base, base_offset + offset));
        }
    }

    // split constant starts from additive recurrences
    if let Scev::AddRec {
        start,
        step,
        loop_header,
    } = scev
        && let Scev::Constant(constant) = start.as_ref()
    {
        let offset = constant_to_i128(constant)?;
        let base = Scev::AddRec {
            start: Box::new(Scev::Constant(constant_zero_like(constant))),
            step: step.clone(),
            loop_header: *loop_header,
        };
        return Some((base, offset));
    }

    // no constant offset found
    None
}

/// Return the constant for a scalar evolution expression when available.
fn scev_constant(scev: &Scev) -> Option<&mir::Constant> {
    // extract the constant node when present
    match scev {
        Scev::Constant(constant) => Some(constant),
        _ => None,
    }
}

/// Convert a constant into a signed offset.
fn constant_to_i128(constant: &mir::Constant) -> Option<i128> {
    // convert integer constants to a signed offset
    match constant {
        mir::Constant::Int { value, .. } => Some(i128::from(*value)),
        mir::Constant::UInt { value, .. } => Some(i128::from(*value)),
        _ => None,
    }
}

/// Check if an index is known non negative from guard comparisons.
fn index_non_negative_from_guards(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    loop_index: usize,
    scev: &ScalarEvolution,
    forwarding: &BlockParamForwarding,
    domtree: &DominatorTree,
    non_negative: &[NonNegativeGuard],
) -> bool {
    // build the affine form for the index
    let index_affine = affine_value_for_loop(value, loop_index, scev, forwarding);

    // match against guard values using base and offset comparisons
    non_negative.iter().any(|guard| {
        // ignore guards from the same block
        if guard.block == block_id {
            return false;
        }

        // require the guard to dominate the check block
        if !domtree.dominates(guard.block, block_id) {
            return false;
        }

        // compare the base expression for the guard
        let guard_affine = affine_value_for_loop(guard.value, loop_index, scev, forwarding);
        if !guard_affine.base_matches(&index_affine) {
            return false;
        }

        // require the guard offset to be no larger than the index offset
        index_affine.offset >= guard_affine.offset
    })
}

/// Check if a value is known to be non negative at a block.
fn index_non_negative(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    ranges: &RangeAnalysis,
    forwarding: &BlockParamForwarding,
) -> bool {
    // use the forwarded value for range queries
    let value = forwarding.resolve(value);

    // read the range at the block entry
    let range = ranges.entry(block_id).get(value);
    let Some(ValueRange::Integer { min, is_signed, .. }) = range else {
        return false;
    };

    // reject unsigned ranges
    if !*is_signed {
        return false;
    }

    // require a non negative minimum bound
    *min >= 0
}

/// Extract an integer constant from a value range.
fn signed_constant_from_range(value: mir::Value, ranges: &RangeMap) -> Option<i128> {
    // read the constant range when available
    let range = ranges.get(value)?;
    let constant = range.as_constant()?;

    // extract signed integer values
    match constant {
        mir::Constant::Int { value, .. } => Some(value as i128),
        _ => None,
    }
}

/// Extract the minimum signed bound for a value range.
fn signed_range_min(value: mir::Value, ranges: &RangeMap) -> Option<i128> {
    // resolve the integer range for the value
    let range = ranges.get(value)?;
    let ValueRange::Integer { min, is_signed, .. } = range else {
        return None;
    };

    // require a signed range
    if !*is_signed {
        return None;
    }

    Some(*min)
}

/// Replace a block terminator with a jump.
fn replace_terminator_with_jump(
    tree: &mut mir::NodeTree,
    block_id: mir::LocalNodeId<mir::Block>,
    target: mir::BlockTarget,
) {
    // overwrite the terminator with a jump
    let block = tree.get(block_id);
    let terminator = mir::Terminator::Jump { target };
    tree.replace(block.terminator, terminator);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Loop guard eliminates redundant bounds checks.
    #[test]
    fn test_eliminate_loop_bounds_check() {
        // source test
        let input = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b5
b2:
    v6: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b3, b4
b3:
    v7: uint32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b5
b2:
    v6: boolean = int.lt.u v4, v3
    jump b3
b3:
    v7: uint32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Latch guards do not eliminate earlier checks.
    #[test]
    fn test_preserve_pre_guard_checks() {
        // source test
        let input = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b2, b3
b2:
    v6: uint32 = int.add v4, v2
    v7: boolean = int.lt.u v6, v3
    branch v7, b1(v6), b4
b3:
    unreachable
b4:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b2, b3
b2:
    v6: uint32 = int.add v4, v2
    v7: boolean = int.lt.u v6, v3
    branch v7, b1(v6), b4
b3:
    unreachable
b4:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Unsigned greater equal guards eliminate redundant checks.
    #[test]
    fn test_eliminate_ge_guard_checks() {
        // source test
        let input = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.ge.u v4, v3
    branch v5, b5, b2
b2:
    v6: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b3, b4
b3:
    v7: uint32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.ge.u v4, v3
    branch v5, b5, b2
b2:
    v6: boolean = int.lt.u v4, v3
    jump b3
b3:
    v7: uint32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Signed guards require non negative indices.
    #[test]
    fn test_preserve_signed_negative_indices() {
        // source test
        let input = r#"
function test(v0: int32[4], v1: int32): void {
b0(v0: int32[4], v1: int32):
    v2: int32 = 1int32
    v3: int32 = 4int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.lt.s v4, v3
    branch v5, b2, b5
b2:
    v6: boolean = int.lt.s v4, v3
    check bounds.s v4, v3, v0 -> b3, b4
b3:
    v7: int32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[4], v1: int32): void {
b0(v0: int32[4], v1: int32):
    v2: int32 = 1int32
    v3: int32 = 4int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.lt.s v4, v3
    branch v5, b2, b5
b2:
    v6: boolean = int.lt.s v4, v3
    check bounds.s v4, v3, v0 -> b3, b4
b3:
    v7: int32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Forwarded loop parameters are matched against guards.
    #[test]
    fn test_eliminate_forwarded_param_checks() {
        // source test
        let input = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2(v4), b5
b2(v6: uint32):
    v7: boolean = int.lt.u v6, v3
    check bounds.u v6, v3, v0 -> b3, b4
b3:
    v8: uint32 = int.add v6, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2(v4), b5
b2(v6: uint32):
    v7: boolean = int.lt.u v6, v3
    jump b3
b3:
    v8: uint32 = int.add v6, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Signed guards with non negative preconditions remove checks.
    #[test]
    fn test_eliminate_signed_bounds_with_non_negative_guard() {
        // source test
        let input = r#"
function test(v0: int32[8], v1: int32): void {
b0(v0: int32[8], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 8int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.ge.s v4, v2
    branch v5, b2, b6
b2:
    v6: boolean = int.lt.s v4, v3
    branch v6, b3, b6
b3:
    v7: boolean = int.lt.s v4, v3
    check bounds.s v4, v3, v0 -> b4, b5
b4:
    v8: int32 = 1int32
    v9: int32 = int.add v4, v8
    jump b1(v9)
b5:
    unreachable
b6:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[8], v1: int32): void {
b0(v0: int32[8], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 8int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.ge.s v4, v2
    branch v5, b2, b6
b2:
    v6: boolean = int.lt.s v4, v3
    branch v6, b3, b6
b3:
    v7: boolean = int.lt.s v4, v3
    jump b4
b4:
    v8: int32 = 1int32
    v9: int32 = int.add v4, v8
    jump b1(v9)
b5:
    unreachable
b6:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Positive lower bounds imply non negative indices.
    #[test]
    fn test_eliminate_signed_bounds_with_positive_guard() {
        // source test
        let input = r#"
function test(v0: int32[8], v1: int32): void {
b0(v0: int32[8], v1: int32):
    v2: int32 = 2int32
    v3: int32 = 8int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.ge.s v4, v2
    branch v5, b2, b6
b2:
    v6: boolean = int.lt.s v4, v3
    branch v6, b3, b6
b3:
    v7: boolean = int.lt.s v4, v3
    check bounds.s v4, v3, v0 -> b4, b5
b4:
    v8: int32 = 1int32
    v9: int32 = int.add v4, v8
    jump b1(v9)
b5:
    unreachable
b6:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[8], v1: int32): void {
b0(v0: int32[8], v1: int32):
    v2: int32 = 2int32
    v3: int32 = 8int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.ge.s v4, v2
    branch v5, b2, b6
b2:
    v6: boolean = int.lt.s v4, v3
    branch v6, b3, b6
b3:
    v7: boolean = int.lt.s v4, v3
    jump b4
b4:
    v8: int32 = 1int32
    v9: int32 = int.add v4, v8
    jump b1(v9)
b5:
    unreachable
b6:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Flipped signed guards still imply non negative indices.
    #[test]
    fn test_eliminate_signed_bounds_with_flipped_guard() {
        // source test
        let input = r#"
function test(v0: int32[8], v1: int32): void {
b0(v0: int32[8], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 8int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.le.s v2, v4
    branch v5, b2, b6
b2:
    v6: boolean = int.lt.s v4, v3
    branch v6, b3, b6
b3:
    v7: boolean = int.lt.s v4, v3
    check bounds.s v4, v3, v0 -> b4, b5
b4:
    v8: int32 = 1int32
    v9: int32 = int.add v4, v8
    jump b1(v9)
b5:
    unreachable
b6:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[8], v1: int32): void {
b0(v0: int32[8], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 8int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.le.s v2, v4
    branch v5, b2, b6
b2:
    v6: boolean = int.lt.s v4, v3
    branch v6, b3, b6
b3:
    v7: boolean = int.lt.s v4, v3
    jump b4
b4:
    v8: int32 = 1int32
    v9: int32 = int.add v4, v8
    jump b1(v9)
b5:
    unreachable
b6:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Negated guards still eliminate bounds checks.
    #[test]
    fn test_eliminate_bounds_with_negated_guard() {
        // source test
        let input = r#"
function test(v0: uint32[4]): void {
b0(v0: uint32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    v6: boolean = int.not v5
    branch v6, b2, b3
b2:
    return
b3:
    v7: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b4, b5
b4:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b5:
    unreachable
}"#;
        // expected output
        let expected = r#"
function test(v0: uint32[4]): void {
b0(v0: uint32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    v6: boolean = int.not v5
    branch v6, b2, b3
b2:
    return
b3:
    v7: boolean = int.lt.u v4, v3
    jump b4
b4:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b5:
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Guard checks inside the loop header remove redundant checks.
    #[test]
    fn test_eliminate_bounds_guard_check() {
        // source test
        let input = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b2, b5
b2:
    v6: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b3, b4
b3:
    v7: uint32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[4]): void {
b0(v0: int32[4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b2, b5
b2:
    v6: boolean = int.lt.u v4, v3
    jump b3
b3:
    v7: uint32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Mismatched signedness guards do not remove checks.
    #[test]
    fn test_preserve_mismatched_signed_guard() {
        // source test
        let input = r#"
function test(v0: int32[4], v1: int32): void {
b0(v0: int32[4], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 4int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b5
b2:
    v6: boolean = int.lt.s v4, v3
    check bounds.s v4, v3, v0 -> b3, b4
b3:
    v7: int32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[4], v1: int32): void {
b0(v0: int32[4], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 4int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b5
b2:
    v6: boolean = int.lt.s v4, v3
    check bounds.s v4, v3, v0 -> b3, b4
b3:
    v7: int32 = int.add v4, v2
    jump b1(v7)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Guard offsets still eliminate redundant bounds checks.
    #[test]
    fn test_eliminate_guard_with_positive_offset() {
        // source test
        let input = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: uint32 = int.add v4, v2
    v6: boolean = int.lt.u v5, v3
    branch v6, b2, b5
b2:
    v7: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b3, b4
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: uint32 = int.add v4, v2
    v6: boolean = int.lt.u v5, v3
    branch v6, b2, b5
b2:
    v7: boolean = int.lt.u v4, v3
    jump b3
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Checks with larger offsets are preserved.
    #[test]
    fn test_preserve_check_with_positive_offset() {
        // source test
        let input = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b5
b2:
    v6: uint32 = int.add v4, v2
    v7: boolean = int.lt.u v6, v3
    check bounds.u v6, v3, v0 -> b3, b4
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v3
    branch v5, b2, b5
b2:
    v6: uint32 = int.add v4, v2
    v7: boolean = int.lt.u v6, v3
    check bounds.u v6, v3, v0 -> b3, b4
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Guards with smaller length offsets eliminate checks.
    #[test]
    fn test_eliminate_guard_with_length_offset() {
        // source test
        let input = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: uint32 = int.sub v3, v2
    v6: boolean = int.lt.u v4, v5
    branch v6, b2, b5
b2:
    v7: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b3, b4
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: uint32 = int.sub v3, v2
    v6: boolean = int.lt.u v4, v5
    branch v6, b2, b5
b2:
    v7: boolean = int.lt.u v4, v3
    jump b3
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Guards with larger length offsets preserve checks.
    #[test]
    fn test_preserve_guard_with_larger_length_offset() {
        // source test
        let input = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: uint32 = int.add v3, v2
    v6: boolean = int.lt.u v4, v5
    branch v6, b2, b5
b2:
    v7: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b3, b4
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: int32[8]): void {
b0(v0: int32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 8uint32
    jump b1(v1)
b1(v4: uint32):
    v5: uint32 = int.add v3, v2
    v6: boolean = int.lt.u v4, v5
    branch v6, b2, b5
b2:
    v7: boolean = int.lt.u v4, v3
    check bounds.u v4, v3, v0 -> b3, b4
b3:
    v8: uint32 = int.add v4, v2
    jump b1(v8)
b4:
    unreachable
b5:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&LoopBoundsCheckEliminate);
        test.assert_output(expected);
    }
}
