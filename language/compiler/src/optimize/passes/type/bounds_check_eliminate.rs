use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{
    ConstantPropagation, ControlFlowGraph, DominatorTree, RangeAnalysis, RangeMap, ValueRange,
};
use crate::common::mir::{
    BlockParamForwarding, constraint_truth_value, evaluate_integer_range_comparison, fold_binary,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Eliminate bounds checks that are proven redundant.
    ///
    /// Uses range analysis, dominator based constraints, and assume metadata
    /// to remove checks that are guaranteed to succeed.
    ///
    /// ```mir
    /// function before(v0: [int32; 4]): void {
    /// b0(v0: [int32; 4]):
    ///     v1 = 2uint32
    ///     v2 = 4uint32
    ///     v3 = int.lt.u v1, v2
    ///     check bounds.u v1, v2, v0 -> b1, b2
    /// b1:
    ///     v4 = int.lt.u v1, v2
    ///     check bounds.u v1, v2, v0 -> b3, b2
    /// b3:
    ///     return
    /// b2:
    ///     unreachable
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: [int32; 4]): void {
    /// b0(v0: [int32; 4]):
    ///     v1 = 2uint32
    ///     v2 = 4uint32
    ///     v3 = int.lt.u v1, v2
    ///     check bounds.u v1, v2, v0 -> b1, b2
    /// b1:
    ///     v4 = int.lt.u v1, v2
    ///     jump b3
    /// b3:
    ///     return
    /// b2:
    ///     unreachable
    /// }
    /// ```
    #[pass(id = "bounds-check-eliminate")]
    pub BoundsCheckEliminate,
    "Eliminate redundant bounds checks"
}

impl FunctionPass for BoundsCheckEliminate {
    /// Run the bounds check elimination pass.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        let Some(entry) = function.entry else {
            return AnalysisPreservation::all();
        };

        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let constants = analyses.get::<ConstantPropagation>();
        let cfg = analyses.get::<ControlFlowGraph>();
        let ranges = analyses.get::<RangeAnalysis>();
        let domtree = analyses.get::<DominatorTree>();

        // build value definition metadata
        let definitions = ValueDefinitions::build(function, tree);

        // build block parameter forwarding
        let forwarding = BlockParamForwarding::build(function, tree, &cfg);

        // build block constraint maps
        let block_constraints = build_block_constraints(
            entry,
            function,
            tree,
            &definitions,
            constants.as_ref(),
            ranges.as_ref(),
            domtree.as_ref(),
        );

        // scan blocks for removable checks
        let mut changed = false;
        for &block_id in &function.blocks {
            // identify bounds check candidates
            let Some(candidate) = bounds_check_candidate(block_id, tree) else {
                continue;
            };

            // handle constant condition outcomes
            let block_ranges = ranges.exit(block_id);
            let condition_truth = candidate.condition.and_then(|condition| {
                condition_truth_value(condition, block_ranges, &definitions, tree)
            });
            let constraint_truth = candidate
                .constraint
                .as_ref()
                .and_then(|constraint| constraint_truth_value(constraint, block_ranges));
            let check_truth = constraint_truth.or(condition_truth);

            // replace checks when the outcome is constant
            if let Some(truth_value) = check_truth {
                // rewrite to the always taken edge
                let target = if truth_value == candidate.in_bounds_truth {
                    candidate.in_bounds_target.clone()
                } else {
                    candidate.out_of_bounds_target.clone()
                };
                replace_terminator_with_jump(tree, block_id, target);
                changed = true;
                continue;
            }

            // collect constraints describing the in bounds path
            let mut required_constraints = Vec::new();
            if let Some(constraint) = candidate.constraint.as_ref() {
                let mut constraints = constraints_for_check_kind(
                    constraint,
                    candidate.in_bounds_truth,
                    block_id,
                    &definitions,
                    tree,
                    constants.as_ref(),
                    block_ranges,
                )
                .unwrap_or_default();
                required_constraints.append(&mut constraints);
            }

            // collect constraints from the condition expression
            if let Some(condition) = candidate.condition {
                let mut condition_constraints = constraints_for_condition(
                    condition,
                    candidate.in_bounds_truth,
                    block_id,
                    &definitions,
                    tree,
                    constants.as_ref(),
                    block_ranges,
                )
                .unwrap_or_default();
                required_constraints.append(&mut condition_constraints);
            }

            // skip when no constraints were derived
            if required_constraints.is_empty() {
                continue;
            }

            // check whether dominated constraints already imply the bounds
            let known_constraints = block_constraints.exit(block_id);
            let constraints_imply = constraints_imply_requirements(
                known_constraints,
                &required_constraints,
                block_ranges,
                &forwarding,
            );

            // rewrite the check when constraints imply the bounds
            if constraints_imply {
                replace_terminator_with_jump(tree, block_id, candidate.in_bounds_target.clone());
                changed = true;
            }
        }

        // return preservation based on whether we rewrote any checks
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "BoundsCheckEliminate"
    }

    /// Return the stable id for this pass.
    fn id(&self) -> &'static str {
        "bounds-check-eliminate"
    }
}

/// Definition kind for an SSA value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueDefinition {
    /// Block parameter definition.
    Parameter {
        /// Block that owns the parameter.
        block: mir::LocalNodeId<mir::Block>,
        /// Parameter index within the block.
        index: usize,
    },
    /// Instruction definition.
    Instruction {
        /// Instruction id that defines the value.
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

/// Definition map for SSA values.
#[derive(Debug)]
struct ValueDefinitions {
    /// Mapping from SSA value to its definition.
    definitions: HashMap<mir::Value, ValueDefinition>,
}

impl ValueDefinitions {
    /// Build the definition map for a function.
    fn build(function: &mir::Function, tree: &mir::Tree) -> Self {
        // seed value definitions from parameters and instructions
        let mut definitions = HashMap::new();
        for &block_id in &function.blocks {
            // record block parameter definitions
            let block = tree.get(block_id);
            for (index, param) in block.parameters.iter().enumerate() {
                let Some(value) = param.value.value() else {
                    continue;
                };

                definitions.insert(
                    value,
                    ValueDefinition::Parameter {
                        block: block_id,
                        index,
                    },
                );
            }

            // record instruction definitions
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination().and_then(|value| value.value())
                {
                    definitions.insert(
                        destination,
                        ValueDefinition::Instruction {
                            instruction: instruction_id,
                        },
                    );
                }
            }
        }

        Self { definitions }
    }

    /// Return the definition for an SSA value.
    fn get(&self, value: mir::Value) -> Option<ValueDefinition> {
        self.definitions.get(&value).copied()
    }
}

/// Key for a constraint operand.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BoundKey {
    /// Reference a dynamic SSA value.
    Value(mir::Value),
    /// Reference a constant value.
    Constant(ConstantKey),
}

/// Canonical constant representation for constraints.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConstantKey {
    /// Signed integer constant.
    Int {
        /// Integer value.
        value: i128,
        /// Bit width for the integer.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Unsigned integer constant.
    UInt {
        /// Unsigned value.
        value: u128,
        /// Bit width for the integer.
        width: u16,
    },
    /// Boolean constant.
    Boolean(bool),
}

/// Directional relation for constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ConstraintRelation {
    /// Value is strictly less than the bound.
    UpperExclusive,
    /// Value is less than or equal to the bound.
    UpperInclusive,
    /// Value is strictly greater than the bound.
    LowerExclusive,
    /// Value is greater than or equal to the bound.
    LowerInclusive,
}

/// Relational constraint between two operands.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BoundsConstraint {
    /// Constrained value.
    value: BoundKey,
    /// Bound value.
    bound: BoundKey,
    /// Relation between the value and the bound.
    relation: ConstraintRelation,
    /// Whether the comparison is signed.
    is_signed: bool,
}

impl BoundsConstraint {
    /// Return a variant with value and bound swapped.
    fn flipped(&self) -> Self {
        // map the relation for the flipped orientation
        let relation = match self.relation {
            ConstraintRelation::UpperExclusive => ConstraintRelation::LowerExclusive,
            ConstraintRelation::UpperInclusive => ConstraintRelation::LowerInclusive,
            ConstraintRelation::LowerExclusive => ConstraintRelation::UpperExclusive,
            ConstraintRelation::LowerInclusive => ConstraintRelation::UpperInclusive,
        };

        // build the flipped constraint
        Self {
            value: self.bound.clone(),
            bound: self.value.clone(),
            relation,
            is_signed: self.is_signed,
        }
    }
}

/// Constraints available at block entry and exit.
#[derive(Debug, Default)]
struct BlockConstraints {
    /// Constraints available at block exit.
    exit: HashMap<mir::LocalNodeId<mir::Block>, Vec<BoundsConstraint>>,
}

impl BlockConstraints {
    /// Return constraints for a block exit.
    fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> &[BoundsConstraint] {
        self.exit.get(&block).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Cached reachability for control flow queries.
#[derive(Debug, Default)]
struct ReachabilityCache {
    /// Reachable blocks keyed by start block.
    reachable: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Block>>>,
}

impl ReachabilityCache {
    /// Return true when target is reachable from the start block.
    fn can_reach(
        &mut self,
        tree: &mir::Tree,
        start: mir::LocalNodeId<mir::Block>,
        target: mir::LocalNodeId<mir::Block>,
    ) -> bool {
        // handle the trivial case
        if start == target {
            return true;
        }

        // compute reachability for this start block
        let entry = self.reachable.entry(start).or_insert_with(|| {
            // collect reachable blocks with a depth first walk
            let mut visited = HashSet::new();
            let mut stack = vec![start];
            while let Some(block_id) = stack.pop() {
                // skip blocks that are already visited
                if !visited.insert(block_id) {
                    continue;
                }

                // enqueue successors for traversal
                let block = tree.get(block_id);
                let terminator = tree.get(block.terminator);
                for successor in terminator.successors() {
                    let Some(successor) = successor.block() else {
                        continue;
                    };

                    if visited.contains(&successor) {
                        continue;
                    }
                    stack.push(successor);
                }
            }

            visited
        });

        entry.contains(&target)
    }
}

/// Bounds check candidate extracted from a terminator.
#[derive(Debug, Clone)]
struct BoundsCheckCandidate {
    /// Condition value being checked.
    condition: Option<mir::Value>,
    /// Target when bounds are satisfied.
    in_bounds_target: mir::BlockTarget,
    /// Target when bounds fail.
    out_of_bounds_target: mir::BlockTarget,
    /// Whether condition truth implies in bounds.
    in_bounds_truth: bool,
    /// Optional structured constraint for checks.
    constraint: Option<mir::CheckConstraint>,
}

/// Return whether a block is a dedicated trap block.
fn is_trap_block(block_id: mir::LocalNodeId<mir::Block>, tree: &mir::Tree) -> bool {
    // only accept blocks that end in one fatal trap
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);
    block.instructions.is_empty() && matches!(terminator, mir::Terminator::Trap { .. })
}

/// Replace a block's terminator with a jump.
fn replace_terminator_with_jump(
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    target: mir::BlockTarget,
) {
    // overwrite the terminator with a jump
    let block = tree.get(block_id);
    let terminator = mir::Terminator::Jump { target };
    tree.replace(block.terminator, terminator);
}

/// Extract a bounds check candidate from a block terminator.
fn bounds_check_candidate(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
) -> Option<BoundsCheckCandidate> {
    // inspect the terminator for check patterns
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator).clone();
    match terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            // consider only branches to trap blocks
            let then_trap = then_target
                .block
                .block()
                .is_some_and(|block| is_trap_block(block, tree));
            let else_trap = else_target
                .block
                .block()
                .is_some_and(|block| is_trap_block(block, tree));
            if !then_trap && !else_trap {
                return None;
            }

            // assign in bounds and out of bounds edges
            let Some(condition) = condition.value() else {
                return None;
            };
            let (in_bounds_target, out_target, in_bounds_truth) = if then_trap {
                (else_target, then_target, false)
            } else {
                (then_target, else_target, true)
            };
            Some(BoundsCheckCandidate {
                condition: Some(condition),
                in_bounds_target,
                out_of_bounds_target: out_target,
                in_bounds_truth,
                constraint: None,
            })
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            // only handle bounds checks
            if !matches!(constraint, mir::CheckConstraint::Bounds { .. }) {
                return None;
            }
            Some(BoundsCheckCandidate {
                condition: None,
                in_bounds_target: success,
                out_of_bounds_target: failure,
                in_bounds_truth: true,
                constraint: Some(constraint),
            })
        }
        _ => None,
    }
}

/// Build block entry and exit constraints using dominance.
fn build_block_constraints(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mir::Tree,
    definitions: &ValueDefinitions,
    constants: &ConstantPropagation,
    ranges: &RangeAnalysis,
    domtree: &DominatorTree,
) -> BlockConstraints {
    // create dominator tree children map
    let mut children: HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> =
        HashMap::new();
    for &block in &function.blocks {
        children.insert(block, Vec::new());
    }

    // link each block to its immediate dominator
    for &block in &function.blocks {
        if let Some(idom) = domtree.immediate_dominator(block) {
            let block_children = children.get_mut(&idom).expect("missing dom child");
            block_children.push(block);
        }
    }

    // cache reachability between branch targets and dominated blocks
    let mut reachability = ReachabilityCache::default();

    // walk the dominator tree and accumulate constraints
    let mut constraints = BlockConstraints::default();
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    stack.push((entry, Vec::new()));
    while let Some((block_id, entry_constraints)) = stack.pop() {
        // skip already visited blocks
        if !visited.insert(block_id) {
            continue;
        }

        // extend with assume constraints in the block
        let block_ranges = ranges.exit(block_id);
        let mut exit_constraints = entry_constraints;
        let mut assume_constraints =
            constraints_from_assumes(block_id, tree, definitions, constants, block_ranges);
        exit_constraints.append(&mut assume_constraints);
        constraints.exit.insert(block_id, exit_constraints.clone());

        // enqueue dominated blocks with edge constraints
        let Some(block_children) = children.get(&block_id) else {
            continue;
        };

        // propagate constraints into dominated children
        for &child in block_children {
            let mut child_constraints = exit_constraints.clone();
            let mut edge_constraints = constraints_for_edge(
                block_id,
                child,
                tree,
                definitions,
                constants,
                block_ranges,
                &mut reachability,
            );
            child_constraints.append(&mut edge_constraints);
            stack.push((child, child_constraints));
        }
    }

    constraints
}

/// Collect constraints from assume instructions in a block.
fn constraints_from_assumes(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    definitions: &ValueDefinitions,
    constants: &ConstantPropagation,
    ranges: &RangeMap,
) -> Vec<BoundsConstraint> {
    // scan instructions for assume operations
    let block = tree.get(block_id);
    let mut constraints = Vec::new();
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        let mir::Instruction::Assume { condition } = instruction else {
            continue;
        };
        let Some(condition) = condition.value() else {
            continue;
        };

        let mut assume_constraints = constraints_for_condition(
            condition,
            true,
            block_id,
            definitions,
            tree,
            constants,
            ranges,
        )
        .unwrap_or_default();
        constraints.append(&mut assume_constraints);
    }

    constraints
}

/// Collect constraints from a terminator edge.
fn constraints_for_edge(
    block_id: mir::LocalNodeId<mir::Block>,
    child: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    definitions: &ValueDefinitions,
    constants: &ConstantPropagation,
    ranges: &RangeMap,
    reachability: &mut ReachabilityCache,
) -> Vec<BoundsConstraint> {
    // derive truth value for this edge
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);
    match terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
            ..
        } => {
            let Some(condition) = condition.value() else {
                return Vec::new();
            };
            let Some(then_target) = then_target.block.block() else {
                return Vec::new();
            };
            let Some(else_target) = else_target.block.block() else {
                return Vec::new();
            };

            // check reachability for each successor
            let then_reaches = reachability.can_reach(tree, then_target, child);
            let else_reaches = reachability.can_reach(tree, else_target, child);

            // skip when both paths can reach the child
            if then_reaches == else_reaches {
                return Vec::new();
            }

            // select the path that reaches the child
            let truth_value = then_reaches;
            constraints_for_condition(
                condition,
                truth_value,
                block_id,
                definitions,
                tree,
                constants,
                ranges,
            )
            .unwrap_or_default()
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let Some(success) = success.block.block() else {
                return Vec::new();
            };
            let Some(failure) = failure.block.block() else {
                return Vec::new();
            };

            // check reachability for each successor
            let success_reaches = reachability.can_reach(tree, success, child);
            let failure_reaches = reachability.can_reach(tree, failure, child);

            // skip when both paths can reach the child
            if success_reaches == failure_reaches {
                return Vec::new();
            }

            // select the path that reaches the child
            let truth_value = success_reaches;
            let constraints = constraints_for_check_kind(
                constraint,
                truth_value,
                block_id,
                definitions,
                tree,
                constants,
                ranges,
            )
            .unwrap_or_default();
            constraints
        }
        _ => Vec::new(),
    }
}

/// Determine if constraints imply all required bounds.
fn constraints_imply_requirements(
    known: &[BoundsConstraint],
    required: &[BoundsConstraint],
    ranges: &RangeMap,
    forwarding: &BlockParamForwarding,
) -> bool {
    // normalize known constraints once for this block
    let normalized_known: Vec<BoundsConstraint> = known
        .iter()
        .map(|constraint| normalize_constraint(constraint, ranges, forwarding))
        .collect();

    // ensure every required constraint is satisfied
    required.iter().all(|required_constraint| {
        let required_constraint = normalize_constraint(required_constraint, ranges, forwarding);
        let implied_by_known = normalized_known
            .iter()
            .any(|known_constraint| constraint_implies(known_constraint, &required_constraint));

        implied_by_known || constraint_implied_by_ranges(&required_constraint, ranges)
    })
}

/// Check whether an existing constraint implies a required constraint.
fn constraint_implies(existing: &BoundsConstraint, required: &BoundsConstraint) -> bool {
    // reject signedness mismatches
    if existing.is_signed != required.is_signed {
        return false;
    }

    // try direct match
    if constraint_implies_direct(existing, required) {
        return true;
    }

    // try flipped orientation
    let flipped = existing.flipped();
    constraint_implies_direct(&flipped, required)
}

/// Normalize constraint bounds using range constants and forwarding.
fn normalize_constraint(
    constraint: &BoundsConstraint,
    ranges: &RangeMap,
    forwarding: &BlockParamForwarding,
) -> BoundsConstraint {
    // normalize the value side
    let value = normalize_bound_key(&constraint.value, ranges, forwarding);

    // normalize the bound side
    let bound = normalize_bound_key(&constraint.bound, ranges, forwarding);
    BoundsConstraint {
        value,
        bound,
        relation: constraint.relation,
        is_signed: constraint.is_signed,
    }
}

/// Normalize a bound key using constant range information.
fn normalize_bound_key(
    bound: &BoundKey,
    ranges: &RangeMap,
    forwarding: &BlockParamForwarding,
) -> BoundKey {
    // resolve value and constant bounds into canonical keys
    match bound {
        BoundKey::Value(value) => {
            // canonicalize the value and check for constants
            let canonical = forwarding.resolve(*value);
            let Some(range) = ranges.get(canonical) else {
                return BoundKey::Value(canonical);
            };

            // fold constant ranges into bound keys
            let Some(constant) = range.as_constant() else {
                return BoundKey::Value(canonical);
            };

            // map the constant into a normalized key
            let Some(constant_key) = constant_key_from_constant(&constant) else {
                return BoundKey::Value(canonical);
            };
            BoundKey::Constant(constant_key)
        }
        // preserve existing constants
        BoundKey::Constant(_) => bound.clone(),
    }
}

/// Check whether a constraint directly implies another with matching operands.
fn constraint_implies_direct(existing: &BoundsConstraint, required: &BoundsConstraint) -> bool {
    // require matching operands
    if existing.value != required.value {
        return false;
    }

    // handle identical bounds
    if existing.bound == required.bound {
        return matches!(
            (existing.relation, required.relation),
            (
                ConstraintRelation::UpperExclusive,
                ConstraintRelation::UpperExclusive
            ) | (
                ConstraintRelation::UpperExclusive,
                ConstraintRelation::UpperInclusive
            ) | (
                ConstraintRelation::UpperInclusive,
                ConstraintRelation::UpperInclusive
            ) | (
                ConstraintRelation::LowerExclusive,
                ConstraintRelation::LowerExclusive
            ) | (
                ConstraintRelation::LowerExclusive,
                ConstraintRelation::LowerInclusive
            ) | (
                ConstraintRelation::LowerInclusive,
                ConstraintRelation::LowerInclusive
            )
        );
    }

    // compare constant bounds when possible
    let (BoundKey::Constant(existing_bound), BoundKey::Constant(required_bound)) =
        (&existing.bound, &required.bound)
    else {
        return false;
    };
    match (existing.relation, required.relation) {
        (ConstraintRelation::UpperExclusive, ConstraintRelation::UpperExclusive) => {
            compare_upper_bounds(existing_bound, required_bound, existing.is_signed, false)
        }
        (ConstraintRelation::UpperExclusive, ConstraintRelation::UpperInclusive) => {
            compare_upper_bounds(existing_bound, required_bound, existing.is_signed, false)
        }
        (ConstraintRelation::UpperInclusive, ConstraintRelation::UpperInclusive) => {
            compare_upper_bounds(existing_bound, required_bound, existing.is_signed, false)
        }
        (ConstraintRelation::UpperInclusive, ConstraintRelation::UpperExclusive) => {
            compare_upper_bounds(existing_bound, required_bound, existing.is_signed, true)
        }
        (ConstraintRelation::LowerExclusive, ConstraintRelation::LowerExclusive) => {
            compare_lower_bounds(existing_bound, required_bound, existing.is_signed, false)
        }
        (ConstraintRelation::LowerExclusive, ConstraintRelation::LowerInclusive) => {
            compare_lower_bounds(existing_bound, required_bound, existing.is_signed, false)
        }
        (ConstraintRelation::LowerInclusive, ConstraintRelation::LowerInclusive) => {
            compare_lower_bounds(existing_bound, required_bound, existing.is_signed, false)
        }
        (ConstraintRelation::LowerInclusive, ConstraintRelation::LowerExclusive) => {
            compare_lower_bounds(existing_bound, required_bound, existing.is_signed, true)
        }
        _ => false,
    }
}

/// Compare upper bound constants for implication.
fn compare_upper_bounds(
    existing: &ConstantKey,
    required: &ConstantKey,
    is_signed: bool,
    require_strict: bool,
) -> bool {
    // compare numeric representations
    let Some((existing_value, required_value)) = compare_constants(existing, required, is_signed)
    else {
        return false;
    };
    if require_strict {
        existing_value < required_value
    } else {
        existing_value <= required_value
    }
}

/// Compare lower bound constants for implication.
fn compare_lower_bounds(
    existing: &ConstantKey,
    required: &ConstantKey,
    is_signed: bool,
    require_strict: bool,
) -> bool {
    // compare numeric representations
    let Some((existing_value, required_value)) = compare_constants(existing, required, is_signed)
    else {
        return false;
    };
    if require_strict {
        existing_value > required_value
    } else {
        existing_value >= required_value
    }
}

/// Convert constant keys into signed numeric values for comparison.
fn compare_constants(
    existing: &ConstantKey,
    required: &ConstantKey,
    is_signed: bool,
) -> Option<(i128, i128)> {
    // choose the comparison domain based on signedness
    if is_signed {
        // normalize both sides to signed values
        let existing_value = constant_key_as_signed(existing)?;
        let required_value = constant_key_as_signed(required)?;
        Some((existing_value, required_value))
    } else {
        // normalize both sides to unsigned values
        let existing_value = constant_key_as_unsigned(existing)?;
        let required_value = constant_key_as_unsigned(required)?;
        if existing_value > i128::MAX as u128 || required_value > i128::MAX as u128 {
            return None;
        }

        // narrow to a signed range for consistent comparisons
        Some((existing_value as i128, required_value as i128))
    }
}

/// Convert a constant key into a signed value.
fn constant_key_as_signed(constant: &ConstantKey) -> Option<i128> {
    // map constant variants into signed values when possible
    match constant {
        ConstantKey::Int {
            value,
            is_signed: true,
            ..
        } => Some(*value),
        ConstantKey::UInt { value, .. } => i128::try_from(*value).ok(),
        _ => None,
    }
}

/// Convert a constant key into an unsigned value.
fn constant_key_as_unsigned(constant: &ConstantKey) -> Option<u128> {
    // map constant variants into unsigned values when possible
    match constant {
        ConstantKey::UInt { value, .. } => Some(*value),
        ConstantKey::Int { value, .. } => u128::try_from(*value).ok(),
        _ => None,
    }
}

/// Check whether range analysis implies a constraint.
fn constraint_implied_by_ranges(constraint: &BoundsConstraint, ranges: &RangeMap) -> bool {
    // extract integer ranges for the value and bound
    let Some(value_range) = integer_range_for_bound(&constraint.value, ranges) else {
        return false;
    };
    let Some(bound_range) = integer_range_for_bound(&constraint.bound, ranges) else {
        return false;
    };

    // require compatible signedness
    if value_range.is_signed != constraint.is_signed
        || bound_range.is_signed != constraint.is_signed
    {
        return false;
    }

    // require compatible widths
    if value_range.width != bound_range.width {
        return false;
    }

    // compare integer bounds
    match constraint.relation {
        ConstraintRelation::UpperExclusive => value_range.max < bound_range.min,
        ConstraintRelation::UpperInclusive => value_range.max <= bound_range.min,
        ConstraintRelation::LowerExclusive => value_range.min > bound_range.max,
        ConstraintRelation::LowerInclusive => value_range.min >= bound_range.max,
    }
}

/// Integer range snapshot for a bound key.
#[derive(Debug, Clone, Copy)]
struct IntegerRangeSnapshot {
    /// Minimum possible value.
    min: i128,
    /// Maximum possible value.
    max: i128,
    /// Bit width for the integer.
    width: u16,
    /// Whether the integer is signed.
    is_signed: bool,
}

/// Convert a bound key into an integer range snapshot.
fn integer_range_for_bound(bound: &BoundKey, ranges: &RangeMap) -> Option<IntegerRangeSnapshot> {
    // resolve range information for values or constants
    match bound {
        BoundKey::Value(value) => {
            // extract integer range details from range analysis
            let range = ranges.get(*value)?;
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
        // map constants into integer ranges when possible
        BoundKey::Constant(constant) => constant_integer_range(constant),
    }
}

/// Convert a constant key into an integer range snapshot.
fn constant_integer_range(constant: &ConstantKey) -> Option<IntegerRangeSnapshot> {
    // map constant variants into integer ranges
    match constant {
        ConstantKey::Int {
            value,
            width,
            is_signed,
        } => Some(IntegerRangeSnapshot {
            min: *value,
            max: *value,
            width: *width,
            is_signed: *is_signed,
        }),
        ConstantKey::UInt { value, width } => {
            let value = i128::try_from(*value).ok()?;
            Some(IntegerRangeSnapshot {
                min: value,
                max: value,
                width: *width,
                is_signed: false,
            })
        }
        ConstantKey::Boolean(_) => None,
    }
}

/// Evaluate a condition when range analysis proves a constant truth value.
fn condition_truth_value(
    condition: mir::Value,
    ranges: &RangeMap,
    definitions: &ValueDefinitions,
    tree: &mir::Tree,
) -> Option<bool> {
    // check range analysis for constant booleans
    if let Some(ValueRange::Boolean {
        can_be_true,
        can_be_false,
    }) = ranges.get(condition)
    {
        return match (*can_be_true, *can_be_false) {
            (true, false) => Some(true),
            (false, true) => Some(false),
            _ => None,
        };
    }

    // attempt to evaluate derived boolean expressions
    let definition = definitions.get(condition)?;
    match definition {
        ValueDefinition::Instruction { instruction } => {
            let instruction = tree.get(instruction);
            match instruction {
                mir::Instruction::Binary {
                    operator,
                    left,
                    right,
                    ..
                } => match operator {
                    mir::BinaryOperator::And => {
                        let left = left.value()?;
                        let right = right.value()?;
                        let left_value = condition_truth_value(left, ranges, definitions, tree)?;
                        let right_value = condition_truth_value(right, ranges, definitions, tree)?;
                        Some(left_value && right_value)
                    }
                    mir::BinaryOperator::Or => {
                        let left = left.value()?;
                        let right = right.value()?;
                        let left_value = condition_truth_value(left, ranges, definitions, tree)?;
                        let right_value = condition_truth_value(right, ranges, definitions, tree)?;
                        Some(left_value || right_value)
                    }
                    _ => evaluate_comparison(*operator, left.value()?, right.value()?, ranges),
                },
                mir::Instruction::Unary {
                    operator: mir::UnaryOperator::Not,
                    argument,
                    ..
                } => {
                    condition_truth_value(argument.value()?, ranges, definitions, tree).map(|v| !v)
                }
                _ => None,
            }
        }
        ValueDefinition::Parameter { .. } => None,
    }
}

/// Evaluate a comparison when range analysis proves a constant result.
fn evaluate_comparison(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    ranges: &RangeMap,
) -> Option<bool> {
    // compare constant ranges when possible
    let left_range = ranges.get(left)?;
    let right_range = ranges.get(right)?;
    if let (Some(left_const), Some(right_const)) =
        (left_range.as_constant(), right_range.as_constant())
    {
        let result = fold_binary(operator, left_const, right_const)?;
        return match result {
            mir::Constant::Boolean { value } => Some(value),
            _ => None,
        };
    }

    // fallback to integer range comparisons
    match operator {
        mir::BinaryOperator::SignedLessThan
        | mir::BinaryOperator::SignedLessEqual
        | mir::BinaryOperator::SignedGreaterThan
        | mir::BinaryOperator::SignedGreaterEqual
        | mir::BinaryOperator::UnsignedLessThan
        | mir::BinaryOperator::UnsignedLessEqual
        | mir::BinaryOperator::UnsignedGreaterThan
        | mir::BinaryOperator::UnsignedGreaterEqual
        | mir::BinaryOperator::Equal
        | mir::BinaryOperator::NotEqual => {
            evaluate_integer_range_comparison(operator, left_range, right_range)
        }
        _ => None,
    }
}

/// Build constraints for a check kind.
fn constraints_for_check_kind(
    kind: &mir::CheckConstraint,
    truth_value: bool,
    block_id: mir::LocalNodeId<mir::Block>,
    definitions: &ValueDefinitions,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
    ranges: &RangeMap,
) -> Option<Vec<BoundsConstraint>> {
    // only bounds constraints are handled here
    let mir::CheckConstraint::Bounds {
        index,
        length,
        is_signed,
        ..
    } = kind
    else {
        return None;
    };
    let index = index.value()?;
    let length = length.value()?;

    // resolve bound keys and initialize constraints
    let index_key = bound_key_for_value(index, block_id, definitions, tree, constants, ranges);
    let length_key = bound_key_for_value(length, block_id, definitions, tree, constants, ranges);
    let mut constraints = Vec::new();

    // derive constraints based on the check outcome
    if truth_value {
        // index < length for in bounds
        constraints.push(BoundsConstraint {
            value: index_key.clone(),
            bound: length_key,
            relation: ConstraintRelation::UpperExclusive,
            is_signed: *is_signed,
        });

        // index >= 0 for signed in bounds
        if *is_signed && let Some(zero_bound) = zero_bound_key_for_value(index, ranges, *is_signed)
        {
            constraints.push(BoundsConstraint {
                value: index_key,
                bound: zero_bound,
                relation: ConstraintRelation::LowerInclusive,
                is_signed: *is_signed,
            });
        }
    } else if !*is_signed {
        // unsigned failure implies index >= length
        constraints.push(BoundsConstraint {
            value: index_key,
            bound: length_key,
            relation: ConstraintRelation::LowerInclusive,
            is_signed: *is_signed,
        });
    }

    Some(constraints)
}

/// Build constraints for a boolean condition.
fn constraints_for_condition(
    condition: mir::Value,
    truth_value: bool,
    block_id: mir::LocalNodeId<mir::Block>,
    definitions: &ValueDefinitions,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
    ranges: &RangeMap,
) -> Option<Vec<BoundsConstraint>> {
    // resolve the condition definition
    let definition = definitions.get(condition)?;

    // derive constraints based on the definition kind
    match definition {
        ValueDefinition::Instruction { instruction } => {
            let instruction = tree.get(instruction);
            match instruction {
                mir::Instruction::Binary {
                    operator,
                    left,
                    right,
                    ..
                } => match operator {
                    mir::BinaryOperator::And => {
                        // require true to accumulate and constraints
                        if !truth_value {
                            return None;
                        }

                        // collect constraints from both sides
                        let mut left_constraints = constraints_for_condition(
                            left.value()?,
                            true,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        )
                        .unwrap_or_default();
                        let mut right_constraints = constraints_for_condition(
                            right.value()?,
                            true,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        )
                        .unwrap_or_default();

                        // merge constraints into a single set
                        left_constraints.append(&mut right_constraints);
                        Some(left_constraints)
                    }
                    mir::BinaryOperator::Or => {
                        // require false to accumulate or constraints
                        if truth_value {
                            return None;
                        }

                        // collect constraints from both sides
                        let mut left_constraints = constraints_for_condition(
                            left.value()?,
                            false,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        )
                        .unwrap_or_default();
                        let mut right_constraints = constraints_for_condition(
                            right.value()?,
                            false,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        )
                        .unwrap_or_default();

                        // merge constraints into a single set
                        left_constraints.append(&mut right_constraints);
                        Some(left_constraints)
                    }
                    mir::BinaryOperator::Equal | mir::BinaryOperator::NotEqual => {
                        // require the equality branch to hold
                        if truth_value != matches!(operator, mir::BinaryOperator::Equal) {
                            return None;
                        }

                        // resolve bound keys for both sides
                        let left_key = bound_key_for_value(
                            left.value()?,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        );
                        let right_key = bound_key_for_value(
                            right.value()?,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        );

                        // require compatible signedness
                        let is_signed = signedness_for_bounds(&left_key, &right_key, ranges)?;

                        // emit inclusive lower and upper constraints
                        let constraints = vec![
                            BoundsConstraint {
                                value: left_key.clone(),
                                bound: right_key.clone(),
                                relation: ConstraintRelation::LowerInclusive,
                                is_signed,
                            },
                            BoundsConstraint {
                                value: left_key,
                                bound: right_key,
                                relation: ConstraintRelation::UpperInclusive,
                                is_signed,
                            },
                        ];

                        Some(constraints)
                    }
                    _ => {
                        // resolve bound keys for comparison
                        let left_key = bound_key_for_value(
                            left.value()?,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        );
                        let right_key = bound_key_for_value(
                            right.value()?,
                            block_id,
                            definitions,
                            tree,
                            constants,
                            ranges,
                        );

                        // build the comparison constraint
                        comparison_constraint(*operator, left_key, right_key, truth_value)
                            .map(|constraint| vec![constraint])
                    }
                },
                mir::Instruction::Unary {
                    operator: mir::UnaryOperator::Not,
                    argument,
                    ..
                } => {
                    // invert the truth value for not
                    constraints_for_condition(
                        argument.value()?,
                        !truth_value,
                        block_id,
                        definitions,
                        tree,
                        constants,
                        ranges,
                    )
                }
                _ => None,
            }
        }
        ValueDefinition::Parameter { .. } => None,
    }
}

/// Determine signedness for equality constraints.
fn signedness_for_bounds(left: &BoundKey, right: &BoundKey, ranges: &RangeMap) -> Option<bool> {
    // check constants first
    if let BoundKey::Constant(constant) = left {
        return constant_signedness(constant);
    }
    if let BoundKey::Constant(constant) = right {
        return constant_signedness(constant);
    }

    // fall back to range analysis
    let left_signedness = integer_signedness_for_bound(left, ranges)?;
    let right_signedness = integer_signedness_for_bound(right, ranges)?;

    // require matching signedness
    if left_signedness == right_signedness {
        Some(left_signedness)
    } else {
        None
    }
}

/// Determine signedness for a constant.
fn constant_signedness(constant: &ConstantKey) -> Option<bool> {
    // map constants to their signedness
    match constant {
        ConstantKey::Int { is_signed, .. } => Some(*is_signed),
        ConstantKey::UInt { .. } => Some(false),
        ConstantKey::Boolean(_) => None,
    }
}

/// Determine signedness for a bound using range analysis.
fn integer_signedness_for_bound(bound: &BoundKey, ranges: &RangeMap) -> Option<bool> {
    // require value based bounds
    let BoundKey::Value(value) = bound else {
        return None;
    };

    // extract integer signedness from ranges
    let range = ranges.get(*value)?;
    let ValueRange::Integer { is_signed, .. } = range else {
        return None;
    };

    Some(*is_signed)
}

/// Build a zero bound key for a value.
fn zero_bound_key_for_value(
    value: mir::Value,
    ranges: &RangeMap,
    is_signed: bool,
) -> Option<BoundKey> {
    // resolve integer range details for the value
    let range = ranges.get(value)?;
    let ValueRange::Integer {
        width,
        is_signed: range_signed,
        ..
    } = range
    else {
        return None;
    };

    // require matching signedness
    if *range_signed != is_signed {
        return None;
    }

    // build a zero constant key for the value width
    let constant = if is_signed {
        ConstantKey::Int {
            value: 0,
            width: *width,
            is_signed: true,
        }
    } else {
        ConstantKey::UInt {
            value: 0,
            width: *width,
        }
    };

    Some(BoundKey::Constant(constant))
}

/// Build a bound key from a value, using constant folding when possible.
fn bound_key_for_value(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    definitions: &ValueDefinitions,
    tree: &mir::Tree,
    constants: &ConstantPropagation,
    ranges: &RangeMap,
) -> BoundKey {
    // prefer constant propagation results
    if let Some(constant_key) = constants
        .constant_at_exit(block_id, value)
        .and_then(constant_key_from_constant)
    {
        return BoundKey::Constant(constant_key);
    }

    // fall back to constant ranges
    if let Some(constant_key) = ranges
        .get(value)
        .and_then(|range| range.as_constant())
        .and_then(|constant| constant_key_from_constant(&constant))
    {
        return BoundKey::Constant(constant_key);
    }

    // fall back to local constants
    if let Some(constant_key) = definitions.get(value).and_then(|definition| {
        // require instruction based values
        let ValueDefinition::Instruction { instruction } = definition else {
            return None;
        };

        // require constant instructions
        let instruction = tree.get(instruction);
        let mir::Instruction::Const {
            value: constant, ..
        } = instruction
        else {
            return None;
        };
        constant_key_from_constant(constant)
    }) {
        return BoundKey::Constant(constant_key);
    }

    BoundKey::Value(value)
}

/// Convert a MIR constant into a constraint key.
fn constant_key_from_constant(constant: &mir::Constant) -> Option<ConstantKey> {
    // map MIR constants into constraint keys
    match constant {
        mir::Constant::Boolean { value } => Some(ConstantKey::Boolean(*value)),
        mir::Constant::Int {
            value,
            width,
            is_signed,
        } => Some(ConstantKey::Int {
            value: *value,
            width: *width,
            is_signed: *is_signed,
        }),
        mir::Constant::UInt { value, width } => Some(ConstantKey::UInt {
            value: *value,
            width: *width,
        }),
        _ => None,
    }
}

/// Create a constraint from a comparison operator.
fn comparison_constraint(
    operator: mir::BinaryOperator,
    left: BoundKey,
    right: BoundKey,
    truth_value: bool,
) -> Option<BoundsConstraint> {
    // map the operator into a base relation and signedness
    let (relation, is_signed) = match operator {
        mir::BinaryOperator::SignedLessThan => (ConstraintRelation::UpperExclusive, true),
        mir::BinaryOperator::SignedLessEqual => (ConstraintRelation::UpperInclusive, true),
        mir::BinaryOperator::SignedGreaterThan => (ConstraintRelation::LowerExclusive, true),
        mir::BinaryOperator::SignedGreaterEqual => (ConstraintRelation::LowerInclusive, true),
        mir::BinaryOperator::UnsignedLessThan => (ConstraintRelation::UpperExclusive, false),
        mir::BinaryOperator::UnsignedLessEqual => (ConstraintRelation::UpperInclusive, false),
        mir::BinaryOperator::UnsignedGreaterThan => (ConstraintRelation::LowerExclusive, false),
        mir::BinaryOperator::UnsignedGreaterEqual => (ConstraintRelation::LowerInclusive, false),
        _ => return None,
    };

    // invert the relation for false branch constraints
    let relation = if truth_value {
        relation
    } else {
        match relation {
            ConstraintRelation::UpperExclusive => ConstraintRelation::LowerInclusive,
            ConstraintRelation::UpperInclusive => ConstraintRelation::LowerExclusive,
            ConstraintRelation::LowerExclusive => ConstraintRelation::UpperInclusive,
            ConstraintRelation::LowerInclusive => ConstraintRelation::UpperExclusive,
        }
    };

    Some(BoundsConstraint {
        value: left,
        bound: right,
        relation,
        is_signed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Constant branch encoded bounds stay explicit.
    #[test]
    fn test_preserve_constant_bounds_branch() {
        // source test
        let input = r#"
function test(v0: [int32; 4]): int32 {
b0(v0: [int32; 4]):
    v1: uint32 = 2uint32
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1, b2
b1:
    v4: int32 = element.get v0, v1
    return v4
b2:
    unreachable
}"#;

        let expected = input;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Constant bounds checks expressed as checks are eliminated.
    #[test]
    fn test_eliminate_constant_check_bounds() {
        // source test
        let input = r#"
function test(v0: [int32; 4]): int32 {
b0(v0: [int32; 4]):
    v1: uint32 = 2uint32
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b1, b2
b1:
    v4: int32 = element.get v0, v1
    return v4
b2:
    unreachable
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 4]): int32 {
b0(v0: [int32; 4]):
    v1: uint32 = 2uint32
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    jump b1
b1:
    v4: int32 = element.get v0, v1
    return v4
b2:
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Redundant branch encoded bounds stay explicit.
    #[test]
    fn test_preserve_redundant_bounds_branch() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1, b2
b1:
    v4: boolean = int.lt.u v1, v2
    branch v4, b3, b2
b2:
    unreachable
b3:
    v5: int32 = element.get v0, v1
    return v5
}"#;

        let expected = input;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Redundant check terminators are removed when dominated.
    #[test]
    fn test_eliminate_redundant_check_bounds() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b1, b2
b1:
    v4: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b3, b2
b2:
    unreachable
b3:
    v5: int32 = element.get v0, v1
    return v5
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b1, b2
b1:
    v4: boolean = int.lt.u v1, v2
    jump b3
b2:
    unreachable
b3:
    v5: int32 = element.get v0, v1
    return v5
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Checks without provable constraints are preserved.
    #[test]
    fn test_preserve_unknown_bounds_check() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1, b2
b1:
    v4: int32 = element.get v0, v1
    return v4
b2:
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_unchanged(input);
    }

    /// Conjoined branch encoded bounds stay explicit.
    #[test]
    fn test_preserve_conjoined_bounds_branch() {
        // source test
        let input = r#"
function test(v0: [int32; 8]): int32 {
b0(v0: [int32; 8]):
    v1: int32 = 3int32
    v2: int32 = 0int32
    v3: int32 = 8int32
    v4: boolean = int.ge.s v1, v2
    v5: boolean = int.lt.s v1, v3
    v6: boolean = int.and v4, v5
    branch v6, b1, b2
b1:
    v7: int32 = element.get v0, v1
    return v7
b2:
    unreachable
}"#;

        let expected = input;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Assume instructions imply bounds checks in the same block.
    #[test]
    fn test_assume_implies_bounds_check() {
        // source test
        let input = r#"
function test(v0: [int32; 16], v1: uint32): int32 {
b0(v0: [int32; 16], v1: uint32):
    v2: uint32 = 16uint32
    v3: boolean = int.lt.u v1, v2
    assume v3
    v4: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b1, b2
b1:
    v5: int32 = element.get v0, v1
    return v5
b2:
    unreachable
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 16], v1: uint32): int32 {
b0(v0: [int32; 16], v1: uint32):
    v2: uint32 = 16uint32
    v3: boolean = int.lt.u v1, v2
    assume v3
    v4: boolean = int.lt.u v1, v2
    jump b1
b1:
    v5: int32 = element.get v0, v1
    return v5
b2:
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Signed bounds checks use both lower and upper constraints.
    #[test]
    fn test_signed_bounds_constraints() {
        // source test
        let input = r#"
function test(v0: [int32; 8], v1: int32): int32 {
b0(v0: [int32; 8], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 8int32
    v4: boolean = int.ge.s v1, v2
    v5: boolean = int.lt.s v1, v3
    v6: boolean = int.and v4, v5
    branch v6, b1, b2
b1:
    check bounds.s v1, v3, v0 -> b3, b2
b2:
    unreachable
b3:
    v7: int32 = element.get v0, v1
    return v7
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 8], v1: int32): int32 {
b0(v0: [int32; 8], v1: int32):
    v2: int32 = 0int32
    v3: int32 = 8int32
    v4: boolean = int.ge.s v1, v2
    v5: boolean = int.lt.s v1, v3
    v6: boolean = int.and v4, v5
    branch v6, b1, b2
b1:
    jump b3
b2:
    unreachable
b3:
    v7: int32 = element.get v0, v1
    return v7
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Dominated checks beyond the immediate successor are removed.
    #[test]
    fn test_eliminate_dominated_bounds_check() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1, b2
b1:
    jump b3
b2:
    unreachable
b3:
    v4: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b4, b2
b4:
    v5: int32 = element.get v0, v1
    return v5
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1, b2
b1:
    jump b3
b2:
    unreachable
b3:
    v4: boolean = int.lt.u v1, v2
    jump b4
b4:
    v5: int32 = element.get v0, v1
    return v5
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Block parameter aliases are honored when proving redundancy.
    #[test]
    fn test_eliminate_bounds_check_with_block_param() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1(v1), b2
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v2
    check bounds.u v4, v2, v0 -> b3, b2
b2:
    unreachable
b3:
    v6: int32 = element.get v0, v4
    return v6
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1(v1), b2
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v2
    jump b3
b2:
    unreachable
b3:
    v6: int32 = element.get v0, v4
    return v6
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Branch encoded trap bounds stay explicit.
    #[test]
    fn test_preserve_bounds_branch_with_trap_then_target() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 2uint32
    v3: uint32 = 4uint32
    v4: boolean = int.ge.u v2, v3
    branch v4, b2, b1
b1:
    v5: int32 = element.get v0, v1
    return v5
b2:
    unreachable
}"#;

        let expected = input;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Dominating assumes eliminate bounds checks in successors.
    #[test]
    fn test_assume_in_predecessor_implies_bounds_check() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    assume v3
    jump b1
b1:
    v4: boolean = int.lt.u v1, v2
    check bounds.u v1, v2, v0 -> b2, b3
b2:
    v5: int32 = element.get v0, v1
    return v5
b3:
    unreachable
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    assume v3
    jump b1
b1:
    v4: boolean = int.lt.u v1, v2
    jump b2
b2:
    v5: int32 = element.get v0, v1
    return v5
b3:
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }

    /// Bounds checks are preserved when both branch paths reach the target.
    #[test]
    fn test_preserve_bounds_check_when_else_reaches_target() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1(v1), b2
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v2
    check bounds.u v4, v2, v0 -> b3, b4
b2:
    jump b1(v1)
b3:
    v6: int32 = element.get v0, v4
    return v6
b4:
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_unchanged(input);
    }

    /// Conflicting block parameters prevent redundancy elimination.
    #[test]
    fn test_preserve_bounds_check_with_conflicting_block_param() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1(v1), b2
b1(v4: uint32):
    v5: boolean = int.lt.u v4, v2
    check bounds.u v4, v2, v0 -> b3, b4
b2:
    v6: uint32 = 1uint32
    jump b1(v6)
b3:
    v7: int32 = element.get v0, v4
    return v7
b4:
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_unchanged(input);
    }

    /// Bounds checks are preserved when the trap block has side effects.
    #[test]
    fn test_preserve_non_trap_branch() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 4uint32
    v3: boolean = int.lt.u v1, v2
    branch v3, b1, b2
b1:
    v4: int32 = element.get v0, v1
    return v4
b2:
    v5: uint32 = 0uint32
    unreachable
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_unchanged(input);
    }

    /// Upper inclusive guards with constant bounds imply bounds checks.
    #[test]
    fn test_eliminate_upper_inclusive_guard() {
        // source test
        let input = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 3uint32
    v3: uint32 = 4uint32
    v4: boolean = int.le.u v1, v2
    branch v4, b1, b2
b1:
    v5: boolean = int.lt.u v1, v3
    check bounds.u v1, v3, v0 -> b3, b2
b2:
    unreachable
b3:
    v6: int32 = element.get v0, v1
    return v6
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 4], v1: uint32): int32 {
b0(v0: [int32; 4], v1: uint32):
    v2: uint32 = 3uint32
    v3: uint32 = 4uint32
    v4: boolean = int.le.u v1, v2
    branch v4, b1, b2
b1:
    v5: boolean = int.lt.u v1, v3
    jump b3
b2:
    unreachable
b3:
    v6: int32 = element.get v0, v1
    return v6
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&BoundsCheckEliminate);
        test.assert_output(expected);
    }
}
