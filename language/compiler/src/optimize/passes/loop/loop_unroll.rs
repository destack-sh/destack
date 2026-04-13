use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    ControlFlowGraph, DominatorTree, Loop, LoopAnalysis, ScalarEvolution, Scev,
};
use crate::optimize::common::{
    BlockParamForwarding, CallsiteHotness, CallsiteHotnessPolicy, ValueTypeMap,
    block_execution_counts, block_hotness_from_counts, build_use_def_maps,
    build_value_definition_map, clone_instruction_metadata, clone_loop_blocks,
    constant_from_global, instruction_is_speculatable, instruction_map,
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
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 0int32
    ///     v2 = 3int32
    ///     jump b1(v1)
    /// b1(v3: int32):
    ///     v4 = int.lt.s v3, v2
    ///     branch v4, b2, b3
    /// b2:
    ///     v5 = int.add v3, v1
    ///     v6 = int.add v3, v1
    ///     jump b1(v6)
    /// b3:
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     v1 = 0int32
    ///     v2 = 3int32
    ///     jump b1(v1)
    /// b1(v3: int32):
    ///     v4 = int.lt.s v3, v2
    ///     v5 = int.add v3, v1
    ///     v6 = int.add v3, v1
    ///     jump b4(v6)
    /// b4(v7: int32):
    ///     v8 = int.lt.s v7, v2
    ///     v9 = int.add v7, v1
    ///     v10 = int.add v7, v1
    ///     jump b7(v10)
    /// b7(v11: int32):
    ///     v12 = int.lt.s v11, v2
    ///     v13 = int.add v11, v1
    ///     v14 = int.add v11, v1
    ///     jump b3(v11)
    /// b3:
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

declare_pass! {
    /// Unroll and jam perfectly nested loops.
    ///
    /// The outer loop is unrolled and the inner loop body is duplicated so that
    /// each inner iteration executes multiple outer iterations.
    ///
    /// ```mir
    /// function before(v0: uint32, v1: uint32, v2: uint32[8]): void {
    /// b0(v0: uint32, v1: uint32, v2: uint32[8]):
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b2, b6
    /// b2:
    ///     v7 = 0uint32
    ///     jump b3(v5, v7)
    /// b3(v8: uint32, v9: uint32):
    ///     v10 = int.lt.u v9, v1
    ///     branch v10, b4(v8, v9), b5(v8)
    /// b4(v11: uint32, v12: uint32):
    ///     v13 = element.address v2, v12 -> ref<uint32, borrowed>
    ///     store v13, v11
    ///     v14 = int.add v12, v4
    ///     jump b3(v11, v14)
    /// b5(v15: uint32):
    ///     v16 = int.add v15, v4
    ///     jump b1(v16)
    /// b6:
    ///     return
    /// }
    /// ```
    /// becomes (with factor = 2):
    /// ```mir
    /// function after(v0: uint32, v1: uint32, v2: uint32[8]): void {
    /// b0(v0: uint32, v1: uint32, v2: uint32[8]):
    ///     v3 = 0uint32
    ///     v4 = 1uint32
    ///     jump b1(v3)
    /// b1(v5: uint32):
    ///     v6 = int.lt.u v5, v0
    ///     branch v6, b2, b6
    /// b2:
    ///     v7 = 0uint32
    ///     jump b3(v5, v7)
    /// b3(v8: uint32, v9: uint32):
    ///     v10 = int.lt.u v9, v1
    ///     branch v10, b4(v8, v9), b5(v8)
    /// b4(v11: uint32, v12: uint32):
    ///     v13 = element.address v2, v12 -> ref<uint32, borrowed>
    ///     store v13, v11
    ///     v14 = 1uint32
    ///     v15 = int.add v11, v14
    ///     v16 = element.address v2, v12 -> ref<uint32, borrowed>
    ///     store v16, v15
    ///     v17 = int.add v12, v4
    ///     jump b3(v11, v17)
    /// b5(v18: uint32):
    ///     v19 = 2uint32
    ///     v20 = int.add v18, v19
    ///     jump b1(v20)
    /// b6:
    ///     return
    /// }
    /// ```
    ///
    /// Requires perfectly nested loops with a constant outer trip count.
    #[pass(id = "loop-unroll-jam")]
    pub LoopUnrollAndJam,
    "Unroll and jam perfectly nested loops"
}

/// Maximum iterations to fully unroll.
const MAX_FULL_UNROLL_ITERATIONS: u64 = 8;
/// Maximum unroll factor for partial unrolling.
const MAX_PARTIAL_UNROLL_FACTOR: u64 = 4;
/// Maximum trip count to consider for partial unrolling.
const MAX_PARTIAL_UNROLL_TRIP_COUNT: u64 = 64;
/// Maximum number of loops to unroll per pass invocation.
const MAX_UNROLL_LOOPS_PER_FUNCTION: usize = 8;
/// Maximum number of loops to unroll and jam per pass invocation.
const MAX_JAM_LOOPS_PER_FUNCTION: usize = 4;

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

impl FunctionPass for LoopUnrollAndJam {
    /// Run loop unroll and jam on the function.
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

        // run loop unroll and jam
        let changed = run_loop_unroll_and_jam(function, tree, ctx);

        // invalidate analyses on change
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "LoopUnrollAndJam"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "loop-unroll-jam"
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

/// Per loop unroll and jam plan.
#[derive(Debug, Clone, Copy)]
struct JamPlan {
    /// Unroll factor for the outer loop.
    factor: u64,
    /// Remainder iterations to peel before jamming.
    remainder: u64,
}

/// Per loop unroll limits derived from profile data.
#[derive(Debug, Clone, Copy)]
struct UnrollLimits {
    /// Maximum iterations to fully unroll.
    max_full_unroll_iterations: u64,
    /// Maximum unroll factor for partial unrolling.
    max_partial_unroll_factor: u64,
    /// Maximum trip count to consider for partial unrolling.
    max_partial_unroll_trip_count: u64,
    /// Maximum body size to consider for unrolling.
    max_body_instructions: usize,
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

/// Candidate loop data for unroll and jam.
#[derive(Debug, Clone)]
struct JamCandidate {
    /// Outer loop header block.
    outer_header: mir::LocalNodeId<mir::Block>,
    /// Outer loop latch block.
    outer_latch: mir::LocalNodeId<mir::Block>,
    /// All blocks in the outer loop.
    outer_blocks: HashSet<mir::LocalNodeId<mir::Block>>,
    /// Inner loop header block.
    inner_header: mir::LocalNodeId<mir::Block>,
    /// Inner loop latch block.
    inner_latch: mir::LocalNodeId<mir::Block>,
    /// Outer induction value.
    outer_induction: mir::Value,
    /// Index of the outer induction parameter in the header.
    outer_param_index: usize,
    /// Outer latch parameter that carries the induction.
    outer_latch_param: mir::Value,
    /// Outer loop step value.
    outer_step: i128,
    /// Outer loop trip count.
    outer_trip_count: u64,
    /// Index of the inner induction parameter in the header.
    inner_param_index: usize,
    /// Inner header parameter that carries the outer induction.
    inner_outer_param: mir::Value,
    /// Values in the inner loop equivalent to the outer induction.
    outer_equivalents: Vec<mir::Value>,
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

    // collect profile data for hotness decisions
    let hotness_policy = ctx.inline_hotness_policy();
    let block_counts = match ctx.profile() {
        Some(profile) => block_execution_counts(function, tree, Some(profile), hotness_policy),
        None => HashMap::new(),
    };
    let entry_count = function
        .entry
        .and_then(|entry| block_counts.get(&entry).copied())
        .unwrap_or(0);

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

            let Some(limits) = unroll_limits_for_loop(
                lp.header,
                &block_counts,
                entry_count,
                hotness_policy,
                unroll_threshold,
            ) else {
                continue;
            };

            let Some(candidate) = find_unroll_candidate(
                lp,
                loop_index,
                function,
                tree,
                &scev,
                &forwarding,
                limits.max_body_instructions,
            ) else {
                continue;
            };

            let Some(mode) = select_unroll_mode(&candidate, &limits) else {
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

/// Run loop unroll and jam and return true when changes were made.
fn run_loop_unroll_and_jam(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // read the unroll threshold from the pipeline options
    let unroll_threshold = ctx.unroll_threshold();

    if unroll_threshold == 0 {
        return false;
    }

    // collect profile data for hotness decisions
    let hotness_policy = ctx.inline_hotness_policy();
    let block_counts = match ctx.profile() {
        Some(profile) => block_execution_counts(function, tree, Some(profile), hotness_policy),
        None => HashMap::new(),
    };
    let entry_count = function
        .entry
        .and_then(|entry| block_counts.get(&entry).copied())
        .unwrap_or(0);

    // track loop unroll and jam progress in this pass
    let mut changed = false;
    let mut jammed_headers = HashSet::new();
    let mut iterations = 0usize;

    loop {
        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let loops = analyses.get::<LoopAnalysis>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let scev = analyses.get::<ScalarEvolution>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let value_types = ValueTypeMap::new(function, tree);

        // bail out when no loops exist
        if loops.num_loops() == 0 {
            break;
        }

        // build forwarding to normalize values
        let forwarding = BlockParamForwarding::build(function, tree, &cfg);

        // build loop child mappings
        let children = build_loop_children_map(&loops);

        // pick the first viable candidate for unroll and jam
        let mut selected: Option<(JamCandidate, JamPlan)> = None;
        let mut loop_indices: Vec<usize> = (0..loops.num_loops()).collect();
        loop_indices.sort_by_key(|index| loops.loops()[*index].depth);

        for outer_index in loop_indices {
            let outer = &loops.loops()[outer_index];
            if jammed_headers.contains(&outer.header) {
                continue;
            }

            let inner_index = if children[outer_index].len() == 1 {
                children[outer_index][0]
            } else {
                let Some(nested) = nearest_nested_loop(&loops, outer_index) else {
                    continue;
                };
                nested
            };
            let inner = &loops.loops()[inner_index];

            let Some(limits) = unroll_limits_for_loop(
                outer.header,
                &block_counts,
                entry_count,
                hotness_policy,
                unroll_threshold,
            ) else {
                continue;
            };

            let Some(candidate) = find_jam_candidate(
                outer_index,
                outer,
                inner,
                function,
                tree,
                &cfg,
                &domtree,
                &scev,
                &forwarding,
            ) else {
                continue;
            };

            let allow_remainder = find_jam_preheader(&candidate, &cfg, &domtree, tree).is_some();
            let Some(plan) = select_jam_plan(&candidate, &limits, tree, allow_remainder) else {
                continue;
            };

            selected = Some((candidate, plan));
            break;
        }

        // exit when no eligible loops remain
        let Some((candidate, plan)) = selected else {
            break;
        };

        // apply transformation
        function.recompute_next_value_id(tree);
        if !unroll_and_jam_loop(
            function,
            tree,
            ctx,
            &candidate,
            plan,
            &cfg,
            &domtree,
            &value_types,
        ) {
            break;
        }

        // record the successful unroll and jam and guard the iteration count
        jammed_headers.insert(candidate.outer_header);
        changed = true;
        iterations += 1;
        if iterations >= MAX_JAM_LOOPS_PER_FUNCTION {
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
    let exiting_terminator = tree.get(exiting.terminator);

    let (condition, in_loop_is_then, _loop_successor, exit_block) = match exiting_terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
            ..
        } => {
            let then_target = then_target.block.block()?;
            let else_target = else_target.block.block()?;
            let condition = condition.value()?;
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
    let latch_terminator = tree.get(latch_block.terminator);
    if !latch_terminator
        .successors()
        .iter()
        .any(|successor| successor.block() == Some(lp.header))
    {
        return None;
    }
    let _latch_arguments = terminator_arguments_for_successor(latch_terminator, lp.header);

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
            .map(|param| param.value.value())
            .collect::<Option<Vec<_>>>()?;
        let exiting_terminator = tree.get(exiting.terminator);
        let (exit_args, _) = guard_exit_arguments(exiting_terminator, in_loop_is_then)?;
        let exit_args = exit_args
            .into_iter()
            .map(|value| value.value())
            .collect::<Option<Vec<_>>>()?;
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

/// Build a map from loop indices to their child loop indices.
fn build_loop_children_map(loops: &LoopAnalysis) -> Vec<Vec<usize>> {
    // initialize child lists
    let mut children = vec![Vec::new(); loops.num_loops()];

    // record parent relationships
    for (index, lp) in loops.loops().iter().enumerate() {
        if let Some(parent) = lp.parent {
            children[parent].push(index);
        }
    }

    children
}

/// Find the nearest nested loop index when parent links are missing.
fn nearest_nested_loop(loops: &LoopAnalysis, outer_index: usize) -> Option<usize> {
    // collect nested loop candidates
    let outer = &loops.loops()[outer_index];
    let mut candidates: Vec<(usize, u32)> = Vec::new();

    for (index, lp) in loops.loops().iter().enumerate() {
        // skip the outer loop
        if index == outer_index {
            continue;
        }

        // record loops fully contained in the outer loop
        if lp.blocks.iter().all(|block| outer.blocks.contains(block)) {
            candidates.push((index, lp.depth));
        }
    }

    // require at least one nested candidate
    if candidates.is_empty() {
        return None;
    }

    // select the shallowest nested loop
    candidates.sort_by_key(|(_, depth)| *depth);
    let min_depth = candidates[0].1;
    let mut nested = candidates
        .into_iter()
        .filter(|(_, depth)| *depth == min_depth)
        .map(|(index, _)| index);

    // reject ambiguous nesting
    let first = nested.next()?;
    if nested.next().is_some() {
        return None;
    }

    Some(first)
}

/// Find a perfectly nested loop candidate for unroll and jam.
// allow many arguments to keep loop selection explicit
#[allow(clippy::too_many_arguments)]
fn find_jam_candidate(
    outer_index: usize,
    outer: &Loop,
    inner: &Loop,
    function: &mir::Function,
    tree: &mir::NodeTree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    scev: &ScalarEvolution,
    forwarding: &BlockParamForwarding,
) -> Option<JamCandidate> {
    // require single latch and exit on both loops
    if !outer.has_single_latch() || !outer.has_single_exit() {
        return None;
    }
    if !inner.has_single_latch() || !inner.has_single_exit() {
        return None;
    }

    // require the inner loop exit to match the outer latch
    let outer_latch = outer.latches[0];
    let inner_exit = inner.exit_blocks.first().copied()?;
    if inner_exit != outer_latch {
        return None;
    }

    // require a header guard on the outer loop
    if outer.exiting_blocks.len() != 1 || outer.exiting_blocks[0] != outer.header {
        return None;
    }

    // require a header guard on the inner loop
    if inner.exiting_blocks.len() != 1 || inner.exiting_blocks[0] != inner.header {
        return None;
    }

    // require a minimal inner loop body (header and latch only)
    if inner.blocks.len() != 2 {
        return None;
    }

    // identify the outer guard and exit blocks
    let outer_header = outer.header;
    let outer_block = tree.get(outer_header);
    let outer_terminator = tree.get(outer_block.terminator);
    let (outer_condition, outer_in_loop_is_then, outer_in_loop_target) = match outer_terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
            ..
        } => {
            let then_target = then_target.block.block()?;
            let else_target = else_target.block.block()?;
            let condition = condition.value()?;
            let then_in_loop = outer.blocks.contains(&then_target);
            let else_in_loop = outer.blocks.contains(&else_target);
            if then_in_loop == else_in_loop {
                return None;
            }

            if then_in_loop {
                (condition, true, then_target)
            } else {
                (condition, false, else_target)
            }
        }
        _ => return None,
    };

    // resolve the inner preheader if needed
    let inner_header = inner.header;
    let inner_preheader = if outer_in_loop_target == inner_header {
        None
    } else {
        let preheader_block = tree.get(outer_in_loop_target);
        let preheader_terminator = tree.get(preheader_block.terminator);
        match preheader_terminator {
            mir::Terminator::Jump { target } if target.block.block() == Some(inner_header) => {
                Some(outer_in_loop_target)
            }
            _ => return None,
        }
    };

    // require the outer loop to be perfectly nested
    if !outer_is_perfectly_nested(outer, inner, inner_preheader) {
        return None;
    }

    // require the outer latch to jump back to the header
    let latch_block = tree.get(outer_latch);
    let latch_terminator = tree.get(latch_block.terminator);
    match latch_terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(outer_header) => {}
        _ => return None,
    }

    // require the inner latch to jump back to the header
    let inner_latch = inner.latches[0];
    let inner_latch_block = tree.get(inner_latch);
    let inner_latch_terminator = tree.get(inner_latch_block.terminator);
    match inner_latch_terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(inner_header) => {}
        _ => return None,
    }

    // extract guard comparisons
    let outer_guard = guard_from_condition(
        outer_condition,
        outer_in_loop_is_then,
        function,
        tree,
        forwarding,
    )?;
    let inner_block = tree.get(inner_header);
    let inner_terminator = tree.get(inner_block.terminator);
    let (inner_condition, inner_in_loop_is_then) = match inner_terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
            ..
        } => {
            let then_target = then_target.block.block()?;
            let else_target = else_target.block.block()?;
            let condition = condition.value()?;
            let then_in_loop = inner.blocks.contains(&then_target);
            let else_in_loop = inner.blocks.contains(&else_target);
            if then_in_loop == else_in_loop {
                return None;
            }

            let exit_target = if then_in_loop {
                else_target
            } else {
                then_target
            };
            if exit_target != inner_exit {
                return None;
            }

            (condition, then_in_loop)
        }
        _ => return None,
    };
    let inner_guard = guard_from_condition(
        inner_condition,
        inner_in_loop_is_then,
        function,
        tree,
        forwarding,
    )?;

    // require increasing strict guards for both loops
    if outer_guard.direction != GuardDirection::Increasing || !outer_guard.is_strict {
        return None;
    }
    if inner_guard.direction != GuardDirection::Increasing || !inner_guard.is_strict {
        return None;
    }

    // locate induction parameter indices
    let outer_param_index = block_param_index(tree.get(outer_header), outer_guard.induction)?;
    let inner_param_index = block_param_index(tree.get(inner_header), inner_guard.induction)?;

    // locate the inner header parameter that carries the outer induction
    let entry_block = inner_preheader.unwrap_or(outer_header);
    let entry_block_data = tree.get(entry_block);
    let entry_terminator = tree.get(entry_block_data.terminator);
    let entry_args = terminator_arguments_for_successor(entry_terminator, inner_header);
    let outer_entry_index = entry_args.iter().position(|&arg| {
        arg.value()
            .is_some_and(|arg| forwarding.resolve(arg) == forwarding.resolve(outer_guard.induction))
    })?;
    let inner_outer_param = tree
        .get(inner_header)
        .parameters
        .get(outer_entry_index)?
        .value
        .value()?;

    // locate the outer latch parameter carrying the induction
    let inner_header_block = tree.get(inner_header);
    let inner_header_terminator = tree.get(inner_header_block.terminator);
    let exit_args = terminator_arguments_for_successor(inner_header_terminator, inner_exit);
    let outer_latch_index = exit_args.iter().position(|&arg| {
        arg.value()
            .is_some_and(|arg| forwarding.resolve(arg) == forwarding.resolve(inner_outer_param))
    })?;
    let outer_latch_param = tree
        .get(inner_exit)
        .parameters
        .get(outer_latch_index)?
        .value
        .value()?;

    // compute outer step from scev or latch
    let outer_step = outer_step_for_guard(outer_index, outer_guard.induction, scev)
        .or_else(|| {
            outer_step_from_latch(
                outer_header,
                outer_latch,
                outer_param_index,
                outer_guard.induction,
                function,
                tree,
                forwarding,
            )
        })
        .or_else(|| {
            outer_step_from_latch(
                outer_header,
                outer_latch,
                outer_param_index,
                outer_latch_param,
                function,
                tree,
                forwarding,
            )
        })?;
    if outer_step <= 0 {
        return None;
    }

    // compute outer trip count from scev or header arguments
    let mut trip_count =
        trip_count_for_guard(&outer_guard, outer_index, scev, forwarding, function, tree)
            .unwrap_or(0);
    if trip_count == 0
        && let Some(fallback) = trip_count_from_header(
            &outer_guard,
            outer,
            outer_header,
            outer_param_index,
            outer_step,
            cfg,
            function,
            tree,
            forwarding,
        )
    {
        trip_count = fallback;
    }
    if trip_count == 0 {
        return None;
    }

    // collect inner loop values equivalent to the outer induction
    let outer_equivalents = outer_equivalent_values(
        inner,
        tree,
        forwarding,
        outer_guard.induction,
        inner_outer_param,
    );

    // reject loops with outer dependent derived values in the inner body
    if !inner_body_is_jammable(
        outer,
        inner,
        inner_guard.induction,
        inner_outer_param,
        outer_guard.induction,
        function,
        tree,
        domtree,
    ) {
        return None;
    }

    Some(JamCandidate {
        outer_header,
        outer_latch,
        outer_blocks: outer.blocks.clone(),
        inner_header,
        inner_latch,
        outer_induction: outer_guard.induction,
        outer_param_index,
        outer_latch_param,
        outer_step,
        outer_trip_count: trip_count,
        inner_param_index,
        inner_outer_param,
        outer_equivalents,
    })
}

/// Check whether the outer loop body is perfectly nested around the inner loop.
fn outer_is_perfectly_nested(
    outer: &Loop,
    inner: &Loop,
    inner_preheader: Option<mir::LocalNodeId<mir::Block>>,
) -> bool {
    // collect outer blocks not in the inner loop
    let mut extras: HashSet<_> = outer
        .blocks
        .iter()
        .copied()
        .filter(|block| !inner.blocks.contains(block))
        .collect();

    // drop the optional inner preheader from the extras
    if let Some(preheader) = inner_preheader {
        extras.remove(&preheader);
    }

    // drop the outer header and latch from the extras
    extras.remove(&outer.header);
    if let Some(latch) = outer.latches.first() {
        extras.remove(latch);
    }

    extras.is_empty()
}

/// Return the parameter index for a given block parameter value.
fn block_param_index(block: &mir::Block, value: mir::Value) -> Option<usize> {
    // locate the matching parameter
    block
        .parameters
        .iter()
        .position(|param| param.value.value() == Some(value))
}

/// Collect inner loop values that forward to the outer induction.
fn outer_equivalent_values(
    inner: &Loop,
    tree: &mir::NodeTree,
    forwarding: &BlockParamForwarding,
    outer_induction: mir::Value,
    inner_outer_param: mir::Value,
) -> Vec<mir::Value> {
    // resolve canonical forwarding roots
    let outer_root = forwarding.resolve(outer_induction);
    let inner_root = forwarding.resolve(inner_outer_param);
    let mut values = HashSet::new();

    // collect forwarded parameters in the inner loop
    for &block_id in &inner.blocks {
        let block = tree.get(block_id);
        for param in &block.parameters {
            let Some(value) = param.value.value() else {
                continue;
            };
            let resolved = forwarding.resolve(value);
            if resolved == outer_root || resolved == inner_root {
                values.insert(value);
            }
        }
    }

    // seed the explicit induction values
    values.insert(outer_induction);
    values.insert(inner_outer_param);

    // return stable ordering for deterministic output
    let mut ordered: Vec<_> = values.into_iter().collect();
    ordered.sort();
    ordered
}

/// Return the outer loop step for the induction value.
fn outer_step_for_guard(
    loop_index: usize,
    induction: mir::Value,
    scev: &ScalarEvolution,
) -> Option<i128> {
    // read the scalar evolution recurrence
    let scev_expr = scev.scev_for_value_in_loop(loop_index, induction)?;
    let (start, step) = match scev_expr {
        Scev::AddRec { start, step, .. } => (start.as_ref(), step.as_ref()),
        _ => return None,
    };

    // extract the constant step
    let _ = start;
    scev_constant(step).and_then(constant_to_i128)
}

/// Resolve the induction step by inspecting the latch update.
fn outer_step_from_latch(
    header: mir::LocalNodeId<mir::Block>,
    latch: mir::LocalNodeId<mir::Block>,
    param_index: usize,
    induction: mir::Value,
    function: &mir::Function,
    tree: &mir::NodeTree,
    forwarding: &BlockParamForwarding,
) -> Option<i128> {
    // read the latch argument for the induction parameter
    let latch_block = tree.get(latch);
    let latch_terminator = tree.get(latch_block.terminator);
    let args = terminator_arguments_for_successor(latch_terminator, header);
    let update_value = args.get(param_index)?.value()?;
    let update_value = forwarding.resolve(update_value);
    let induction = forwarding.resolve(induction);

    // require an updated induction value
    if update_value == induction {
        return None;
    }

    // locate the defining instruction
    let definitions = ValueDefinitions::build(function, tree);
    let instruction_id = definitions.definition_for(update_value)?;
    let instruction = tree.get(instruction_id);

    // decode the step from a simple add or sub
    let (operator, left, right) = match instruction {
        mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } => (*operator, left.value()?, right.value()?),
        _ => return None,
    };

    let left = forwarding.resolve(left);
    let right = forwarding.resolve(right);

    match operator {
        mir::BinaryOperator::Add if left == induction => {
            constant_value_for(right, function, tree, forwarding)
                .and_then(|constant| constant_to_i128(&constant))
        }
        mir::BinaryOperator::Add if right == induction => {
            constant_value_for(left, function, tree, forwarding)
                .and_then(|constant| constant_to_i128(&constant))
        }
        mir::BinaryOperator::Subtract if left == induction => {
            constant_value_for(right, function, tree, forwarding)
                .and_then(|constant| constant_to_i128(&constant))
                .map(|value| -value)
        }
        _ => None,
    }
}

/// Resolve a trip count using the loop header arguments.
// allow many arguments to keep trip count extraction explicit
#[allow(clippy::too_many_arguments)]
fn trip_count_from_header(
    guard: &GuardComparison,
    outer: &Loop,
    header: mir::LocalNodeId<mir::Block>,
    param_index: usize,
    step: i128,
    cfg: &ControlFlowGraph,
    function: &mir::Function,
    tree: &mir::NodeTree,
    forwarding: &BlockParamForwarding,
) -> Option<u64> {
    // find the unique predecessor outside the loop
    let mut outside_preds = cfg
        .predecessors(header)
        .iter()
        .copied()
        .filter(|pred| !outer.blocks.contains(pred));
    let entry_pred = outside_preds.next()?;
    if outside_preds.next().is_some() {
        return None;
    }

    // read the starting induction argument
    let entry_block = tree.get(entry_pred);
    let entry_terminator = tree.get(entry_block.terminator);
    let args = terminator_arguments_for_successor(entry_terminator, header);
    let start_value = args.get(param_index)?.value()?;
    let start_const = constant_value_for(start_value, function, tree, forwarding)?;
    let bound_const = constant_value_for(guard.bound, function, tree, forwarding)?;

    if guard.is_signed {
        let start = constant_to_i128(&start_const)?;
        let bound = constant_to_i128(&bound_const)?;
        trip_count_signed(start, bound, step, guard.is_strict, guard.direction)
    } else {
        let start = constant_to_u128(&start_const)?;
        let bound = constant_to_u128(&bound_const)?;
        let step = u128::try_from(step).ok()?;
        trip_count_unsigned(start, bound, step, guard.is_strict)
    }
}

/// Check if the inner loop body can be safely jammed.
// allow many arguments to keep jamming checks explicit
#[allow(clippy::too_many_arguments)]
fn inner_body_is_jammable(
    outer: &Loop,
    inner: &Loop,
    inner_induction: mir::Value,
    inner_outer_param: mir::Value,
    outer_induction: mir::Value,
    function: &mir::Function,
    tree: &mir::NodeTree,
    domtree: &DominatorTree,
) -> bool {
    // gather definition maps for dependency checks
    let def_maps = build_use_def_maps(function, tree);
    let def_map = build_value_definition_map(function, tree);

    // cache outer block parameters for dependency checks
    let outer_block_params: HashSet<_> = outer
        .blocks
        .iter()
        .filter(|block_id| !inner.blocks.contains(block_id))
        .flat_map(|block_id| {
            tree.get(*block_id)
                .parameters
                .iter()
                .filter_map(|param| param.value.value())
        })
        .collect();

    // locate the outer induction definition block
    let outer_def_block = match def_maps.def_block.get(&outer_induction) {
        Some(block) => *block,
        None => return false,
    };

    // reject when the induction does not dominate the inner header
    if !domtree.dominates(outer_def_block, inner.header) {
        return false;
    }

    // scan inner loop blocks for unsafe outer dependencies
    for &block_id in &inner.blocks {
        let block = tree.get(block_id);

        // check instruction uses
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if !inner_uses_are_safe(
                &instruction.uses(),
                inner,
                Some(inner_induction),
                Some(inner_outer_param),
                &outer_block_params,
                &def_maps.def_block,
                &def_map,
                outer_induction,
                tree,
            ) {
                return false;
            }

            // check externalized arguments
            if let Some(args) = instruction.argument_slice() {
                let arguments = tree.get_arguments(args);
                if !inner_uses_are_safe(
                    arguments,
                    inner,
                    Some(inner_induction),
                    Some(inner_outer_param),
                    &outer_block_params,
                    &def_maps.def_block,
                    &def_map,
                    outer_induction,
                    tree,
                ) {
                    return false;
                }
            }
        }

        // check terminator uses
        let terminator = tree.get(block.terminator);
        if !inner_uses_are_safe(
            &terminator.uses(),
            inner,
            Some(inner_induction),
            Some(inner_outer_param),
            &outer_block_params,
            &def_maps.def_block,
            &def_map,
            outer_induction,
            tree,
        ) {
            return false;
        }
    }

    true
}

/// Check whether a set of uses is safe for jamming.
// allow many arguments to keep dependency checks explicit
#[allow(clippy::too_many_arguments)]
fn inner_uses_are_safe(
    uses: &[mir::ValueReference],
    inner: &Loop,
    inner_induction: Option<mir::Value>,
    inner_outer_param: Option<mir::Value>,
    outer_block_params: &HashSet<mir::Value>,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    def_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    outer_induction: mir::Value,
    tree: &mir::NodeTree,
) -> bool {
    // cache header parameters for non induction checks
    let header_params: Vec<_> = tree
        .get(inner.header)
        .parameters
        .iter()
        .filter_map(|param| param.value.value())
        .collect();

    // scan each used value for unsafe dependencies
    for &value in uses {
        let Some(value) = value.value() else {
            continue;
        };

        // allow direct outer induction uses
        if value == outer_induction {
            continue;
        }

        // reject outer block parameters that are not explicitly allowed
        if outer_block_params.contains(&value) {
            if let Some(inner_induction) = inner_induction
                && value == inner_induction
            {
                continue;
            }

            if let Some(inner_outer_param) = inner_outer_param
                && value == inner_outer_param
            {
                continue;
            }

            return false;
        }

        // reject extra header parameters
        if header_params.contains(&value) {
            if let Some(inner_induction) = inner_induction
                && value == inner_induction
            {
                continue;
            }

            if let Some(inner_outer_param) = inner_outer_param
                && value == inner_outer_param
            {
                continue;
            }

            return false;
        }

        // reject values derived from the outer induction
        if value_depends_on(
            value,
            outer_induction,
            inner_outer_param,
            def_map,
            tree,
            &mut HashSet::new(),
        ) {
            return false;
        }

        // allow values defined within the inner loop
        if def_blocks
            .get(&value)
            .is_some_and(|block| inner.blocks.contains(block))
        {
            continue;
        }
    }

    true
}

/// Return true if a value depends on the outer induction value.
fn value_depends_on(
    value: mir::Value,
    outer_induction: mir::Value,
    inner_outer_param: Option<mir::Value>,
    def_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
    visiting: &mut HashSet<mir::Value>,
) -> bool {
    // treat the outer induction as a dependency root
    if value == outer_induction {
        return true;
    }

    // treat the inner outer parameter as a dependency root
    if let Some(inner_outer_param) = inner_outer_param
        && value == inner_outer_param
    {
        return true;
    }

    // avoid cycles
    if !visiting.insert(value) {
        return false;
    }

    // stop at values without defining instructions
    let Some(&instruction_id) = def_map.get(&value) else {
        return false;
    };

    let instruction = tree.get(instruction_id);

    // check instruction operands
    for use_value in instruction.uses() {
        let Some(use_value) = use_value.value() else {
            continue;
        };
        if value_depends_on(
            use_value,
            outer_induction,
            inner_outer_param,
            def_map,
            tree,
            visiting,
        ) {
            return true;
        }
    }

    // check externalized arguments
    if let Some(args) = instruction.argument_slice() {
        for &arg in tree.get_arguments(args) {
            let Some(arg) = arg.value() else {
                continue;
            };
            if value_depends_on(
                arg,
                outer_induction,
                inner_outer_param,
                def_map,
                tree,
                visiting,
            ) {
                return true;
            }
        }
    }

    false
}

/// Select an unroll and jam plan for the outer loop.
fn select_jam_plan(
    candidate: &JamCandidate,
    limits: &UnrollLimits,
    tree: &mir::NodeTree,
    allow_remainder: bool,
) -> Option<JamPlan> {
    // compute the inner body size
    let inner_body_size = tree.get(candidate.inner_latch).instructions.len();
    if inner_body_size == 0 {
        return None;
    }

    // cap factor by body size growth
    let max_factor_by_size = (limits.max_body_instructions / inner_body_size).max(1) as u64;
    let mut factor = limits
        .max_partial_unroll_factor
        .min(candidate.outer_trip_count)
        .min(max_factor_by_size);

    if factor < 2 {
        return None;
    }

    // allow remainders when peeling is supported
    if allow_remainder {
        let remainder = candidate.outer_trip_count % factor;
        return Some(JamPlan { factor, remainder });
    }

    // require exact divisibility when peeling is unavailable
    while factor >= 2 {
        if candidate.outer_trip_count.is_multiple_of(factor) {
            return Some(JamPlan {
                factor,
                remainder: 0,
            });
        }

        factor -= 1;
    }

    None
}

/// Apply loop unroll and jam for the candidate.
// allow extra arguments to keep loop state explicit
#[allow(clippy::too_many_arguments)]
fn unroll_and_jam_loop(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
    candidate: &JamCandidate,
    plan: JamPlan,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    value_types: &ValueTypeMap,
) -> bool {
    // reject degenerate factors
    if plan.factor < 2 {
        return false;
    }

    // peel remainder iterations before unroll and jam
    if plan.remainder > 0 {
        let peeled = peel_jam_remainder(function, tree, candidate, cfg, domtree, plan.remainder);
        if !peeled {
            return false;
        }
    }

    // locate the inner update instruction
    let def_map = build_value_definition_map(function, tree);
    let Some(update_info) = inner_update_info(candidate, tree, &def_map) else {
        return false;
    };

    // update the outer latch induction step
    if !rewrite_outer_latch_step(function, tree, ctx, candidate, plan.factor, value_types) {
        return false;
    }

    // duplicate the inner loop body for the unrolled outer iterations
    if !jam_inner_body(
        function,
        tree,
        ctx,
        candidate,
        plan.factor,
        &update_info,
        value_types,
    ) {
        return false;
    }

    true
}

/// Locate the outer loop preheader for unroll and jam.
fn find_jam_preheader(
    candidate: &JamCandidate,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::NodeTree,
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::ValueReference>)> {
    // collect predecessors outside the loop
    let mut outside_preds: Vec<_> = cfg
        .predecessors(candidate.outer_header)
        .iter()
        .copied()
        .filter(|pred| !candidate.outer_blocks.contains(pred))
        .collect();

    // require a single outside predecessor
    if outside_preds.len() != 1 {
        return None;
    }
    let preheader = outside_preds.pop()?;

    // ensure the preheader dominates the header
    if !domtree.dominates(preheader, candidate.outer_header) {
        return None;
    }

    // require a direct jump to the header
    let preheader_block = tree.get(preheader);
    let preheader_terminator = tree.get(preheader_block.terminator);
    let arguments = match preheader_terminator {
        mir::Terminator::Jump { target }
            if target.block.block() == Some(candidate.outer_header) =>
        {
            target.arguments.clone()
        }
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Peel remainder iterations before unroll and jam.
fn peel_jam_remainder(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    candidate: &JamCandidate,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    remainder: u64,
) -> bool {
    // find a preheader outside of the loop
    let Some((preheader, preheader_args)) = find_jam_preheader(candidate, cfg, domtree, tree)
    else {
        return false;
    };

    // clone iterations for the remainder
    let mut peeled_iterations = Vec::new();
    for _ in 0..remainder {
        // clone loop blocks and values
        let (block_map, value_map) = clone_loop_blocks(&candidate.outer_blocks, function, tree);

        // remap cloned terminators
        for &cloned_id in block_map.values() {
            let block = tree.get(cloned_id);
            let mut terminator = tree.get(block.terminator).clone();
            terminator_remap(&mut terminator, &block_map, &value_map);
            tree.replace(block.terminator, terminator);
        }

        // insert cloned blocks into the function
        let mut cloned_blocks: Vec<_> = block_map.values().copied().collect();
        cloned_blocks.sort();
        for block_id in cloned_blocks {
            function.blocks.push(block_id);
        }

        // record peeled header and latch
        peeled_iterations.push(UnrollIteration {
            header: block_map[&candidate.outer_header],
            latch: block_map[&candidate.outer_latch],
        });
    }

    // redirect the preheader to the first peeled header
    let Some(first_iteration) = peeled_iterations.first() else {
        return true;
    };
    let preheader_block = tree.get(preheader);
    let preheader_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: first_iteration.header.into(),
            arguments: preheader_args,
        },
    };
    tree.replace(preheader_block.terminator, preheader_terminator);

    // chain peeled iterations together
    for (index, iteration) in peeled_iterations.iter().enumerate() {
        let is_last = index + 1 == peeled_iterations.len();
        let next_header = if is_last {
            candidate.outer_header
        } else {
            peeled_iterations[index + 1].header
        };

        let mut latch_block = tree.get(iteration.latch).clone();
        let updated = rewrite_latch_to_jump(tree, &mut latch_block, iteration.header, next_header);
        if !updated {
            return false;
        }

        tree.replace(iteration.latch, latch_block);
    }

    true
}

/// Metadata about the inner loop update instruction.
struct InnerUpdateInfo {
    /// The update instruction id.
    update_instruction: mir::LocalNodeId<mir::Instruction>,
    /// Body instructions before the update.
    body_instructions: Vec<mir::LocalNodeId<mir::Instruction>>,
    /// Instructions after the update that can stay in the latch.
    trailing_instructions: Vec<mir::LocalNodeId<mir::Instruction>>,
}

/// Find the inner loop induction update instruction information.
fn inner_update_info(
    candidate: &JamCandidate,
    tree: &mir::NodeTree,
    def_map: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<InnerUpdateInfo> {
    // find the update value passed to the header
    let latch_block = tree.get(candidate.inner_latch);
    let latch_terminator = tree.get(latch_block.terminator);
    let update_value = terminator_arguments_for_successor(latch_terminator, candidate.inner_header)
        .get(candidate.inner_param_index)
        .and_then(|value| value.value())?;

    // locate the defining instruction
    let update_instruction = *def_map.get(&update_value)?;
    let update_index = latch_block
        .instructions
        .iter()
        .position(|&id| id == update_instruction)?;

    // split body and trailing instructions
    let body_instructions = latch_block.instructions[..update_index].to_vec();
    let trailing_instructions = latch_block.instructions[update_index + 1..].to_vec();
    if body_instructions.is_empty() {
        return None;
    }

    if !trailing_instructions.is_empty() {
        // reject trailing instructions that depend on outer induction values
        for instruction_id in &trailing_instructions {
            let instruction = tree.get(*instruction_id);
            if !instruction_is_speculatable(instruction, tree) {
                return None;
            }

            for value in instruction.uses() {
                let Some(value) = value.value() else {
                    continue;
                };
                if candidate.outer_equivalents.contains(&value) {
                    return None;
                }

                if value_depends_on(
                    value,
                    candidate.outer_induction,
                    Some(candidate.inner_outer_param),
                    def_map,
                    tree,
                    &mut HashSet::new(),
                ) {
                    return None;
                }
            }

            if let Some(arguments) = instruction.argument_slice() {
                for &value in tree.get_arguments(arguments) {
                    let Some(value) = value.value() else {
                        continue;
                    };
                    if candidate.outer_equivalents.contains(&value) {
                        return None;
                    }

                    if value_depends_on(
                        value,
                        candidate.outer_induction,
                        Some(candidate.inner_outer_param),
                        def_map,
                        tree,
                        &mut HashSet::new(),
                    ) {
                        return None;
                    }
                }
            }
        }
    }

    Some(InnerUpdateInfo {
        update_instruction,
        body_instructions,
        trailing_instructions,
    })
}

/// Rewrite the outer latch to advance by the unroll factor.
fn rewrite_outer_latch_step(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
    candidate: &JamCandidate,
    factor: u64,
    value_types: &ValueTypeMap,
) -> bool {
    // resolve the outer induction type
    let outer_type = value_types.require_value_type(candidate.outer_induction);

    let pointer_width_bits = ctx.type_context().pointer_width_bits;
    let scaled_constant = match scaled_step_constant(
        candidate.outer_step,
        factor,
        outer_type,
        tree,
        pointer_width_bits,
    ) {
        Some(constant) => constant,
        None => return false,
    };

    let mut latch_block = tree.get(candidate.outer_latch).clone();
    let latch_terminator = tree.get(latch_block.terminator);

    // read the current induction value from the latch arguments
    let mut arguments = match latch_terminator {
        mir::Terminator::Jump { target }
            if target.block.block() == Some(candidate.outer_header) =>
        {
            target.arguments.clone()
        }
        _ => return false,
    };
    if candidate.outer_param_index >= arguments.len() {
        return false;
    }
    let current_value = candidate.outer_latch_param;

    // build the scaled induction update
    let const_value = function.next_typed_value_like(current_value);
    let const_instruction = mir::Instruction::Const {
        destination: const_value.into(),
        value: scaled_constant,
    };
    let const_id = tree.insert(const_instruction);

    let updated_value = function.next_typed_value_like(current_value);
    let add_instruction = mir::Instruction::Binary {
        destination: updated_value.into(),
        operator: mir::BinaryOperator::Add,
        left: current_value.into(),
        right: const_value.into(),
    };
    let add_id = tree.insert(add_instruction);

    // append the update before the terminator
    latch_block.instructions.push(const_id);
    latch_block.instructions.push(add_id);

    arguments[candidate.outer_param_index] = updated_value.into();

    let new_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: candidate.outer_header.into(),
            arguments,
        },
    };
    tree.replace(latch_block.terminator, new_terminator);

    tree.replace(candidate.outer_latch, latch_block);

    true
}

/// Duplicate the inner loop body for the unrolled outer iterations.
fn jam_inner_body(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
    candidate: &JamCandidate,
    factor: u64,
    update_info: &InnerUpdateInfo,
    value_types: &ValueTypeMap,
) -> bool {
    // resolve the outer induction type
    let outer_type = value_types.require_value_type(candidate.outer_induction);

    let pointer_width_bits = ctx.type_context().pointer_width_bits;
    let mut new_instructions = Vec::new();
    new_instructions.extend(update_info.body_instructions.iter().copied());

    for offset in 1..factor {
        // materialize the outer induction value for this unrolled iteration
        let step_constant = match scaled_step_constant(
            candidate.outer_step,
            offset,
            outer_type,
            tree,
            pointer_width_bits,
        ) {
            Some(constant) => constant,
            None => return false,
        };

        let offset_const_value = function.next_typed_value_like(candidate.inner_outer_param);
        let offset_const_instruction = mir::Instruction::Const {
            destination: offset_const_value.into(),
            value: step_constant,
        };
        let offset_const_id = tree.insert(offset_const_instruction);
        new_instructions.push(offset_const_id);

        let offset_value = function.next_typed_value_like(candidate.inner_outer_param);
        let offset_add_instruction = mir::Instruction::Binary {
            destination: offset_value.into(),
            operator: mir::BinaryOperator::Add,
            left: candidate.inner_outer_param.into(),
            right: offset_const_value.into(),
        };
        let offset_add_id = tree.insert(offset_add_instruction);
        new_instructions.push(offset_add_id);

        // clone body instructions with remapped values
        let mut value_map = HashMap::new();
        for &value in &candidate.outer_equivalents {
            value_map.insert(value, offset_value);
        }

        for &instruction_id in &update_info.body_instructions {
            let instruction = tree.get(instruction_id).clone();

            if let Some(destination) = instruction.destination() {
                let Some(destination) = destination.value() else {
                    continue;
                };
                let new_value = function.next_typed_value_like(destination);
                value_map.insert(destination, new_value);
            }

            let cloned = instruction_map(&instruction, &value_map, tree);
            let cloned_id = tree.insert(cloned);
            clone_instruction_metadata(tree, instruction_id, cloned_id, &value_map);
            new_instructions.push(cloned_id);
        }
    }

    new_instructions.push(update_info.update_instruction);
    new_instructions.extend(update_info.trailing_instructions.iter().copied());

    let mut latch_block = tree.get(candidate.inner_latch).clone();
    latch_block.instructions = new_instructions;
    tree.replace(candidate.inner_latch, latch_block);

    true
}

/// Create a scaled induction step constant for the given type.
fn scaled_step_constant(
    step: i128,
    factor: u64,
    type_id: mir::LocalNodeId<mir::Type>,
    tree: &mir::NodeTree,
    pointer_width_bits: u16,
) -> Option<mir::Constant> {
    let scaled = step.checked_mul(factor as i128)?;

    match tree.get(type_id) {
        mir::Type::Int {
            width,
            is_signed: signed,
        } => {
            let width = *width;
            if *signed {
                let value = i64::try_from(scaled).ok()?;
                Some(mir::Constant::Int {
                    value,
                    width: width as u8,
                    is_signed: true,
                })
            } else {
                let value = u64::try_from(scaled).ok()?;
                Some(mir::Constant::UInt {
                    value,
                    width: width as u8,
                })
            }
        }
        mir::Type::Isize => {
            let value = i64::try_from(scaled).ok()?;
            Some(mir::Constant::Int {
                value,
                width: pointer_width_bits as u8,
                is_signed: true,
            })
        }
        mir::Type::Usize => {
            let value = u64::try_from(scaled).ok()?;
            Some(mir::Constant::UInt {
                value,
                width: pointer_width_bits as u8,
            })
        }
        _ => None,
    }
}

/// Compute unroll limits based on block hotness.
fn unroll_limits_for_loop(
    header: mir::LocalNodeId<mir::Block>,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
    entry_count: u64,
    policy: &CallsiteHotnessPolicy,
    unroll_threshold: usize,
) -> Option<UnrollLimits> {
    // default to base limits when no profile data exists
    if block_counts.is_empty() {
        return Some(UnrollLimits {
            max_full_unroll_iterations: MAX_FULL_UNROLL_ITERATIONS,
            max_partial_unroll_factor: MAX_PARTIAL_UNROLL_FACTOR,
            max_partial_unroll_trip_count: MAX_PARTIAL_UNROLL_TRIP_COUNT,
            max_body_instructions: unroll_threshold,
        });
    }

    // classify loop hotness from the header count
    let header_count = block_counts.get(&header).copied().unwrap_or(0);
    let hotness = block_hotness_from_counts(header_count, entry_count, policy);
    if matches!(hotness, CallsiteHotness::Cold) {
        return None;
    }

    // estimate average iterations from header and entry counts
    let avg_iterations = if entry_count > 0 {
        (header_count / entry_count).max(1)
    } else {
        1
    };

    let iteration_boost: u64 = match avg_iterations {
        0..=3 => 1,
        4..=7 => 2,
        8..=15 => 3,
        _ => 4,
    };

    let hotness_boost: u64 = if matches!(hotness, CallsiteHotness::Hot) {
        2
    } else {
        1
    };

    let scale = iteration_boost.saturating_mul(hotness_boost);

    Some(UnrollLimits {
        max_full_unroll_iterations: MAX_FULL_UNROLL_ITERATIONS.saturating_mul(scale),
        max_partial_unroll_factor: MAX_PARTIAL_UNROLL_FACTOR.saturating_mul(scale),
        max_partial_unroll_trip_count: MAX_PARTIAL_UNROLL_TRIP_COUNT.saturating_mul(scale),
        max_body_instructions: unroll_threshold.saturating_mul(scale as usize),
    })
}

/// Select unroll mode based on trip count and thresholds.
fn select_unroll_mode(candidate: &UnrollCandidate, limits: &UnrollLimits) -> Option<UnrollMode> {
    // allow full unroll when the trip count is small
    if candidate.trip_count <= limits.max_full_unroll_iterations {
        return Some(UnrollMode::Full {
            trip_count: candidate.trip_count,
        });
    }

    // reject partial unroll for large trip counts
    if candidate.trip_count > limits.max_partial_unroll_trip_count {
        return None;
    }

    // choose a factor based on trip count
    let factor = limits.max_partial_unroll_factor.min(candidate.trip_count);
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
            let block = tree.get(cloned_id);
            let mut terminator = tree.get(block.terminator).clone();
            terminator_remap(&mut terminator, &block_map, &value_map);
            tree.replace(block.terminator, terminator);
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
            tree,
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
            let block = tree.get(cloned_id);
            let mut terminator = tree.get(block.terminator).clone();
            terminator_remap(&mut terminator, &block_map, &value_map);
            tree.replace(block.terminator, terminator);
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
    let preheader_block = tree.get(preheader);
    let preheader_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: first_iteration.header.into(),
            arguments: preheader_args,
        },
    };
    tree.replace(preheader_block.terminator, preheader_terminator);

    // chain peeled iterations together
    for (index, iteration) in peeled_iterations.iter().enumerate() {
        let is_last = index + 1 == peeled_iterations.len();
        let next_header = if is_last {
            candidate.header
        } else {
            peeled_iterations[index + 1].header
        };

        let mut latch_block = tree.get(iteration.latch).clone();
        let updated = rewrite_latch_to_jump(tree, &mut latch_block, iteration.header, next_header);
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
) -> Option<(mir::LocalNodeId<mir::Block>, Vec<mir::ValueReference>)> {
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
    let preheader_terminator = tree.get(preheader_block.terminator);
    let arguments = match preheader_terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(candidate.header) => {
            target.arguments.clone()
        }
        _ => return None,
    };

    Some((preheader, arguments))
}

/// Rewrite a latch to unconditionally jump to the next header.
fn rewrite_latch_to_jump(
    tree: &mut mir::NodeTree,
    block: &mut mir::Block,
    header: mir::LocalNodeId<mir::Block>,
    next_header: mir::LocalNodeId<mir::Block>,
) -> bool {
    // locate the loop backedge arguments
    let terminator = tree.get(block.terminator).clone();
    let latch_has_edge = terminator
        .successors()
        .iter()
        .any(|successor| successor.block() == Some(header));
    if !latch_has_edge {
        return false;
    }
    let latch_args = terminator_arguments_for_successor(&terminator, header);

    // replace the latch terminator with a jump
    let new_terminator = mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: next_header.into(),
            arguments: latch_args.to_vec(),
        },
    };
    tree.replace(block.terminator, new_terminator);

    true
}

/// Rewrite an exiting block for a specific unrolled iteration.
fn rewrite_latch_block(
    tree: &mut mir::NodeTree,
    block: &mut mir::Block,
    candidate: &UnrollCandidate,
    iteration: &UnrollIteration,
    next_iteration: Option<&UnrollIteration>,
    mode: UnrollMode,
    is_last: bool,
) -> bool {
    let terminator = tree.get(block.terminator).clone();

    // extract latch arguments
    let latch_has_edge = terminator
        .successors()
        .iter()
        .any(|successor| successor.block() == Some(iteration.header));
    if !latch_has_edge {
        return false;
    }
    let latch_args = terminator_arguments_for_successor(&terminator, iteration.header);

    // handle non last iterations
    if !is_last {
        let Some(next) = next_iteration else {
            return false;
        };

        // redirect to the next iteration header
        let new_terminator = mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: next.header.into(),
                arguments: latch_args.to_vec(),
            },
        };
        tree.replace(block.terminator, new_terminator);

        return true;
    }

    // handle full unroll last iteration
    if matches!(mode, UnrollMode::Full { .. }) {
        // resolve exit arguments for this iteration
        let exit_arguments = if candidate.guard_at_latch {
            let Some((exit_args, _)) = guard_exit_arguments(&terminator, candidate.in_loop_is_then)
            else {
                return false;
            };
            exit_args
        } else {
            latch_args.to_vec()
        };

        let new_terminator = mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: candidate.exit_block.into(),
                arguments: exit_arguments,
            },
        };
        tree.replace(block.terminator, new_terminator);

        return true;
    }

    // handle partial unroll with header guards
    if !candidate.guard_at_latch {
        let new_terminator = mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: candidate.header.into(),
                arguments: latch_args.to_vec(),
            },
        };
        tree.replace(block.terminator, new_terminator);

        return true;
    }

    // handle partial unroll last iteration by redirecting the guard
    let (condition, then_target, else_target) = match terminator {
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => (condition, then_target, else_target),
        _ => return false,
    };

    let (then_arguments, else_arguments) = if candidate.in_loop_is_then {
        (then_target.arguments.clone(), else_target.arguments.clone())
    } else {
        (else_target.arguments.clone(), then_target.arguments.clone())
    };

    let new_terminator = mir::Terminator::Branch {
        condition,
        then_target: mir::BlockTarget {
            block: if candidate.in_loop_is_then {
                candidate.header.into()
            } else {
                candidate.exit_block.into()
            },
            arguments: then_arguments,
        },
        else_target: mir::BlockTarget {
            block: if candidate.in_loop_is_then {
                candidate.exit_block.into()
            } else {
                candidate.header.into()
            },
            arguments: else_arguments,
        },
    };
    tree.replace(block.terminator, new_terminator);

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
        } => (*operator, left.value()?, right.value()?),
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
        mir::Instruction::GlobalConst { global, .. } => {
            constant_from_global(global.global()?, tree)
        }
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
                    if let Some(destination) = destination.value() {
                        definitions.insert(destination, instruction_id);
                    }
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
    terminator: &mir::Terminator,
    in_loop_is_then: bool,
) -> Option<(Vec<mir::ValueReference>, Vec<mir::ValueReference>)> {
    // branch is required for unroll
    let (then_arguments, else_arguments) = match terminator {
        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => (then_target.arguments.clone(), else_target.arguments.clone()),
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
    use destack_mir as mir;

    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::{LoopSimplify, LoopUnroll, LoopUnrollAndJam};

    /// Fully unroll a loop with a small constant trip count.
    #[test]
    fn test_full_unroll_small_trip_count() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 3int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b1(v6)
b3(v7: int32):
    return v7
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 3int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b4(v6)
b3(v7: int32):
    return v7
b4(v8: int32):
    v9: boolean = int.lt.s v8, v1
    branch v9, b5(v8), b3(v8)
b5(v10: int32):
    v11: int32 = int.add v10, v2
    jump b6(v11)
b6(v12: int32):
    v13: boolean = int.lt.s v12, v1
    branch v13, b7(v12), b3(v12)
b7(v14: int32):
    v15: int32 = int.add v14, v2
    jump b3(v15)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnroll);
        test.assert_output(expected);
    }

    /// Fully unroll a decreasing loop with constant trip count.
    #[test]
    fn test_full_unroll_decreasing_trip_count() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 3int32
    v1: int32 = 0int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.gt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.sub v5, v2
    jump b1(v6)
b3(v7: int32):
    return v7
}"#;

        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 3int32
    v1: int32 = 0int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.gt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.sub v5, v2
    jump b4(v6)
b3(v7: int32):
    return v7
b4(v8: int32):
    v9: boolean = int.gt.s v8, v1
    branch v9, b5(v8), b3(v8)
b5(v10: int32):
    v11: int32 = int.sub v10, v2
    jump b6(v11)
b6(v12: int32):
    v13: boolean = int.gt.s v12, v1
    branch v13, b7(v12), b3(v12)
b7(v14: int32):
    v15: int32 = int.sub v14, v2
    jump b3(v15)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnroll);
        test.assert_output(expected);
    }

    /// Do not unroll loops without constant trip count.
    #[test]
    fn test_unroll_requires_constant_trip_count() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    jump b1(v1)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v0
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b1(v6)
b3(v7: int32):
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnroll);
        test.assert_output(input);
    }

    /// Partially unroll loops with remainder by peeling iterations.
    #[test]
    fn test_partial_unroll_with_remainder() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 10int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: int32 = int.add v3, v2
    v5: boolean = int.lt.s v4, v1
    branch v5, b1(v4), b2(v4)
b2(v6: int32):
    return v6
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 10int32
    v2: int32 = 1int32
    jump b3(v0)
b1(v3: int32):
    v4: int32 = int.add v3, v2
    v5: boolean = int.lt.s v4, v1
    jump b5(v4)
b2(v6: int32):
    return v6
b3(v7: int32):
    v8: int32 = int.add v7, v2
    v9: boolean = int.lt.s v8, v1
    jump b4(v8)
b4(v10: int32):
    v11: int32 = int.add v10, v2
    v12: boolean = int.lt.s v11, v1
    jump b1(v11)
b5(v13: int32):
    v14: int32 = int.add v13, v2
    v15: boolean = int.lt.s v14, v1
    jump b6(v14)
b6(v16: int32):
    v17: int32 = int.add v16, v2
    v18: boolean = int.lt.s v17, v1
    jump b7(v17)
b7(v19: int32):
    v20: int32 = int.add v19, v2
    v21: boolean = int.lt.s v20, v1
    branch v21, b1(v20), b2(v20)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnroll);
        test.assert_output(expected);
    }

    /// Header guarded loops are partially unrolled with guard chaining.
    #[test]
    fn test_partial_unroll_header_guard() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 9int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b1(v6)
b3(v7: int32):
    return v7
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 9int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b4(v6)
b3(v7: int32):
    return v7
b4(v8: int32):
    v9: boolean = int.lt.s v8, v1
    branch v9, b5(v8), b3(v8)
b5(v10: int32):
    v11: int32 = int.add v10, v2
    jump b6(v11)
b6(v12: int32):
    v13: boolean = int.lt.s v12, v1
    branch v13, b7(v12), b3(v12)
b7(v14: int32):
    v15: int32 = int.add v14, v2
    jump b8(v15)
b8(v16: int32):
    v17: boolean = int.lt.s v16, v1
    branch v17, b9(v16), b3(v16)
b9(v18: int32):
    v19: int32 = int.add v18, v2
    jump b1(v19)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnroll);
        test.assert_output(expected);
    }

    /// Non unit stride loops can be unrolled when the trip count is constant.
    #[test]
    fn test_unroll_non_unit_stride() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 6int32
    v2: int32 = 2int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b1(v6)
b3(v7: int32):
    return v7
}"#;

        let expected = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 6int32
    v2: int32 = 2int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b4(v6)
b3(v7: int32):
    return v7
b4(v8: int32):
    v9: boolean = int.lt.s v8, v1
    branch v9, b5(v8), b3(v8)
b5(v10: int32):
    v11: int32 = int.add v10, v2
    jump b6(v11)
b6(v12: int32):
    v13: boolean = int.lt.s v12, v1
    branch v13, b7(v12), b3(v12)
b7(v14: int32):
    v15: int32 = int.add v14, v2
    jump b3(v15)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnroll);
        test.assert_output(expected);
    }

    /// Multiple exits prevent unrolling.
    #[test]
    fn test_unroll_skips_multiple_exits() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: int32 = 0int32
    v2: int32 = 4int32
    v3: int32 = 1int32
    jump b1(v1)
b1(v4: int32):
    v5: boolean = int.lt.s v4, v2
    branch v5, b2(v4), b5(v4)
b2(v6: int32):
    branch v0, b3(v6), b4(v6)
b3(v7: int32):
    v8: int32 = int.add v7, v3
    jump b1(v8)
b4(v9: int32):
    return v9
b5(v10: int32):
    return v10
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnroll);
        test.assert_output(input);
    }

    /// Cold profile blocks disable unrolling.
    #[test]
    fn test_unroll_skips_cold_profile() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 0int32
    v1: int32 = 3int32
    v2: int32 = 1int32
    jump b1(v0)
b1(v3: int32):
    v4: boolean = int.lt.s v3, v1
    branch v4, b2(v3), b3(v3)
b2(v5: int32):
    v6: int32 = int.add v5, v2
    jump b1(v6)
b3(v7: int32):
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);

        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let header = function.blocks[1];

        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, header, 1);

        test.run_pass_with_profile(&LoopUnroll, profile);
        test.assert_output(input);
    }

    /// Unroll and jam a perfectly nested loop.
    #[test]
    fn test_unroll_and_jam_nested_loop() {
        let input = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 2uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    jump b3(v5, v7)
b3(v8: uint32, v9: uint32):
    v10: boolean = int.lt.u v9, v3
    branch v10, b4(v8, v9), b5(v8)
b4(v11: uint32, v12: uint32):
    v13: ref<uint32, borrowed> = element.address v0, v12
    store v13, v11
    v14: uint32 = int.add v12, v4
    jump b3(v11, v14)
b5(v15: uint32):
    v16: uint32 = int.add v15, v4
    jump b1(v16)
b6:
    return
}"#;

        let expected = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 2uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    jump b3(v5, v7)
b3(v8: uint32, v9: uint32):
    v10: boolean = int.lt.u v9, v3
    branch v10, b4(v8, v9), b5(v8)
b4(v11: uint32, v12: uint32):
    v13: ref<uint32, borrowed> = element.address v0, v12
    store v13, v11
    v14: uint32 = 1uint32
    v15: uint32 = int.add v8, v14
    v16: ref<uint32, borrowed> = element.address v0, v12
    store v16, v15
    v17: uint32 = int.add v12, v4
    jump b3(v11, v17)
b5(v18: uint32):
    v19: uint32 = int.add v18, v4
    v20: uint32 = 2uint32
    v21: uint32 = int.add v18, v20
    jump b1(v21)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnrollAndJam);
        test.assert_output(expected);
    }

    /// Outer derived values block unroll and jam.
    #[test]
    fn test_unroll_and_jam_skips_outer_dependency() {
        let input = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    v8: uint32 = int.add v5, v4
    jump b3(v5, v7, v8)
b3(v9: uint32, v10: uint32, v11: uint32):
    v12: boolean = int.lt.u v10, v3
    branch v12, b4(v9, v10, v11), b5(v9)
b4(v13: uint32, v14: uint32, v15: uint32):
    v16: ref<uint32, borrowed> = element.address v0, v15
    store v16, v13
    v17: uint32 = int.add v14, v4
    jump b3(v13, v17, v15)
b5(v18: uint32):
    v19: uint32 = int.add v18, v4
    jump b1(v19)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let baseline = test.format();

        test.run_pass(&LoopUnrollAndJam);
        test.assert_output(&baseline);
    }

    /// Inner update instructions must be last for unroll and jam.
    #[test]
    fn test_unroll_and_jam_skips_inner_update_not_last() {
        let input = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 4uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    jump b3(v5, v7)
b3(v8: uint32, v9: uint32):
    v10: boolean = int.lt.u v9, v3
    branch v10, b4(v8, v9), b5(v8)
b4(v11: uint32, v12: uint32):
    v13: ref<uint32, borrowed> = element.address v0, v12
    store v13, v11
    v14: uint32 = int.add v12, v4
    v15: uint32 = int.add v11, v4
    jump b3(v15, v14)
b5(v16: uint32):
    v17: uint32 = int.add v16, v4
    jump b1(v17)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        let baseline = test.format();

        test.run_pass(&LoopUnrollAndJam);
        test.assert_output(&baseline);
    }

    /// Trailing invariant updates after the inner step are preserved.
    #[test]
    fn test_unroll_and_jam_allows_trailing_invariants() {
        let input = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 2uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    jump b3(v5, v7)
b3(v8: uint32, v9: uint32):
    v10: boolean = int.lt.u v9, v3
    branch v10, b4(v8, v9), b5(v8)
b4(v11: uint32, v12: uint32):
    v13: ref<uint32, borrowed> = element.address v0, v12
    store v13, v11
    v14: uint32 = int.add v12, v4
    v15: uint32 = int.add v14, v4
    jump b3(v11, v14)
b5(v16: uint32):
    v17: uint32 = int.add v16, v4
    jump b1(v17)
b6:
    return
}"#;

        let expected = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 2uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    jump b3(v5, v7)
b3(v8: uint32, v9: uint32):
    v10: boolean = int.lt.u v9, v3
    branch v10, b4(v8, v9), b5(v8)
b4(v11: uint32, v12: uint32):
    v13: ref<uint32, borrowed> = element.address v0, v12
    store v13, v11
    v14: uint32 = 1uint32
    v15: uint32 = int.add v8, v14
    v16: ref<uint32, borrowed> = element.address v0, v12
    store v16, v15
    v17: uint32 = int.add v12, v4
    v18: uint32 = int.add v17, v4
    jump b3(v11, v17)
b5(v19: uint32):
    v20: uint32 = int.add v19, v4
    v21: uint32 = 2uint32
    v22: uint32 = int.add v19, v21
    jump b1(v22)
b6:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnrollAndJam);
        test.assert_output(expected);
    }

    /// Loops with non divisible trip counts are jammed by peeling.
    #[test]
    fn test_unroll_and_jam_peels_remainder_trip_count() {
        let input = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 5uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b1(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    jump b3(v5, v7)
b3(v8: uint32, v9: uint32):
    v10: boolean = int.lt.u v9, v3
    branch v10, b4(v8, v9), b5(v8)
b4(v11: uint32, v12: uint32):
    v13: ref<uint32, borrowed> = element.address v0, v12
    store v13, v11
    v14: uint32 = int.add v12, v4
    jump b3(v11, v14)
b5(v15: uint32):
    v16: uint32 = int.add v15, v4
    jump b1(v16)
b6:
    return
}"#;
        let expected = r#"
function test(v0: uint32[8]): void {
b0(v0: uint32[8]):
    v1: uint32 = 0uint32
    v2: uint32 = 5uint32
    v3: uint32 = 2uint32
    v4: uint32 = 1uint32
    jump b7(v1)
b1(v5: uint32):
    v6: boolean = int.lt.u v5, v2
    branch v6, b2, b6
b2:
    v7: uint32 = 0uint32
    jump b3(v5, v7)
b3(v8: uint32, v9: uint32):
    v10: boolean = int.lt.u v9, v3
    branch v10, b4(v8, v9), b5(v8)
b4(v11: uint32, v12: uint32):
    v13: ref<uint32, borrowed> = element.address v0, v12
    store v13, v11
    v14: uint32 = 1uint32
    v15: uint32 = int.add v8, v14
    v16: ref<uint32, borrowed> = element.address v0, v12
    store v16, v15
    v17: uint32 = 2uint32
    v18: uint32 = int.add v8, v17
    v19: ref<uint32, borrowed> = element.address v0, v12
    store v19, v18
    v20: uint32 = 3uint32
    v21: uint32 = int.add v8, v20
    v22: ref<uint32, borrowed> = element.address v0, v12
    store v22, v21
    v23: uint32 = int.add v12, v4
    jump b3(v11, v23)
b5(v24: uint32):
    v25: uint32 = int.add v24, v4
    v26: uint32 = 4uint32
    v27: uint32 = int.add v24, v26
    jump b1(v27)
b6:
    return
b7(v28: uint32):
    v29: boolean = int.lt.u v28, v2
    branch v29, b8, b6
b8:
    v30: uint32 = 0uint32
    jump b9(v28, v30)
b9(v31: uint32, v32: uint32):
    v33: boolean = int.lt.u v32, v3
    branch v33, b10(v31, v32), b11(v31)
b10(v34: uint32, v35: uint32):
    v36: ref<uint32, borrowed> = element.address v0, v35
    store v36, v34
    v37: uint32 = int.add v35, v4
    jump b9(v34, v37)
b11(v38: uint32):
    v39: uint32 = int.add v38, v4
    jump b1(v39)
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoopSimplify);
        test.run_pass(&LoopUnrollAndJam);
        test.assert_output(expected);
    }
}
