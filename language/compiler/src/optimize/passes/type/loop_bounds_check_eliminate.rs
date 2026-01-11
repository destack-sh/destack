use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, RangeAnalysis, RangeMap, ScalarEvolution,
    ValueRange,
};
use crate::optimize::common::BlockParamForwarding;
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

declare_pass! {
    /// Eliminate bounds checks dominated by loop guards.
    ///
    /// Uses loop guard comparisons to remove redundant bounds checks inside
    /// loop bodies when the guard implies the bounds check is satisfied.
    ///
    /// ```mir
    /// function @before(v0: [i32; 4]) -> void {
    /// block0(v0: [i32; 4]):
    ///     v1 = iconst 0u32
    ///     v2 = iconst 1u32
    ///     v3 = iconst 4u32
    ///     jump block1(v1)
    /// block1(v4: u32):
    ///     v5 = icmp_ult v4, v3
    ///     branch v5, block2, block3
    /// block2:
    ///     v6 = icmp_ult v4, v3
    ///     check v6, bounds.unsigned v4, v3, v0, block4, block5
    /// block4:
    ///     v7 = iadd v4, v2
    ///     jump block1(v7)
    /// block5:
    ///     unreachable
    /// block3:
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: [i32; 4]) -> void {
    /// block0(v0: [i32; 4]):
    ///     v1 = iconst 0u32
    ///     v2 = iconst 1u32
    ///     v3 = iconst 4u32
    ///     jump block1(v1)
    /// block1(v4: u32):
    ///     v5 = icmp_ult v4, v3
    ///     branch v5, block2, block3
    /// block2:
    ///     v6 = icmp_ult v4, v3
    ///     jump block4
    /// block4:
    ///     v7 = iadd v4, v2
    ///     jump block1(v7)
    /// block5:
    ///     unreachable
    /// block3:
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
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // gather analyses
        let analyses = FunctionAnalyses::new(function, tree);
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
                definitions.insert(
                    param.value,
                    ValueDefinition {
                        kind: ValueDefinitionKind::Parameter,
                    },
                );
            }

            // record instruction destinations
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
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
            let terminator = tree.get(block_id).terminator.clone();
            let (constraint, success) = match terminator {
                mir::Terminator::Check {
                    condition: _,
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

            // find a guard that implies the bounds check
            let mut guard_implies = false;
            for guard in &guards.bounds {
                // skip guards in the same block
                if guard.block == block_id {
                    continue;
                }

                // require guard dominance
                if !domtree.dominates(guard.block, block_id) {
                    continue;
                }

                // require matching signedness
                if guard.is_signed != *is_signed {
                    continue;
                }

                // require equivalent index and length values
                if !values_equivalent(*index, guard.index, loop_index, scev, forwarding)
                    || !values_equivalent(*length, guard.length, loop_index, scev, forwarding)
                {
                    continue;
                }

                // require non negative signed indices
                if *is_signed
                    && !index_non_negative(*index, block_id, ranges, forwarding)
                    && !index_non_negative_from_guards(
                        *index,
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
            replace_terminator_with_jump(tree, block_id, success.target, &success.arguments);
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

    for &block_id in &lp.exiting_blocks {
        let block = tree.get(block_id);

        // collect bounds from check terminators when available
        if let mir::Terminator::Check {
            constraint,
            success,
            failure,
            ..
        } = &block.terminator
            && let mir::CheckConstraint::Bounds {
                index,
                length,
                is_signed,
                ..
            } = constraint
        {
            let success_in_loop = lp.blocks.contains(&success.target);
            let failure_in_loop = lp.blocks.contains(&failure.target);
            if success_in_loop != failure_in_loop && success_in_loop {
                bounds.push(LoopGuard {
                    block: block_id,
                    index: *index,
                    length: *length,
                    is_signed: *is_signed,
                });

                if *is_signed {
                    non_negative.push(NonNegativeGuard {
                        block: block_id,
                        value: *index,
                    });
                }

                continue;
            }
        }

        // extract the in loop guard condition
        let Some((condition, guard_is_true)) = guard_condition(block.terminator.clone(), lp) else {
            continue;
        };

        // require an instruction definition for the condition
        let Some(definition) = definitions.definition_for(condition) else {
            continue;
        };
        let ValueDefinitionKind::Instruction { instruction } = definition.kind else {
            continue;
        };

        // require a binary comparison
        let instruction = tree.get(instruction);
        let mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } = instruction
        else {
            continue;
        };

        // record non negative guards when possible
        let ranges_at_guard = ranges.exit(block_id);
        if let Some(value) =
            guard_non_negative(*operator, *left, *right, guard_is_true, ranges_at_guard)
        {
            non_negative.push(NonNegativeGuard {
                block: block_id,
                value,
            });
        }

        // normalize to a strict less than guard
        let Some(guard_info) = guard_less_than(*operator, *left, *right, guard_is_true) else {
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
            let then_in_loop = lp.blocks.contains(&then_target);
            let else_in_loop = lp.blocks.contains(&else_target);
            if then_in_loop == else_in_loop {
                return None;
            }

            let guard_is_true = then_in_loop;
            Some((condition, guard_is_true))
        }
        mir::Terminator::Check {
            condition,
            success,
            failure,
            ..
        } => {
            let success_in_loop = lp.blocks.contains(&success.target);
            let failure_in_loop = lp.blocks.contains(&failure.target);
            if success_in_loop == failure_in_loop {
                return None;
            }

            let guard_is_true = success_in_loop;
            Some((condition, guard_is_true))
        }
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

    let bound_min = signed_range_min(bound_value, ranges)?;

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
    match operator {
        mir::BinaryOperator::SignedLessThan => Some(mir::BinaryOperator::SignedGreaterThan),
        mir::BinaryOperator::SignedLessEqual => Some(mir::BinaryOperator::SignedGreaterEqual),
        mir::BinaryOperator::SignedGreaterThan => Some(mir::BinaryOperator::SignedLessThan),
        mir::BinaryOperator::SignedGreaterEqual => Some(mir::BinaryOperator::SignedLessEqual),
        _ => None,
    }
}

/// Check if two values are equivalent within a loop.
fn values_equivalent(
    left: mir::Value,
    right: mir::Value,
    loop_index: usize,
    scev: &ScalarEvolution,
    forwarding: &BlockParamForwarding,
) -> bool {
    // resolve forwarded parameters first
    let left = forwarding.resolve(left);
    let right = forwarding.resolve(right);
    if left == right {
        return true;
    }

    // compare scalar evolution expressions
    let Some(left_scev) = scev.scev_for_value_in_loop(loop_index, left) else {
        return false;
    };
    let Some(right_scev) = scev.scev_for_value_in_loop(loop_index, right) else {
        return false;
    };

    left_scev == right_scev
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
    // match against guard values using scev equivalence
    non_negative.iter().any(|guard| {
        if guard.block == block_id {
            return false;
        }

        if !domtree.dominates(guard.block, block_id) {
            return false;
        }

        values_equivalent(value, guard.value, loop_index, scev, forwarding)
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
    let range = ranges.entry(block_id).get(value);
    let Some(ValueRange::Integer { min, is_signed, .. }) = range else {
        return false;
    };

    if !*is_signed {
        return false;
    }

    *min >= 0
}

/// Extract an integer constant from a value range.
fn signed_constant_from_range(value: mir::Value, ranges: &RangeMap) -> Option<i128> {
    let range = ranges.get(value)?;
    let constant = range.as_constant()?;
    match constant {
        mir::Constant::Int { value, .. } => Some(value as i128),
        _ => None,
    }
}

/// Extract the minimum signed bound for a value range.
fn signed_range_min(value: mir::Value, ranges: &RangeMap) -> Option<i128> {
    let range = ranges.get(value)?;
    let ValueRange::Integer { min, is_signed, .. } = range else {
        return None;
    };
    if !*is_signed {
        return None;
    }

    Some(*min)
}

/// Replace a block terminator with a jump.
fn replace_terminator_with_jump(
    tree: &mut mir::NodeTree,
    block_id: mir::LocalNodeId<mir::Block>,
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
) {
    // overwrite the terminator with a jump
    let block = tree.get_mut(block_id);
    block.terminator = mir::Terminator::Jump {
        target,
        arguments: arguments.to_vec(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Loop guard eliminates redundant bounds checks.
    #[test]
    fn test_eliminate_loop_bounds_check() {
        // source program
        let input = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    branch v5, block2, block3
block2:
    v6 = icmp_ult v4, v3
    check v6, bounds.unsigned v4, v3, v0, block4, block5
block4:
    v7 = iadd v4, v2
    jump block1(v7)
block5:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    branch v5, block2, block5
block2:
    v6 = icmp_ult v4, v3
    jump block3
block3:
    v7 = iadd v4, v2
    jump block1(v7)
block4:
    unreachable
block5:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Latch guards do not eliminate earlier checks.
    #[test]
    fn test_preserve_pre_guard_checks() {
        // source program
        let input = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    check v5, bounds.unsigned v4, v3, v0, block2, block5
block2:
    v6 = iadd v4, v2
    v7 = icmp_ult v6, v3
    branch v7, block1(v6), block3
block5:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    check v5, bounds.unsigned v4, v3, v0, block2, block3
block2:
    v6 = iadd v4, v2
    v7 = icmp_ult v6, v3
    branch v7, block1(v6), block4
block3:
    unreachable
block4:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Unsigned greater equal guards eliminate redundant checks.
    #[test]
    fn test_eliminate_ge_guard_checks() {
        // source program
        let input = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_uge v4, v3
    branch v5, block3, block2
block2:
    v6 = icmp_ult v4, v3
    check v6, bounds.unsigned v4, v3, v0, block4, block5
block4:
    v7 = iadd v4, v2
    jump block1(v7)
block5:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_uge v4, v3
    branch v5, block5, block2
block2:
    v6 = icmp_ult v4, v3
    jump block3
block3:
    v7 = iadd v4, v2
    jump block1(v7)
block4:
    unreachable
block5:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Signed guards require non negative indices.
    #[test]
    fn test_preserve_signed_negative_indices() {
        // source program
        let input = r#"function @test(v0: [i32; 4], v1: i32) -> void {
block0(v0: [i32; 4], v1: i32):
    v2 = iconst 1i32
    v3 = iconst 4i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_slt v4, v3
    branch v5, block2, block3
block2:
    v6 = icmp_slt v4, v3
    check v6, bounds.signed v4, v3, v0, block4, block5
block4:
    v7 = iadd v4, v2
    jump block1(v7)
block5:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4], v1: i32) -> void {
block0(v0: [i32; 4], v1: i32):
    v2 = iconst 1i32
    v3 = iconst 4i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_slt v4, v3
    branch v5, block2, block5
block2:
    v6 = icmp_slt v4, v3
    check v6, bounds.signed v4, v3, v0, block3, block4
block3:
    v7 = iadd v4, v2
    jump block1(v7)
block4:
    unreachable
block5:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Forwarded loop parameters are matched against guards.
    #[test]
    fn test_eliminate_forwarded_param_checks() {
        // source program
        let input = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    branch v5, block2(v4), block3
block2(v6: u32):
    v7 = icmp_ult v6, v3
    check v7, bounds.unsigned v6, v3, v0, block4, block5
block4:
    v8 = iadd v6, v2
    jump block1(v8)
block5:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    branch v5, block2(v4), block5
block2(v6: u32):
    v7 = icmp_ult v6, v3
    jump block3
block3:
    v8 = iadd v6, v2
    jump block1(v8)
block4:
    unreachable
block5:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Signed guards with non negative preconditions remove checks.
    #[test]
    fn test_eliminate_signed_bounds_with_non_negative_guard() {
        // source program
        let input = r#"function @test(v0: [i32; 8], v1: i32) -> void {
block0(v0: [i32; 8], v1: i32):
    v2 = iconst 0i32
    v3 = iconst 8i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_sge v4, v2
    branch v5, block2, block3
block2:
    v6 = icmp_slt v4, v3
    branch v6, block4, block3
block4:
    v7 = icmp_slt v4, v3
    check v7, bounds.signed v4, v3, v0, block5, block6
block5:
    v8 = iconst 1i32
    v9 = iadd v4, v8
    jump block1(v9)
block6:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 8], v1: i32) -> void {
block0(v0: [i32; 8], v1: i32):
    v2 = iconst 0i32
    v3 = iconst 8i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_sge v4, v2
    branch v5, block2, block6
block2:
    v6 = icmp_slt v4, v3
    branch v6, block3, block6
block3:
    v7 = icmp_slt v4, v3
    jump block4
block4:
    v8 = iconst 1i32
    v9 = iadd v4, v8
    jump block1(v9)
block5:
    unreachable
block6:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Positive lower bounds imply non negative indices.
    #[test]
    fn test_eliminate_signed_bounds_with_positive_guard() {
        // source program
        let input = r#"function @test(v0: [i32; 8], v1: i32) -> void {
block0(v0: [i32; 8], v1: i32):
    v2 = iconst 2i32
    v3 = iconst 8i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_sge v4, v2
    branch v5, block2, block3
block2:
    v6 = icmp_slt v4, v3
    branch v6, block4, block3
block4:
    v7 = icmp_slt v4, v3
    check v7, bounds.signed v4, v3, v0, block5, block6
block5:
    v8 = iconst 1i32
    v9 = iadd v4, v8
    jump block1(v9)
block6:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 8], v1: i32) -> void {
block0(v0: [i32; 8], v1: i32):
    v2 = iconst 2i32
    v3 = iconst 8i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_sge v4, v2
    branch v5, block2, block6
block2:
    v6 = icmp_slt v4, v3
    branch v6, block3, block6
block3:
    v7 = icmp_slt v4, v3
    jump block4
block4:
    v8 = iconst 1i32
    v9 = iadd v4, v8
    jump block1(v9)
block5:
    unreachable
block6:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Flipped signed guards still imply non negative indices.
    #[test]
    fn test_eliminate_signed_bounds_with_flipped_guard() {
        // source program
        let input = r#"function @test(v0: [i32; 8], v1: i32) -> void {
block0(v0: [i32; 8], v1: i32):
    v2 = iconst 0i32
    v3 = iconst 8i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_sle v2, v4
    branch v5, block2, block3
block2:
    v6 = icmp_slt v4, v3
    branch v6, block4, block3
block4:
    v7 = icmp_slt v4, v3
    check v7, bounds.signed v4, v3, v0, block5, block6
block5:
    v8 = iconst 1i32
    v9 = iadd v4, v8
    jump block1(v9)
block6:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 8], v1: i32) -> void {
block0(v0: [i32; 8], v1: i32):
    v2 = iconst 0i32
    v3 = iconst 8i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_sle v2, v4
    branch v5, block2, block6
block2:
    v6 = icmp_slt v4, v3
    branch v6, block3, block6
block3:
    v7 = icmp_slt v4, v3
    jump block4
block4:
    v8 = iconst 1i32
    v9 = iadd v4, v8
    jump block1(v9)
block5:
    unreachable
block6:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Guard checks inside the loop header remove redundant checks.
    #[test]
    fn test_eliminate_bounds_guard_check() {
        // source program
        let input = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    check v5, bounds.unsigned v4, v3, v0, block2, block3
block2:
    v6 = icmp_ult v4, v3
    check v6, bounds.unsigned v4, v3, v0, block4, block5
block4:
    v7 = iadd v4, v2
    jump block1(v7)
block5:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1 = iconst 0u32
    v2 = iconst 1u32
    v3 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5 = icmp_ult v4, v3
    check v5, bounds.unsigned v4, v3, v0, block2, block5
block2:
    v6 = icmp_ult v4, v3
    jump block3
block3:
    v7 = iadd v4, v2
    jump block1(v7)
block4:
    unreachable
block5:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }

    /// Mismatched signedness guards do not remove checks.
    #[test]
    fn test_preserve_mismatched_signed_guard() {
        // source program
        let input = r#"function @test(v0: [i32; 4], v1: i32) -> void {
block0(v0: [i32; 4], v1: i32):
    v2 = iconst 0i32
    v3 = iconst 4i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_ult v4, v3
    branch v5, block2, block3
block2:
    v6 = icmp_slt v4, v3
    check v6, bounds.signed v4, v3, v0, block4, block5
block4:
    v7 = iadd v4, v2
    jump block1(v7)
block5:
    unreachable
block3:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4], v1: i32) -> void {
block0(v0: [i32; 4], v1: i32):
    v2 = iconst 0i32
    v3 = iconst 4i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_ult v4, v3
    branch v5, block2, block5
block2:
    v6 = icmp_slt v4, v3
    check v6, bounds.signed v4, v3, v0, block3, block4
block3:
    v7 = iadd v4, v2
    jump block1(v7)
block4:
    unreachable
block5:
    return
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&LoopBoundsCheckEliminate);
        program.assert_output(expected);
    }
}
