use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree};
use crate::optimize::common::{
    CallsiteHotness, CallsiteHotnessPolicy, EdgeSplitPolicy, block_execution_counts,
    block_hotness_from_counts, block_parameters_used_outside_block,
    block_uses_available_in_predecessor, build_use_def_maps, clone_instruction_metadata,
    collect_reachable_blocks, ensure_edge_block, instruction_is_speculatable, instruction_map,
    scaled_profile_count, terminator_edges, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Reorder blocks based on profile hotness.
    ///
    /// Hot paths are laid out contiguously and cold blocks are placed last.
    ///
    /// ```mir
    /// function before(v0: boolean): int32 {
    /// b0(v0: boolean):
    ///     branch v0, b1, b2
    /// b2:
    ///     v1 = 2int32
    ///     return v1
    /// b1:
    ///     v2 = 1int32
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: boolean): int32 {
    /// b0(v0: boolean):
    ///     branch v0, b1, b2
    /// b1:
    ///     v2 = 1int32
    ///     return v2
    /// b2:
    ///     v1 = 2int32
    ///     return v1
    /// }
    /// ```
    #[pass(id = "cfg-layout")]
    pub CfgLayout,
    "Profile guided block layout"
}

impl FunctionPass for CfgLayout {
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

        // skip when no profile data is available
        let Some(profile) = ctx.profile() else {
            return AnalysisPreservation::all();
        };
        if profile.is_empty() {
            return AnalysisPreservation::all();
        }

        // compute a new layout
        let changed = run_cfg_layout(function, tree, entry, profile, ctx);

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "CfgLayout"
    }

    fn id(&self) -> &'static str {
        "cfg-layout"
    }
}

/// Maximum instructions to duplicate on hot edges.
const MAX_HOT_EDGE_DUP_INSTRUCTIONS: usize = 6;
/// Maximum predecessors to duplicate per hot block.
const MAX_HOT_EDGE_DUP_PREDECESSORS: usize = 4;
/// Ratio of total edge count required to duplicate all hot edges.
const HOT_EDGE_DUP_RATIO: f64 = 0.70;
/// Ratio of total edge count required to duplicate the hottest edge.
const HOT_EDGE_DUP_MIN_RATIO: f64 = 0.20;

/// Predecessor edge data for hot edge duplication.
#[derive(Debug, Clone)]
struct EdgePredecessor {
    /// The predecessor block.
    pred: mir::LocalNodeId<mir::Block>,
    /// The edge kind from the predecessor.
    edge_kind: mir::EdgeKind,
    /// Arguments passed to the target block.
    arguments: Vec<mir::ValueReference>,
    /// Profile count for this edge.
    count: u64,
}

/// Reorder blocks according to profile data.
fn run_cfg_layout(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    entry: mir::LocalNodeId<mir::Block>,
    profile: &mir::ProfileTable,
    ctx: &PipelineContext<'_>,
) -> bool {
    // derive block counts and hotness
    let hotness_policy = ctx.inline_hotness_policy();
    let mut block_counts = block_execution_counts(function, tree, Some(profile), hotness_policy);
    if block_counts.is_empty() {
        return false;
    }

    let entry_count = block_counts.get(&entry).copied().unwrap_or(0);
    let mut cold_blocks = classify_cold_blocks(&block_counts, entry_count, hotness_policy);
    cold_blocks.remove(&entry);

    // fetch required analyses
    let analyses = ctx.function_analyses(function, tree);
    let domtree = analyses.get::<DominatorTree>().clone();

    // duplicate hot edges into small blocks
    let duplicated = duplicate_hot_edges(
        function,
        tree,
        &domtree,
        profile,
        hotness_policy,
        &mut block_counts,
    );

    // rebuild cfg after duplication for cold edge outlining
    let cfg = ControlFlowGraph::build(function, tree);

    // outline hot to cold edges for layout
    let outlined = outline_cold_edges(function, tree, &cfg, &mut cold_blocks, &mut block_counts);

    // record reachability for stable layout
    let reachable = collect_reachable_blocks(function, tree, entry);
    let reachable_set: HashSet<_> = reachable.iter().copied().collect();

    // compute edge weights
    let edge_weights = compute_edge_weights(function, tree, profile, hotness_policy, &block_counts);

    // build a hot trace layout
    let mut placed = HashSet::new();
    let mut hot_order = Vec::new();
    let start_blocks = layout_start_blocks(entry, &reachable, &cold_blocks, &block_counts);

    // build traces from each start block
    for start in start_blocks {
        // skip cold or previously placed blocks
        if placed.contains(&start) || cold_blocks.contains(&start) {
            continue;
        }

        // walk the hottest successor chain
        let mut current = start;
        loop {
            // stop once the block is already placed
            if !placed.insert(current) {
                break;
            }

            // record the block in the hot trace
            hot_order.push(current);

            // pick the next hot successor to extend the trace
            let next = select_hot_successor(
                current,
                tree,
                &edge_weights,
                &block_counts,
                &cold_blocks,
                &placed,
                &reachable_set,
            );
            let Some(next) = next else {
                break;
            };
            current = next;
        }
    }

    // append remaining reachable blocks
    let mut warm_remaining = Vec::new();
    let mut cold_remaining = Vec::new();
    for block_id in &reachable {
        // skip blocks that are already placed
        if placed.contains(block_id) {
            continue;
        }

        // collect blocks into cold or warm buckets
        if cold_blocks.contains(block_id) {
            cold_remaining.push(*block_id);
        } else {
            warm_remaining.push(*block_id);
        }
    }

    // order remaining blocks by hotness
    sort_blocks_by_hotness(&mut warm_remaining, &block_counts);
    sort_blocks_by_hotness(&mut cold_remaining, &block_counts);

    // assemble the final block order
    let mut ordered = Vec::new();
    ordered.extend(hot_order);
    ordered.extend(warm_remaining);
    ordered.extend(cold_remaining);

    // append unreachable blocks in original order
    for block_id in &function.blocks {
        // keep unreachable blocks after reachable layout
        if !reachable_set.contains(block_id) {
            ordered.push(*block_id);
        }
    }

    // skip when the layout is unchanged
    if ordered == function.blocks {
        return outlined || duplicated;
    }

    function.blocks = ordered;
    true
}

/// Classify cold blocks using profile counts.
fn classify_cold_blocks(
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
    entry_count: u64,
    policy: &CallsiteHotnessPolicy,
) -> HashSet<mir::LocalNodeId<mir::Block>> {
    // collect blocks classified as cold
    let mut cold = HashSet::new();
    for (&block, &count) in block_counts {
        // record blocks below cold thresholds
        if matches!(
            block_hotness_from_counts(count, entry_count, policy),
            CallsiteHotness::Cold
        ) {
            cold.insert(block);
        }
    }

    cold
}

/// Outline cold edges by inserting cold edge blocks.
fn outline_cold_edges(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    cfg: &ControlFlowGraph,
    cold_blocks: &mut HashSet<mir::LocalNodeId<mir::Block>>,
    block_counts: &mut HashMap<mir::LocalNodeId<mir::Block>, u64>,
) -> bool {
    // track newly created edge blocks and changes
    let mut edge_blocks = HashMap::new();
    let mut changed = false;

    // scan edges from warm to cold blocks
    let block_ids = function.blocks.clone();
    for block_id in block_ids {
        // skip edges from cold blocks
        if cold_blocks.contains(&block_id) {
            continue;
        }

        // inspect successor edges for cold targets
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };

            if !cold_blocks.contains(&successor) {
                continue;
            }

            // insert an edge block for multi successor preds
            let edge_block = ensure_edge_block(
                block_id,
                successor,
                function,
                tree,
                cfg,
                &mut edge_blocks,
                EdgeSplitPolicy::PredecessorMultiSuccessor,
                &mut changed,
            );

            // record outlined blocks for layout
            if edge_block != block_id {
                cold_blocks.insert(edge_block);

                // mirror target counts for outlined blocks
                let target_count = block_counts.get(&successor).copied().unwrap_or(0);
                if target_count > 0 {
                    block_counts.insert(edge_block, target_count);
                }
            }
        }
    }

    changed
}

/// Duplicate hot edges into small blocks to improve fallthrough.
fn duplicate_hot_edges(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    domtree: &DominatorTree,
    profile: &mir::ProfileTable,
    policy: &CallsiteHotnessPolicy,
    block_counts: &mut HashMap<mir::LocalNodeId<mir::Block>, u64>,
) -> bool {
    // build definition metadata
    let use_def = build_use_def_maps(function, tree);
    let value_def_blocks = &use_def.def_block;

    // collect edge predecessors keyed by target
    let mut predecessors: HashMap<mir::LocalNodeId<mir::Block>, Vec<EdgePredecessor>> =
        HashMap::new();
    let scale = policy.scaling_policy();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        let mut record_edge = |edge_kind: mir::EdgeKind,
                               target: &mir::BlockTarget,
                               arguments: &[mir::ValueReference]| {
            let Some(target_block) = target.block.block() else {
                return;
            };

            let edge = mir::EdgeKey::new(block_id, edge_kind, target_block);
            let count = profile
                .edge_profile(&edge)
                .map(|edge| scaled_profile_count(edge.count, profile.source, &scale))
                .unwrap_or(0);

            predecessors
                .entry(target_block)
                .or_default()
                .push(EdgePredecessor {
                    pred: block_id,
                    edge_kind,
                    arguments: arguments.to_vec(),
                    count,
                });
        };

        match terminator {
            mir::Terminator::Jump { target } => {
                record_edge(mir::EdgeKind::Jump, target, &target.arguments);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                record_edge(
                    mir::EdgeKind::BranchThen,
                    then_target,
                    &then_target.arguments,
                );
                record_edge(
                    mir::EdgeKind::BranchElse,
                    else_target,
                    &else_target.arguments,
                );
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                record_edge(mir::EdgeKind::CheckSuccess, success, &success.arguments);
                record_edge(mir::EdgeKind::CheckFailure, failure, &failure.arguments);
            }
            _ => {}
        }
    }

    // duplicate small blocks on hot edges
    let mut changed = false;
    for (target, preds) in predecessors {
        // skip targets with a single predecessor
        if preds.len() <= 1 {
            continue;
        }

        // skip entry blocks
        if function.entry == Some(target) {
            continue;
        }

        // skip blocks without instructions
        let block = tree.get(target).clone();
        if block.instructions.is_empty() {
            continue;
        }

        // skip blocks with too many instructions
        if block.instructions.len() > MAX_HOT_EDGE_DUP_INSTRUCTIONS {
            continue;
        }

        // require simple terminators
        let block_terminator = tree.get(block.terminator);
        if !matches!(
            block_terminator,
            mir::Terminator::Return { .. } | mir::Terminator::Jump { .. }
        ) {
            continue;
        }

        // skip blocks with parameters used outside the block
        if block_parameters_used_outside_block(&block, &use_def.use_blocks, target) {
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

        let candidates = select_hot_edge_predecessors(&preds);
        if candidates.is_empty() {
            continue;
        }

        let mut safe_candidates: Vec<EdgePredecessor> = Vec::new();
        for pred in candidates {
            if pred.arguments.len() != block.parameters.len() {
                continue;
            }

            if !block_uses_available_in_predecessor(
                target,
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

        if safe_candidates.len() > MAX_HOT_EDGE_DUP_PREDECESSORS {
            continue;
        }

        // duplicate the block into each hot predecessor edge
        for pred in safe_candidates.clone() {
            if pred.pred == target {
                continue;
            }

            // build value map for parameters and new instruction values
            let mut value_map: HashMap<mir::Value, mir::Value> = HashMap::new();
            for (param, arg) in block.parameters.iter().zip(pred.arguments.iter()) {
                let Some(parameter) = param.value.value() else {
                    continue;
                };
                let Some(argument) = arg.value() else {
                    continue;
                };

                value_map.insert(parameter, argument);
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
            let block_terminator = tree.get(block.terminator);
            let new_terminator = terminator_substitute_uses(block_terminator, &value_map);

            // create the duplicated block
            let new_terminator_id = tree.insert(new_terminator);
            let mut new_block = mir::Block::new(new_terminator_id);
            new_block.instructions = new_instructions;

            let new_block_id = tree.insert(new_block);
            insert_block_after(function, pred.pred, new_block_id);

            // rewrite the predecessor edge to the duplicated block
            let pred_block = tree.get(pred.pred).clone();
            let pred_terminator = tree.get(pred_block.terminator);
            let Some(updated) =
                rewrite_hot_edge_target(pred_terminator, pred.edge_kind, target, new_block_id)
            else {
                continue;
            };

            tree.replace(pred_block.terminator, updated);

            // track hot block counts for layout ordering
            let new_count = if pred.count > 0 {
                pred.count
            } else {
                block_counts.get(&target).copied().unwrap_or(0)
            };
            if new_count > 0 {
                block_counts.insert(new_block_id, new_count);
            }

            changed = true;
        }
    }

    changed
}

/// Select hot edge predecessors based on profile ratios.
fn select_hot_edge_predecessors(predecessors: &[EdgePredecessor]) -> Vec<EdgePredecessor> {
    // collect total counts for the target
    let total_count: u64 = predecessors.iter().map(|pred| pred.count).sum();
    if total_count == 0 {
        return Vec::new();
    }

    // collect hot edges by ratio
    let mut hot_preds = Vec::new();
    for pred in predecessors {
        let ratio = pred.count as f64 / total_count as f64;
        if ratio >= HOT_EDGE_DUP_RATIO {
            hot_preds.push(pred.pred);
        }
    }

    if !hot_preds.is_empty() {
        return predecessors
            .iter()
            .filter(|pred| hot_preds.contains(&pred.pred))
            .cloned()
            .collect();
    }

    // select the hottest edge when it dominates enough
    let mut hottest: Option<(mir::LocalNodeId<mir::Block>, u64)> = None;
    for pred in predecessors {
        if hottest.map(|(_, count)| pred.count > count).unwrap_or(true) {
            hottest = Some((pred.pred, pred.count));
        }
    }

    let Some((hottest_pred, hottest_count)) = hottest else {
        return Vec::new();
    };
    let ratio = hottest_count as f64 / total_count as f64;
    if ratio < HOT_EDGE_DUP_MIN_RATIO {
        return Vec::new();
    }

    predecessors
        .iter()
        .filter(|pred| pred.pred == hottest_pred)
        .cloned()
        .collect()
}

/// Insert a block immediately after the predecessor.
fn insert_block_after(
    function: &mut mir::Function,
    predecessor: mir::LocalNodeId<mir::Block>,
    block: mir::LocalNodeId<mir::Block>,
) {
    // insert directly after the predecessor when it exists
    if let Some(index) = function.blocks.iter().position(|id| *id == predecessor) {
        function.blocks.insert(index + 1, block);
        return;
    }

    // fallback to appending when the predecessor is not found
    function.blocks.push(block);
}

/// Rewrite a hot edge target to the duplicated block.
fn rewrite_hot_edge_target(
    terminator: &mir::Terminator,
    edge_kind: mir::EdgeKind,
    target: mir::LocalNodeId<mir::Block>,
    new_target: mir::LocalNodeId<mir::Block>,
) -> Option<mir::Terminator> {
    // rewrite jump edge targets
    if let mir::Terminator::Jump {
        target: jump_target,
        ..
    } = terminator
        && matches!(edge_kind, mir::EdgeKind::Jump)
        && jump_target.block.block() == Some(target)
    {
        return Some(mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: new_target.into(),
                arguments: Vec::new(),
            },
        });
    }

    if let mir::Terminator::Branch {
        condition,
        then_target,
        else_target,
    } = terminator
    {
        return match edge_kind {
            mir::EdgeKind::BranchThen if then_target.block.block() == Some(target) => {
                Some(mir::Terminator::Branch {
                    condition: *condition,
                    then_target: mir::BlockTarget {
                        block: new_target.into(),
                        arguments: Vec::new(),
                    },
                    else_target: else_target.clone(),
                })
            }
            mir::EdgeKind::BranchElse if else_target.block.block() == Some(target) => {
                Some(mir::Terminator::Branch {
                    condition: *condition,
                    then_target: then_target.clone(),
                    else_target: mir::BlockTarget {
                        block: new_target.into(),
                        arguments: Vec::new(),
                    },
                })
            }
            _ => None,
        };
    }

    // rewrite check edge targets
    if let mir::Terminator::Check {
        constraint,
        success,
        failure,
    } = terminator
    {
        return match edge_kind {
            mir::EdgeKind::CheckSuccess if success.block.block() == Some(target) => {
                let mut updated_success = success.clone();
                updated_success.block = new_target.into();
                updated_success.arguments = Vec::new();
                Some(mir::Terminator::Check {
                    constraint: constraint.clone(),
                    success: updated_success,
                    failure: failure.clone(),
                })
            }
            mir::EdgeKind::CheckFailure if failure.block.block() == Some(target) => {
                let mut updated_failure = failure.clone();
                updated_failure.block = new_target.into();
                updated_failure.arguments = Vec::new();
                Some(mir::Terminator::Check {
                    constraint: constraint.clone(),
                    success: success.clone(),
                    failure: updated_failure,
                })
            }
            _ => None,
        };
    }

    None
}

/// Order layout seeds by hotness.
fn layout_start_blocks(
    entry: mir::LocalNodeId<mir::Block>,
    reachable: &[mir::LocalNodeId<mir::Block>],
    cold_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
) -> Vec<mir::LocalNodeId<mir::Block>> {
    // collect non cold reachable candidates
    let mut candidates: Vec<_> = reachable
        .iter()
        .copied()
        .filter(|block| !cold_blocks.contains(block) && *block != entry)
        .collect();

    // order candidates by hotness
    sort_blocks_by_hotness(&mut candidates, block_counts);

    // seed with the entry block
    let mut ordered = Vec::with_capacity(candidates.len() + 1);
    ordered.push(entry);
    ordered.extend(candidates);
    ordered
}

/// Pick the hottest successor for a block.
fn select_hot_successor(
    block: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    edge_weights: &HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>), u64>,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
    cold_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    placed: &HashSet<mir::LocalNodeId<mir::Block>>,
    reachable: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> Option<mir::LocalNodeId<mir::Block>> {
    // scan successors for the hottest candidate
    let block_id = block;
    let block = tree.get(block_id);
    let terminator = tree.get(block.terminator);
    let mut best: Option<(u64, u64, mir::LocalNodeId<mir::Block>)> = None;

    for successor in terminator.successors() {
        let Some(successor) = successor.block() else {
            continue;
        };

        // skip successors already placed or marked cold
        if placed.contains(&successor) || cold_blocks.contains(&successor) {
            continue;
        }

        // skip unreachable successors
        if !reachable.contains(&successor) {
            continue;
        }

        // compute edge and block weights
        let edge_key = (block_id, successor);
        let weight = edge_weights.get(&edge_key).copied().unwrap_or(0);
        let count = block_counts.get(&successor).copied().unwrap_or(0);

        // update the best candidate when hotter
        let candidate = (weight, count, successor);
        if best.is_none() || candidate > best.unwrap() {
            best = Some(candidate);
        }
    }

    best.map(|(_, _, successor)| successor)
}

/// Sort blocks by execution count and id.
fn sort_blocks_by_hotness(
    blocks: &mut [mir::LocalNodeId<mir::Block>],
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
) {
    // order by descending count with id as a tie breaker
    blocks.sort_by(|left, right| {
        // compute block counts for ordering
        let left_count = block_counts.get(left).copied().unwrap_or(0);
        let right_count = block_counts.get(right).copied().unwrap_or(0);

        // order by hotness then block id
        right_count.cmp(&left_count).then_with(|| left.cmp(right))
    });
}

/// Compute edge weights for layout decisions.
fn compute_edge_weights(
    function: &mir::Function,
    tree: &mir::Tree,
    profile: &mir::ProfileTable,
    policy: &CallsiteHotnessPolicy,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
) -> HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>), u64> {
    // derive edge scaling parameters
    let scale = policy.scaling_policy();
    let mut weights = HashMap::new();

    // compute a weight per edge using profile data when possible
    for &block_id in &function.blocks {
        // read terminator edge list
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        for (edge_key, target) in terminator_edges(block_id, terminator) {
            // prefer explicit edge profiles
            let edge_weight = if let Some(edge_profile) = profile.edge_profile(&edge_key) {
                scaled_profile_count(edge_profile.count, profile.source, &scale)
            } else {
                block_counts.get(&target).copied().unwrap_or(0)
            };

            // record the computed weight
            weights.insert((block_id, target), edge_weight);
        }
    }

    weights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Layout moves hot successors earlier.
    #[test]
    fn test_cfg_layout_orders_hot_path() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b2, b1
b1:
    v1: int32 = 2int32
    return v1
b2:
    v2: int32 = 1int32
    return v2
}"#;

        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[2];
        let block2 = function.blocks[1];

        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, block1, 90);
        test.record_block_profile(&mut profile, block2, 10);

        test.run_pass_with_profile(&CfgLayout, profile);
        test.assert_output(expected);
    }

    /// Layout preserves order without profile data.
    #[test]
    fn test_cfg_layout_skips_without_profile() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b2, b1
b1:
    v1: int32 = 2int32
    return v1
b2:
    v2: int32 = 1int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let baseline = test.format();

        test.run_pass(&CfgLayout);
        test.assert_output(&baseline);
    }

    /// Cold blocks are split to the end of the layout.
    #[test]
    fn test_cfg_layout_splits_cold_blocks() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b2, b1
b1:
    v1: int32 = 2int32
    return v1
b2:
    v2: int32 = 1int32
    return v2
}"#;

        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b3
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
b3:
    jump b2
}"#;

        let mut test = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[2];
        let block2 = function.blocks[1];

        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, block1, 80);
        test.record_block_profile(&mut profile, block2, 1);

        test.run_pass_with_profile(&CfgLayout, profile);
        test.assert_output(expected);
    }

    /// Switch blocks are ordered by hotness.
    #[test]
    fn test_cfg_layout_switch_orders_hot_blocks() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    switch v0, b1, 0 => b2
b1:
    v1: int32 = 2int32
    return v1
b2:
    v2: int32 = 1int32
    return v2
}"#;

        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    switch v0, b3, 0 => b1
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
b3:
    jump b2
}"#;

        let mut test = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block2 = function.blocks[1];
        let block1 = function.blocks[2];

        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, block1, 90);
        test.record_block_profile(&mut profile, block2, 5);

        test.run_pass_with_profile(&CfgLayout, profile);
        test.assert_output(expected);
    }

    /// Check terminators reorder blocks by hotness.
    #[test]
    fn test_cfg_layout_check_orders_hot_blocks() {
        let input = r#"
function test(v0: uint32, v1: uint32[8]): int32 {
b0(v0: uint32, v1: uint32[8]):
    v2: uint32 = 1uint32
    v3: boolean = int.lt.u v0, v2
    check bounds.u v0, v2, v1 -> b2, b1
b1:
    v4: int32 = 2int32
    return v4
b2:
    v5: int32 = 1int32
    return v5
}"#;

        let expected = r#"
function test(v0: uint32, v1: uint32[8]): int32 {
b0(v0: uint32, v1: uint32[8]):
    v2: uint32 = 1uint32
    v3: boolean = int.lt.u v0, v2
    check bounds.u v0, v2, v1 -> b1, b3
b1:
    v4: int32 = 1int32
    return v4
b2:
    v5: int32 = 2int32
    return v5
b3:
    jump b2
}"#;

        let mut test = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block2 = function.blocks[1];
        let block1 = function.blocks[2];

        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, block1, 90);
        test.record_block_profile(&mut profile, block2, 2);

        test.run_pass_with_profile(&CfgLayout, profile);
        test.assert_output(expected);
    }

    /// Edge profiles override block counts for trace selection.
    #[test]
    fn test_cfg_layout_prefers_edge_profiles() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
}"#;

        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b2, b1
b1:
    v1: int32 = 2int32
    return v1
b2:
    v2: int32 = 1int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, block1, 90);
        test.record_block_profile(&mut profile, block2, 10);

        test.record_edge_profile(&mut profile, entry, mir::EdgeKind::BranchThen, block1, 20);
        test.record_edge_profile(&mut profile, entry, mir::EdgeKind::BranchElse, block2, 80);

        test.run_pass_with_profile(&CfgLayout, profile);
        test.assert_output(expected);
    }

    /// Hot branch edges duplicate small targets.
    #[test]
    fn test_cfg_layout_duplicates_hot_edge() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b2, b1
b1:
    jump b2
b2:
    v1: int32 = 1int32
    return v1
}"#;

        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b3
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 1int32
    return v2
b3:
    jump b2
}"#;

        let mut test = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, block1, 20);
        test.record_block_profile(&mut profile, block2, 40);
        test.record_edge_profile(&mut profile, entry, mir::EdgeKind::BranchThen, block2, 80);
        test.record_edge_profile(&mut profile, entry, mir::EdgeKind::BranchElse, block1, 20);
        test.record_edge_profile(&mut profile, block1, mir::EdgeKind::Jump, block2, 20);

        test.run_pass_with_profile(&CfgLayout, profile);
        test.assert_output(expected);
    }

    /// Unreachable blocks are kept last.
    #[test]
    fn test_cfg_layout_preserves_unreachable_order() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b3
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 3int32
    return v2
b3:
    v3: int32 = 2int32
    return v3
}"#;

        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    return v1
b2:
    v2: int32 = 2int32
    return v2
b3:
    v3: int32 = 3int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = test.entry_function_id();
        let function = test.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[1];
        let block3 = function.blocks[2];
        let block2 = function.blocks[3];

        test.record_block_profile(&mut profile, entry, 100);
        test.record_block_profile(&mut profile, block1, 90);
        test.record_block_profile(&mut profile, block2, 10);
        test.record_block_profile(&mut profile, block3, 1);

        test.run_pass_with_profile(&CfgLayout, profile);
        test.assert_output(expected);
    }
}
