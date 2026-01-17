use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::common::{
    CallsiteHotness, block_execution_counts, block_hotness_from_counts, collect_reachable_blocks,
    scaled_profile_count, terminator_edges,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Reorder blocks based on profile hotness.
    ///
    /// Hot paths are laid out contiguously and cold blocks are placed last.
    ///
    /// ```mir
    /// function @before(v0: bool) -> i32 {
    /// block0(v0: bool):
    ///     branch v0, block1, block2
    /// block2:
    ///     v1 = iconst 2i32
    ///     return v1
    /// block1:
    ///     v2 = iconst 1i32
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: bool) -> i32 {
    /// block0(v0: bool):
    ///     branch v0, block1, block2
    /// block1:
    ///     v2 = iconst 1i32
    ///     return v2
    /// block2:
    ///     v1 = iconst 2i32
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
        tree: &mut mir::NodeTree,
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

/// Reorder blocks according to profile data.
fn run_cfg_layout(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
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

    // outline hot to cold edges for layout
    let cfg = ctx
        .function_analyses(function, tree)
        .get::<crate::optimize::analyses::ControlFlowGraph>()
        .clone();
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
        return outlined;
    }

    function.blocks = ordered;
    true
}

/// Classify cold blocks using profile counts.
fn classify_cold_blocks(
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
    entry_count: u64,
    policy: &crate::optimize::common::CallsiteHotnessPolicy,
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
    tree: &mut mir::NodeTree,
    cfg: &crate::optimize::analyses::ControlFlowGraph,
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
        let terminator = &tree.get(block_id).terminator;
        for successor in terminator.successors() {
            if !cold_blocks.contains(&successor) {
                continue;
            }

            // insert an edge block for multi successor preds
            let edge_block = crate::optimize::common::ensure_edge_block(
                block_id,
                successor,
                function,
                tree,
                cfg,
                &mut edge_blocks,
                crate::optimize::common::EdgeSplitPolicy::PredecessorMultiSuccessor,
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
    tree: &mir::NodeTree,
    edge_weights: &HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>), u64>,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
    cold_blocks: &HashSet<mir::LocalNodeId<mir::Block>>,
    placed: &HashSet<mir::LocalNodeId<mir::Block>>,
    reachable: &HashSet<mir::LocalNodeId<mir::Block>>,
) -> Option<mir::LocalNodeId<mir::Block>> {
    // scan successors for the hottest candidate
    let terminator = &tree.get(block).terminator;
    let mut best: Option<(u64, u64, mir::LocalNodeId<mir::Block>)> = None;

    for successor in terminator.successors() {
        // skip successors already placed or marked cold
        if placed.contains(&successor) || cold_blocks.contains(&successor) {
            continue;
        }

        // skip unreachable successors
        if !reachable.contains(&successor) {
            continue;
        }

        // compute edge and block weights
        let edge_key = (block, successor);
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
    tree: &mir::NodeTree,
    profile: &mir::ProfileTable,
    policy: &crate::optimize::common::CallsiteHotnessPolicy,
    block_counts: &HashMap<mir::LocalNodeId<mir::Block>, u64>,
) -> HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>), u64> {
    // derive edge scaling parameters
    let scale = policy.scaling_policy();
    let mut weights = HashMap::new();

    // compute a weight per edge using profile data when possible
    for &block_id in &function.blocks {
        // read terminator edge list
        let terminator = &tree.get(block_id).terminator;
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
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block2:
    v1 = iconst 2i32
    return v1
block1:
    v2 = iconst 1i32
    return v2
}"#;

        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v2 = iconst 1i32
    return v2
block2:
    v1 = iconst 2i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[2];
        let block2 = function.blocks[1];

        program.record_block_profile(&mut profile, entry, 100);
        program.record_block_profile(&mut profile, block1, 90);
        program.record_block_profile(&mut profile, block2, 10);

        program.run_pass_with_profile(&CfgLayout, profile);
        program.assert_output(expected);
    }

    /// Layout preserves order without profile data.
    #[test]
    fn test_cfg_layout_skips_without_profile() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block2:
    v1 = iconst 2i32
    return v1
block1:
    v2 = iconst 1i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        let baseline = program.format();

        program.run_pass(&CfgLayout);
        program.assert_output(&baseline);
    }

    /// Cold blocks are split to the end of the layout.
    #[test]
    fn test_cfg_layout_splits_cold_blocks() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block2:
    v2 = iconst 2i32
    return v2
block1:
    v1 = iconst 1i32
    return v1
}"#;

        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block3
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
block3:
    jump block2
}"#;

        let mut program = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[2];
        let block2 = function.blocks[1];

        program.record_block_profile(&mut profile, entry, 100);
        program.record_block_profile(&mut profile, block1, 80);
        program.record_block_profile(&mut profile, block2, 1);

        program.run_pass_with_profile(&CfgLayout, profile);
        program.assert_output(expected);
    }

    /// Switch blocks are ordered by hotness.
    #[test]
    fn test_cfg_layout_switch_orders_hot_blocks() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block2, 0 => block1
block2:
    v2 = iconst 2i32
    return v2
block1:
    v1 = iconst 1i32
    return v1
}"#;

        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    switch v0, block3, 0 => block1
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
block3:
    jump block2
}"#;

        let mut program = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block2 = function.blocks[1];
        let block1 = function.blocks[2];

        program.record_block_profile(&mut profile, entry, 100);
        program.record_block_profile(&mut profile, block1, 90);
        program.record_block_profile(&mut profile, block2, 5);

        program.run_pass_with_profile(&CfgLayout, profile);
        program.assert_output(expected);
    }

    /// Check terminators reorder blocks by hotness.
    #[test]
    fn test_cfg_layout_check_orders_hot_blocks() {
        let input = r#"function @test(v0: u32, v1: [u32; 8]) -> i32 {
block0(v0: u32, v1: [u32; 8]):
    v2 = iconst 1u32
    v3 = icmp_ult v0, v2
    check v3, bounds.unsigned v0, v2, v1, block1, block2
block2:
    v5 = iconst 2i32
    return v5
block1:
    v4 = iconst 1i32
    return v4
}"#;

        let expected = r#"function @test(v0: u32, v1: [u32; 8]) -> i32 {
block0(v0: u32, v1: [u32; 8]):
    v2 = iconst 1u32
    v3 = icmp_ult v0, v2
    check v3, bounds.unsigned v0, v2, v1, block1, block3
block1:
    v4 = iconst 1i32
    return v4
block2:
    v5 = iconst 2i32
    return v5
block3:
    jump block2
}"#;

        let mut program = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block2 = function.blocks[1];
        let block1 = function.blocks[2];

        program.record_block_profile(&mut profile, entry, 100);
        program.record_block_profile(&mut profile, block1, 90);
        program.record_block_profile(&mut profile, block2, 2);

        program.run_pass_with_profile(&CfgLayout, profile);
        program.assert_output(expected);
    }

    /// Edge profiles override block counts for trace selection.
    #[test]
    fn test_cfg_layout_prefers_edge_profiles() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
}"#;

        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block2, block1
block1:
    v2 = iconst 2i32
    return v2
block2:
    v1 = iconst 1i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        program.record_block_profile(&mut profile, entry, 100);
        program.record_block_profile(&mut profile, block1, 90);
        program.record_block_profile(&mut profile, block2, 10);

        program.record_edge_profile(&mut profile, entry, mir::EdgeKind::BranchThen, block1, 20);
        program.record_edge_profile(&mut profile, entry, mir::EdgeKind::BranchElse, block2, 80);

        program.run_pass_with_profile(&CfgLayout, profile);
        program.assert_output(expected);
    }

    /// Unreachable blocks are kept last.
    #[test]
    fn test_cfg_layout_preserves_unreachable_order() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block3:
    v3 = iconst 3i32
    return v3
block2:
    v2 = iconst 2i32
    return v2
}"#;

        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    return v1
block2:
    v2 = iconst 2i32
    return v2
block3:
    v3 = iconst 3i32
    return v3
}"#;

        let mut program = TestProgram::new(input);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let entry = function.entry.unwrap();
        let block1 = function.blocks[1];
        let block3 = function.blocks[2];
        let block2 = function.blocks[3];

        program.record_block_profile(&mut profile, entry, 100);
        program.record_block_profile(&mut profile, block1, 90);
        program.record_block_profile(&mut profile, block2, 10);
        program.record_block_profile(&mut profile, block3, 1);

        program.run_pass_with_profile(&CfgLayout, profile);
        program.assert_output(expected);
    }
}
