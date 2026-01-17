use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, ScalarEvolution, Scev,
};
use crate::optimize::common::{
    BlockParamForwarding, clone_loop_blocks, constant_from_global,
    terminator_arguments_for_successor, terminator_remap,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Unroll loops with a constant trip count.
    ///
    /// Replaces the loop backedge with a chain of unrolled iterations.
    /// This eliminates loop control overhead and exposes instruction level parallelism for further scalar optimizations.
    ///
    /// ```mir
    /// function @before(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iconst 0i32
    ///     v2 = iconst 3i32
    ///     jump block1(v1)
    /// block1(v3: i32):
    ///     v4 = icmp_slt v3, v2
    ///     branch v4, block2, block3
    /// block2:
    ///     v5 = iadd v3, v1
    ///     v6 = iadd v3, v1
    ///     jump block1(v6)
    /// block3:
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     v1 = iconst 0i32
    ///     v2 = iconst 3i32
    ///     jump block1(v1)
    /// block1(v3: i32):
    ///     v4 = icmp_slt v3, v2
    ///     v5 = iadd v3, v1
    ///     v6 = iadd v3, v1
    ///     jump block4(v6)
    /// block4(v7: i32):
    ///     v8 = icmp_slt v7, v2
    ///     v9 = iadd v7, v1
    ///     v10 = iadd v7, v1
    ///     jump block7(v10)
    /// block7(v11: i32):
    ///     v12 = icmp_slt v11, v2
    ///     v13 = iadd v11, v1
    ///     v14 = iadd v11, v1
    ///     jump block3(v11)
    /// block3:
    ///     return v3
    /// }
    /// ```
    ///
    /// Requires a single latch and a single exiting block.
    /// Requires a constant trip count computed from scalar evolution.
    /// Only unrolls loops with speculatable guards.
    #[pass(id = "loop-unroll")]
    pub LoopUnroll,
    "Unroll loops with constant trip counts"
}

/// Maximum iterations to fully unroll.
const MAX_FULL_UNROLL_ITERATIONS: u64 = 8;
/// Maximum unroll factor for partial unrolling.
const MAX_PARTIAL_UNROLL_FACTOR: u64 = 4;
/// Maximum trip count to consider for partial unrolling.
const MAX_PARTIAL_UNROLL_TRIP_COUNT: u64 = 64;
/// Maximum number of loops to unroll per pass invocation.
const MAX_UNROLL_LOOPS_PER_FUNCTION: usize = 8;

impl FunctionPass for LoopUnroll {
    /// Run loop unrolling on the function.
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

        // run loop unrolling
        let changed = run_loop_unroll(function, tree, ctx);

        // invalidate analyses on change
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "LoopUnroll"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "loop-unroll"
    }
}

/// Unroll strategy selection.
#[derive(Debug, Clone, Copy)]
enum UnrollMode {
    /// Fully unroll the loop for the exact trip count.
    Full {
        /// Number of iterations.
        trip_count: u64,
    },
    /// Partially unroll by a factor without remainder handling.
    Partial {
        /// Unroll factor.
        factor: u64,
        /// Remainder iterations peeled before the loop.
        remainder: u64,
    },
}

/// Candidate loop data for unrolling.
#[derive(Debug, Clone)]
struct UnrollCandidate {
    /// Loop header block.
    header: mir::LocalNodeId<mir::Block>,
    /// Loop latch block.
    latch: mir::LocalNodeId<mir::Block>,
    /// Exit block outside the loop.
    exit_block: mir::LocalNodeId<mir::Block>,
    /// True when the in loop edge is the then branch.
    in_loop_is_then: bool,
    /// Loop blocks (original iteration).
    loop_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// True when the guard is located in the latch.
    guard_at_latch: bool,
    /// Trip count for the loop.
    trip_count: u64,
}

/// Comparison semantics for the loop guard.
#[derive(Debug, Clone)]
struct GuardComparison {
    /// Induction variable value.
    induction: mir::Value,
    /// Bound value.
    bound: mir::Value,
    /// True if the comparison is signed.
    is_signed: bool,
    /// True for a strict comparison (< or >).
    is_strict: bool,
    /// Direction implied by the comparison.
    direction: GuardDirection,
}

/// Loop direction implied by the guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuardDirection {
    /// Induction variable increases toward a bound.
    Increasing,
    /// Induction variable decreases toward a bound.
    Decreasing,
}

/// Per iteration state used while unrolling.
#[derive(Debug, Clone)]
struct UnrollIteration {
    /// Latch block for this iteration.
    latch: mir::LocalNodeId<mir::Block>,
    /// Header block for this iteration.
    header: mir::LocalNodeId<mir::Block>,
}

/// Run loop unrolling and return true when changes were made.
fn run_loop_unroll(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // read the unroll threshold from the pipeline options
    let unroll_threshold = ctx.unroll_threshold();

    if unroll_threshold == 0 {
        return false;
    }

    // track loop unrolling progress in this pass
    let mut changed = false;
    let mut unrolled_headers = HashSet::new();
    let mut iterations = 0usize;

    loop {
        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let loops = analyses.get::<LoopAnalysis>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let scev = analyses.get::<ScalarEvolution>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();

        // bail out when no loops exist
        if loops.num_loops() == 0 {
            break;
        }

        // build forwarding to normalize values
        let forwarding = BlockParamForwarding::build(function, tree, &cfg);

        // gather candidates in order of inner loops first
        let mut loop_indices: Vec<usize> = (0..loops.num_loops()).collect();
        loop_indices.sort_by_key(|index| {
            let lp = &loops.loops()[*index];
            (std::cmp::Reverse(lp.depth), lp.blocks.len(), lp.header)
        });

        // pick the first viable candidate for unrolling
        let mut selected: Option<(UnrollCandidate, UnrollMode)> = None;
        for loop_index in loop_indices {
            let lp = &loops.loops()[loop_index];
            if unrolled_headers.contains(&lp.header) {
                continue;
            }

            let Some(candidate) = find_unroll_candidate(
                lp,
                loop_index,
                function,
                tree,
                &scev,
                &forwarding,
                unroll_threshold,
            ) else {
                continue;
            };

            let Some(mode) = select_unroll_mode(&candidate, tree) else {
                continue;
            };

            selected = Some((candidate, mode));
            break;
        }

        // exit when no eligible loops remain
        let Some((candidate, mode)) = selected else {
            break;
        };

        // apply transformation
        function.recompute_next_value_id(tree);
        if !unroll_loop(function, tree, &candidate, mode, &cfg, &domtree) {
            break;
        }

        // record the successful unroll and guard the iteration count
        unrolled_headers.insert(candidate.header);
        changed = true;
        iterations += 1;
        if iterations >= MAX_UNROLL_LOOPS_PER_FUNCTION {
            break;
        }
    }

    changed
}

/// Find a loop candidate with a constant trip count.
fn find_unroll_candidate(
    lp: &Loop,
    loop_index: usize,
    function: &mir::Function,
    tree: &mir::NodeTree,
    scev: &ScalarEvolution,
    forwarding: &BlockParamForwarding,
    unroll_threshold: usize,
) -> Option<UnrollCandidate> {
    // require a single latch and a single exit edge
    if !lp.has_single_latch() {
        return None;
    }

    if !lp.has_single_exit() {
        return None;
    }

    if lp.exiting_blocks.len() != 1 {
        return None;
    }

    // compute loop size to avoid excessive code growth
    let loop_size: usize = lp
        .blocks
        .iter()
        .map(|block_id| tree.get(*block_id).instructions.len())
        .sum();
    if loop_size > unroll_threshold {
        return None;
    }

    // identify the exiting block and guard branch
    let exiting_block = lp.exiting_blocks[0];
    let exiting = tree.get(exiting_block);

    let (condition, in_loop_is_then, _loop_successor, exit_block) = match exiting.terminator {
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

            if then_in_loop {
                (condition, true, then_target, else_target)
            } else {
                (condition, false, else_target, then_target)
            }
        }
        _ => return None,
    };

    // find the latch and validate its backedge
    let latch = lp.latches[0];
    let latch_block = tree.get(latch);
    if !latch_block.terminator.successors().contains(&lp.header) {
        return None;
    }
    let _latch_arguments = terminator_arguments_for_successor(&latch_block.terminator, lp.header);

    // require guard either in header or latch
    let guard_at_latch = exiting_block == latch;
    if !guard_at_latch && exiting_block != lp.header {
        return None;
    }

    // extract guard comparison
    let guard = guard_from_condition(condition, in_loop_is_then, function, tree, forwarding)?;

    // compute trip count from scalar evolution
    let mut trip_count =
        trip_count_for_guard(&guard, loop_index, scev, forwarding, function, tree)?;
    if guard_at_latch {
        trip_count = trip_count.saturating_add(1);
    }
    if trip_count == 0 {
        return None;
    }

    // validate exit arguments for header guarded loops
    if !guard_at_latch {
        let header_params: Vec<mir::Value> = tree
            .get(lp.header)
            .parameters
            .iter()
            .map(|param| param.value)
            .collect();
        let (exit_args, _) = guard_exit_arguments(exiting, in_loop_is_then)?;
        if exit_args != header_params {
            return None;
        }
    }

    Some(UnrollCandidate {
        header: lp.header,
        latch,
        exit_block,
        in_loop_is_then,
        loop_blocks: lp.blocks.clone(),
        guard_at_latch,
        trip_count,
    })
}

/// Select unroll mode based on trip count and thresholds.
fn select_unroll_mode(candidate: &UnrollCandidate, _tree: &mir::NodeTree) -> Option<UnrollMode> {
    // allow full unroll when the trip count is small
    if candidate.trip_count <= MAX_FULL_UNROLL_ITERATIONS {
        return Some(UnrollMode::Full {
            trip_count: candidate.trip_count,
        });
    }

    // reject partial unroll for large trip counts
    if candidate.trip_count > MAX_PARTIAL_UNROLL_TRIP_COUNT {
        return None;
    }

    // choose a factor based on trip count
    let factor = MAX_PARTIAL_UNROLL_FACTOR.min(candidate.trip_count);
    if factor < 2 {
        return None;
    }

    // compute remainder to peel before the main loop
    let remainder = if candidate.guard_at_latch {
        candidate.trip_count % factor
    } else {
        0
    };

    Some(UnrollMode::Partial { factor, remainder })
}

/// Unroll the loop according to the selected mode.
fn unroll_loop(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    candidate: &UnrollCandidate,
    mode: UnrollMode,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
) -> bool {
    // peel remainder iterations before unrolling
    if let UnrollMode::Partial { remainder, .. } = mode
        && remainder > 0
    {
        let peeled = peel_remainder(function, tree, candidate, cfg, domtree, remainder);
        if !peeled {
            return false;
        }
    }

    // determine iteration count in this unroll group
    let iterations = match mode {
        UnrollMode::Full { trip_count } => trip_count,
        UnrollMode::Partial { factor, .. } => factor,
    };

    // skip degenerate cases
    if iterations < 2 {
        return false;
    }

    // build iteration metadata
    let mut iteration_data = Vec::new();
    iteration_data.push(UnrollIteration {
        latch: candidate.latch,
        header: candidate.header,
    });

    // clone loop blocks for each extra iteration
    for _ in 1..iterations {
        // clone blocks and values
        let (block_map, value_map) = clone_loop_blocks(&candidate.loop_blocks, function, tree);

        // remap terminators to cloned targets
        for &cloned_id in block_map.values() {
            let mut block = tree.get(cloned_id).clone();
            terminator_remap(&mut block.terminator, &block_map, &value_map);
            tree.replace(cloned_id, block);
        }

        // add cloned blocks to the function
        let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
        cloned_blocks.sort();
        for block_id in cloned_blocks {
            function.blocks.push(block_id);
        }

        // record iteration data
        iteration_data.push(UnrollIteration {
            latch: block_map[&candidate.latch],
            header: block_map[&candidate.header],
        });
    }

    // rewrite latch edges for each iteration
    for (index, iteration) in iteration_data.iter().enumerate() {
        // determine if this is the last unrolled iteration
        let is_last = index + 1 == iteration_data.len();
        let next_iteration = if is_last {
            None
        } else {
            Some(&iteration_data[index + 1])
        };

        // update the latch terminator
        let mut latch_block = tree.get(iteration.latch).clone();
        let updated = rewrite_latch_block(
            &mut latch_block,
            candidate,
            iteration,
            next_iteration,
            mode,
            is_last,
        );
        if !updated {
            return false;
        }

        tree.replace(iteration.latch, latch_block);
    }

    true
}

/// Peel remainder iterations before the main unrolled loop.
fn peel_remainder(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    candidate: &UnrollCandidate,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    remainder: u64,
) -> bool {
    // reject non latch guarded loops
    if !candidate.guard_at_latch {
        return false;
    }

    // find a preheader outside of the loop
    let Some((preheader, preheader_args)) = find_preheader(candidate, cfg, domtree, tree) else {
        return false;
    };

    // clone iterations for the remainder
    let mut peeled_iterations = Vec::new();
    for _ in 0..remainder {
        // clone loop blocks and values
        let (block_map, value_map) = clone_loop_blocks(&candidate.loop_blocks, function, tree);

        // remap cloned terminators
        for &cloned_id in block_map.values() {
            let mut block = tree.get(cloned_id).clone();
            terminator_remap(&mut block.terminator, &block_map, &value_map);
            tree.replace(cloned_id, block);
        }

        // insert cloned blocks into the function
        let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
        cloned_blocks.sort();
        for block_id in cloned_blocks {
            function.blocks.push(block_id);
        }

        // record peeled header and latch
        peeled_iterations.push(UnrollIteration {
            header: block_map[&candidate.header],
            latch: block_map[&candidate.latch],
        });
    }

    // redirect the preheader to the first peeled header
    let Some(first_iteration) = peeled_iterations.first() else {
        return true;
    };
    let mut preheader_block = tree.get(preheader).clone();
    preheader_block.terminator = mir::Terminator::Jump {
        target: first_iteration.header,
        arguments: preheader_args,
    };
    tree.replace(preheader, preheader_block);

    // chain peeled iterations together
    for (index, iteration) in peeled_iterations.iter().enumerate() {
        let is_last = index + 1 == peeled_iterations.len();
        let next_header = if is_last {
            candidate.header
        } else {
            peeled_iterations[index + 1].header
        };

        let mut latch_block = tree.get(iteration.latch).clone();
        let updated = rewrite_latch_to_jump(&mut latch_block, iteration.header, next_header);
        if !updated {
            return false;
        }

        tree.replace(iteration.latch, latch_block);
    }

    true
}

/// Locate a loop preheader and its arguments.
fn find_preheader(
    candidate: &UnrollCandidate,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::NodeTree,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)> {
    // collect predecessors outside the loop
    let mut outside_preds: Vec<_> = cfg
        .predecessors(candidate.header)
        .iter()
        .copied()
        .filter(|pred| !candidate.loop_blocks.contains(pred))
        .collect();

    // require a single outside predecessor
    if outside_preds.len() != 1 {
        return None;
    }
    let preheader = outside_preds.pop()?;

    // ensure the preheader dominates the header
    if !domtree.dominates(preheader, candidate.header) {
        return None;
    }

    // require a direct jump to the header
    let preheader_block = tree.get(preheader);
    let arguments = match &preheader_block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == candidate.header => {
            arguments.clone()
        }
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Rewrite a latch to unconditionally jump to the next header.
fn rewrite_latch_to_jump(
    block: &mut mir::Block,
    header: mir::LocalNodeId<mir::Block>,
    next_header: mir::LocalNodeId<mir::Block>,
) -> bool {
    // locate the loop backedge arguments
    let latch_has_edge = block.terminator.successors().contains(&header);
    if !latch_has_edge {
        return false;
    }
    let latch_args = terminator_arguments_for_successor(&block.terminator, header);

    // replace the latch terminator with a jump
    block.terminator = mir::Terminator::Jump {
        target: next_header,
        arguments: latch_args.to_vec(),
    };

    true
}

/// Rewrite an exiting block for a specific unrolled iteration.
fn rewrite_latch_block(
    block: &mut mir::Block,
    candidate: &UnrollCandidate,
    iteration: &UnrollIteration,
    next_iteration: Option<&UnrollIteration>,
    mode: UnrollMode,
    is_last: bool,
) -> bool {
    // extract latch arguments
    let latch_has_edge = block.terminator.successors().contains(&iteration.header);
    if !latch_has_edge {
        return false;
    }
    let latch_args = terminator_arguments_for_successor(&block.terminator, iteration.header);

    // handle non last iterations
    if !is_last {
        let Some(next) = next_iteration else {
            return false;
        };

        // redirect to the next iteration header
        block.terminator = mir::Terminator::Jump {
            target: next.header,
            arguments: latch_args.to_vec(),
        };

        return true;
    }

    // handle full unroll last iteration
    if matches!(mode, UnrollMode::Full { .. }) {
        // resolve exit arguments for this iteration
        let exit_arguments = if candidate.guard_at_latch {
            let Some((exit_args, _)) = guard_exit_arguments(block, candidate.in_loop_is_then)
            else {
                return false;
            };
            exit_args
        } else {
            latch_args.to_vec()
        };

        block.terminator = mir::Terminator::Jump {
            target: candidate.exit_block,
            arguments: exit_arguments,
        };

        return true;
    }

    // handle partial unroll with header guards
    if !candidate.guard_at_latch {
        block.terminator = mir::Terminator::Jump {
            target: candidate.header,
            arguments: latch_args.to_vec(),
        };

        return true;
    }

    // handle partial unroll last iteration by redirecting the guard
    let mut updated = block.clone();
    let (condition, then_arguments, else_arguments) = match updated.terminator {
        mir::Terminator::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => (condition, then_arguments, else_arguments),
        _ => return false,
    };

    let (then_target, else_target) = if candidate.in_loop_is_then {
        (candidate.header, candidate.exit_block)
    } else {
        (candidate.exit_block, candidate.header)
    };

    updated.terminator = mir::Terminator::Branch {
        condition,
        then_target,
        then_arguments,
        else_target,
        else_arguments,
    };
    *block = updated;

    true
}

/// Build a guard comparison from a condition value.
fn guard_from_condition(
    condition: mir::Value,
    guard_is_true: bool,
    function: &mir::Function,
    tree: &mir::NodeTree,
    forwarding: &BlockParamForwarding,
) -> Option<GuardComparison> {
    // resolve forwarded values
    let condition = forwarding.resolve(condition);

    // look up the instruction defining the condition
    let definitions = ValueDefinitions::build(function, tree);
    let instruction_id = definitions.definition_for(condition)?;
    let instruction = tree.get(instruction_id);

    // require a comparison instruction
    let (operator, left, right) = match instruction {
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => (*operator, *left, *right),
        _ => return None,
    };

    // normalize to a guard comparison
    let left = forwarding.resolve(left);
    let right = forwarding.resolve(right);
    normalize_guard(operator, left, right, guard_is_true)
}

/// Normalize a comparison into a guard description.
fn normalize_guard(
    operator: mir::BinaryOperator,
    left: mir::Value,
    right: mir::Value,
    guard_is_true: bool,
) -> Option<GuardComparison> {
    // flip operator when guard is false
    let operator = if guard_is_true {
        operator
    } else {
        invert_comparison(operator)?
    };

    // map operators to a comparison description
    match operator {
        mir::BinaryOperator::SignedLessThan => Some(GuardComparison {
            induction: left,
            bound: right,
            is_signed: true,
            is_strict: true,
            direction: GuardDirection::Increasing,
        }),
        mir::BinaryOperator::SignedLessEqual => Some(GuardComparison {
            induction: left,
            bound: right,
            is_signed: true,
            is_strict: false,
            direction: GuardDirection::Increasing,
        }),
        mir::BinaryOperator::UnsignedLessThan => Some(GuardComparison {
            induction: left,
            bound: right,
            is_signed: false,
            is_strict: true,
            direction: GuardDirection::Increasing,
        }),
        mir::BinaryOperator::UnsignedLessEqual => Some(GuardComparison {
            induction: left,
            bound: right,
            is_signed: false,
            is_strict: false,
            direction: GuardDirection::Increasing,
        }),
        mir::BinaryOperator::SignedGreaterThan => Some(GuardComparison {
            induction: left,
            bound: right,
            is_signed: true,
            is_strict: true,
            direction: GuardDirection::Decreasing,
        }),
        mir::BinaryOperator::SignedGreaterEqual => Some(GuardComparison {
            induction: left,
            bound: right,
            is_signed: true,
            is_strict: false,
            direction: GuardDirection::Decreasing,
        }),
        _ => None,
    }
}

/// Invert a comparison operator.
fn invert_comparison(operator: mir::BinaryOperator) -> Option<mir::BinaryOperator> {
    match operator {
        mir::BinaryOperator::SignedLessThan => Some(mir::BinaryOperator::SignedGreaterEqual),
        mir::BinaryOperator::SignedLessEqual => Some(mir::BinaryOperator::SignedGreaterThan),
        mir::BinaryOperator::SignedGreaterThan => Some(mir::BinaryOperator::SignedLessEqual),
        mir::BinaryOperator::SignedGreaterEqual => Some(mir::BinaryOperator::SignedLessThan),
        mir::BinaryOperator::UnsignedLessThan => Some(mir::BinaryOperator::UnsignedGreaterEqual),
        mir::BinaryOperator::UnsignedLessEqual => Some(mir::BinaryOperator::UnsignedGreaterThan),
        mir::BinaryOperator::UnsignedGreaterThan => Some(mir::BinaryOperator::UnsignedLessEqual),
        mir::BinaryOperator::UnsignedGreaterEqual => Some(mir::BinaryOperator::UnsignedLessThan),
        _ => None,
    }
}

/// Compute the trip count for the loop guard.
fn trip_count_for_guard(
    guard: &GuardComparison,
    loop_index: usize,
    scev: &ScalarEvolution,
    forwarding: &BlockParamForwarding,
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> Option<u64> {
    // resolve forwarded values
    let induction = forwarding.resolve(guard.induction);
    let bound = forwarding.resolve(guard.bound);

    // look up scalar evolution for the induction variable
    let scev_expr = scev.scev_for_value_in_loop(loop_index, induction)?;
    let (start, step) = match scev_expr {
        Scev::AddRec { start, step, .. } => (start.as_ref(), step.as_ref()),
        _ => return None,
    };

    // extract constants
    let start_const = scev_constant(start)?;
    let step_const = scev_constant(step)?;
    let bound_const = scev
        .scev_for_value_in_loop(loop_index, bound)
        .and_then(scev_constant)
        .cloned()
        .or_else(|| constant_value_for(bound, function, tree, forwarding))?;

    // compute trip count by signed or unsigned semantics
    if guard.is_signed {
        let start = constant_to_i128(start_const)?;
        let step = constant_to_i128(step_const)?;
        let bound = constant_to_i128(&bound_const)?;
        trip_count_signed(start, bound, step, guard.is_strict, guard.direction)
    } else {
        let start = constant_to_u128(start_const)?;
        let step = constant_to_u128(step_const)?;
        let bound = constant_to_u128(&bound_const)?;
        trip_count_unsigned(start, bound, step, guard.is_strict)
    }
}

/// Extract a constant from a scalar evolution node.
fn scev_constant(scev: &Scev) -> Option<&mir::Constant> {
    match scev {
        Scev::Constant(constant) => Some(constant),
        _ => None,
    }
}

/// Resolve a constant value when it is defined outside of SCEV.
fn constant_value_for(
    value: mir::Value,
    function: &mir::Function,
    tree: &mir::NodeTree,
    forwarding: &BlockParamForwarding,
) -> Option<mir::Constant> {
    // resolve forwarded values
    let value = forwarding.resolve(value);

    // build a definition map for constants
    let definitions = ValueDefinitions::build(function, tree);
    let instruction_id = definitions.definition_for(value)?;
    let instruction = tree.get(instruction_id);

    match instruction {
        mir::Instruction::Const { value, .. } => Some(value.clone()),
        mir::Instruction::GlobalConst { global, .. } => constant_from_global(*global, tree),
        _ => None,
    }
}

/// Convert a constant to a signed integer.
fn constant_to_i128(constant: &mir::Constant) -> Option<i128> {
    match constant {
        mir::Constant::Int { value, .. } => Some(*value as i128),
        mir::Constant::UInt { value, .. } => Some(*value as i128),
        _ => None,
    }
}

/// Convert a constant to an unsigned integer.
fn constant_to_u128(constant: &mir::Constant) -> Option<u128> {
    match constant {
        mir::Constant::UInt { value, .. } => Some(*value as u128),
        mir::Constant::Int { value, .. } if *value >= 0 => Some(*value as u128),
        _ => None,
    }
}

/// Return the ceil division for a non negative signed span and positive step.
#[allow(clippy::manual_div_ceil)]
/// Return the ceil division for a non negative signed span and positive step.
fn div_ceil_signed(span: i128, step: i128) -> i128 {
    // validate preconditions
    debug_assert!(span >= 0);
    debug_assert!(step > 0);

    // compute the adjusted numerator
    let adjusted = span.saturating_add(step - 1);

    adjusted / step
}

/// Return the ceil division for a non negative unsigned span and positive step.
#[allow(clippy::manual_div_ceil)]
/// Return the ceil division for a non negative unsigned span and positive step.
fn div_ceil_unsigned(span: u128, step: u128) -> u128 {
    // validate preconditions
    debug_assert!(step > 0);

    // compute the adjusted numerator
    let adjusted = span.saturating_add(step - 1);

    adjusted / step
}

/// Compute trip count for signed induction variables.
fn trip_count_signed(
    start: i128,
    bound: i128,
    step: i128,
    is_strict: bool,
    direction: GuardDirection,
) -> Option<u64> {
    // reject zero step
    if step == 0 {
        return None;
    }

    // compute trip count based on direction
    let count = match direction {
        GuardDirection::Increasing => {
            if step < 0 {
                return None;
            }

            if is_strict {
                if start >= bound {
                    return Some(0);
                }

                let span = bound - start;
                div_ceil_signed(span, step)
            } else {
                if start > bound {
                    return Some(0);
                }

                let span = bound - start;
                span / step + 1
            }
        }
        GuardDirection::Decreasing => {
            if step > 0 {
                return None;
            }

            let step = -step;
            if is_strict {
                if start <= bound {
                    return Some(0);
                }

                let span = start - bound;
                div_ceil_signed(span, step)
            } else {
                if start < bound {
                    return Some(0);
                }

                let span = start - bound;
                span / step + 1
            }
        }
    };

    u64::try_from(count).ok()
}

/// Compute trip count for unsigned induction variables.
fn trip_count_unsigned(start: u128, bound: u128, step: u128, is_strict: bool) -> Option<u64> {
    // reject zero step
    if step == 0 {
        return None;
    }

    // compute trip count for increasing loops
    let count = if is_strict {
        if start >= bound {
            return Some(0);
        }

        let span = bound - start;
        div_ceil_unsigned(span, step)
    } else {
        if start > bound {
            return Some(0);
        }

        let span = bound - start;
        span / step + 1
    };

    u64::try_from(count).ok()
}

/// Map from values to the instructions that define them.
#[derive(Debug)]
struct ValueDefinitions {
    /// Definition sites by value.
    definitions: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
}

impl ValueDefinitions {
    /// Build a value definition map for a function.
    fn build(function: &mir::Function, tree: &mir::NodeTree) -> Self {
        let mut definitions = HashMap::new();

        // scan all instruction destinations
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    definitions.insert(destination, instruction_id);
                }
            }
        }

        Self { definitions }
    }

    /// Get the instruction defining a value.
    fn definition_for(&self, value: mir::Value) -> Option<mir::LocalNodeId<mir::Instruction>> {
        self.definitions.get(&value).copied()
    }
}

/// Extract exit arguments for a guard terminator.
fn guard_exit_arguments(
    block: &mir::Block,
    in_loop_is_then: bool,
) -> Option<(Vec<mir::Value>, Vec<mir::Value>)> {
    // branch is required for unroll
    let (then_arguments, else_arguments) = match &block.terminator {
        mir::Terminator::Branch {
            then_arguments,
            else_arguments,
            ..
        } => (then_arguments.clone(), else_arguments.clone()),
        _ => return None,
    };

    // select exit args and in loop args
    if in_loop_is_then {
        Some((else_arguments, then_arguments))
    } else {
        Some((then_arguments, else_arguments))
    }
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::{LoopSimplify, LoopUnroll};

    /// Fully unroll a loop with a small constant trip count.
    #[test]
    fn test_full_unroll_small_trip_count() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 3i32
    v2 = iconst 1i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_slt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = iadd v5, v2
    jump block1(v6)
block3(v7: i32):
    return v7
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 3i32
    v2 = iconst 1i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_slt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = iadd v5, v2
    jump block4(v6)
block3(v7: i32):
    return v7
block4(v8: i32):
    v9 = icmp_slt v8, v1
    branch v9, block5(v8), block3(v8)
block5(v10: i32):
    v11 = iadd v10, v2
    jump block6(v11)
block6(v12: i32):
    v13 = icmp_slt v12, v1
    branch v13, block7(v12), block3(v12)
block7(v14: i32):
    v15 = iadd v14, v2
    jump block3(v15)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnroll);
        program.assert_output(expected);
    }

    /// Fully unroll a decreasing loop with constant trip count.
    #[test]
    fn test_full_unroll_decreasing_trip_count() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 3i32
    v1 = iconst 0i32
    v2 = iconst 1i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_sgt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = isub v5, v2
    jump block1(v6)
block3(v7: i32):
    return v7
}"#;

        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 3i32
    v1 = iconst 0i32
    v2 = iconst 1i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_sgt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = isub v5, v2
    jump block4(v6)
block3(v7: i32):
    return v7
block4(v8: i32):
    v9 = icmp_sgt v8, v1
    branch v9, block5(v8), block3(v8)
block5(v10: i32):
    v11 = isub v10, v2
    jump block6(v11)
block6(v12: i32):
    v13 = icmp_sgt v12, v1
    branch v13, block7(v12), block3(v12)
block7(v14: i32):
    v15 = isub v14, v2
    jump block3(v15)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnroll);
        program.assert_output(expected);
    }

    /// Do not unroll loops without constant trip count.
    #[test]
    fn test_unroll_requires_constant_trip_count() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    v2 = iconst 1i32
    jump block1(v1)
block1(v3: i32):
    v4 = icmp_slt v3, v0
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = iadd v5, v2
    jump block1(v6)
block3(v7: i32):
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnroll);
        program.assert_output(input);
    }

    /// Partially unroll loops with remainder by peeling iterations.
    #[test]
    fn test_partial_unroll_with_remainder() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 10i32
    v2 = iconst 1i32
    jump block1(v0)
block1(v3: i32):
    v4 = iadd v3, v2
    v5 = icmp_slt v4, v1
    branch v5, block1(v4), block2(v4)
block2(v6: i32):
    return v6
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 10i32
    v2 = iconst 1i32
    jump block3(v0)
block1(v3: i32):
    v4 = iadd v3, v2
    v5 = icmp_slt v4, v1
    jump block5(v4)
block2(v6: i32):
    return v6
block3(v7: i32):
    v8 = iadd v7, v2
    v9 = icmp_slt v8, v1
    jump block4(v8)
block4(v10: i32):
    v11 = iadd v10, v2
    v12 = icmp_slt v11, v1
    jump block1(v11)
block5(v13: i32):
    v14 = iadd v13, v2
    v15 = icmp_slt v14, v1
    jump block6(v14)
block6(v16: i32):
    v17 = iadd v16, v2
    v18 = icmp_slt v17, v1
    jump block7(v17)
block7(v19: i32):
    v20 = iadd v19, v2
    v21 = icmp_slt v20, v1
    branch v21, block1(v20), block2(v20)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnroll);
        program.assert_output(expected);
    }

    /// Header guarded loops are partially unrolled with guard chaining.
    #[test]
    fn test_partial_unroll_header_guard() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 9i32
    v2 = iconst 1i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_slt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = iadd v5, v2
    jump block1(v6)
block3(v7: i32):
    return v7
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 9i32
    v2 = iconst 1i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_slt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = iadd v5, v2
    jump block4(v6)
block3(v7: i32):
    return v7
block4(v8: i32):
    v9 = icmp_slt v8, v1
    branch v9, block5(v8), block3(v8)
block5(v10: i32):
    v11 = iadd v10, v2
    jump block6(v11)
block6(v12: i32):
    v13 = icmp_slt v12, v1
    branch v13, block7(v12), block3(v12)
block7(v14: i32):
    v15 = iadd v14, v2
    jump block8(v15)
block8(v16: i32):
    v17 = icmp_slt v16, v1
    branch v17, block9(v16), block3(v16)
block9(v18: i32):
    v19 = iadd v18, v2
    jump block1(v19)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnroll);
        program.assert_output(expected);
    }

    /// Non unit stride loops can be unrolled when the trip count is constant.
    #[test]
    fn test_unroll_non_unit_stride() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 6i32
    v2 = iconst 2i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_slt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = iadd v5, v2
    jump block1(v6)
block3(v7: i32):
    return v7
}"#;

        let expected = r#"function @test() -> i32 {
block0:
    v0 = iconst 0i32
    v1 = iconst 6i32
    v2 = iconst 2i32
    jump block1(v0)
block1(v3: i32):
    v4 = icmp_slt v3, v1
    branch v4, block2(v3), block3(v3)
block2(v5: i32):
    v6 = iadd v5, v2
    jump block4(v6)
block3(v7: i32):
    return v7
block4(v8: i32):
    v9 = icmp_slt v8, v1
    branch v9, block5(v8), block3(v8)
block5(v10: i32):
    v11 = iadd v10, v2
    jump block6(v11)
block6(v12: i32):
    v13 = icmp_slt v12, v1
    branch v13, block7(v12), block3(v12)
block7(v14: i32):
    v15 = iadd v14, v2
    jump block3(v15)
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnroll);
        program.assert_output(expected);
    }

    /// Multiple exits prevent unrolling.
    #[test]
    fn test_unroll_skips_multiple_exits() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = iconst 0i32
    v2 = iconst 4i32
    v3 = iconst 1i32
    jump block1(v1)
block1(v4: i32):
    v5 = icmp_slt v4, v2
    branch v5, block2(v4), block5(v4)
block2(v6: i32):
    branch v0, block3(v6), block4(v6)
block3(v7: i32):
    v8 = iadd v7, v3
    jump block1(v8)
block4(v9: i32):
    return v9
block5(v10: i32):
    return v10
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoopSimplify);
        program.run_pass(&LoopUnroll);
        program.assert_output(input);
    }
}
