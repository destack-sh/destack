use std::collections::HashMap;

use destack_mir as mir;

/// Minimum execution count for a hot operation.
const HOT_COUNT: u64 = 50;
/// Maximum execution count for a cold operation.
const COLD_COUNT: u64 = 8;
/// Minimum caller relative count for a hot operation.
const HOT_RATIO: f64 = 0.10;
/// Maximum caller relative count for a cold operation.
const COLD_RATIO: f64 = 0.01;
/// Maximum unknown indirect call ratio accepted for a hot operation.
const UNKNOWN_RATIO_MAX: f64 = 0.25;

/// Hotness classification for a callsite under profile data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallsiteHotness {
    /// No profile data or no strong signal is available.
    Unknown,
    /// The callsite is hot.
    Hot,
    /// The callsite is cold or missing profile data.
    Cold,
}

/// Classify a callsite as hot or cold based on profile counts.
pub fn callsite_hotness(
    profile: Option<&mir::Profile>,
    caller: mir::LocalNodeId<mir::Function>,
    callsite: mir::CallSite,
) -> CallsiteHotness {
    // bail out when no profile data is present
    let Some(profile) = profile else {
        return CallsiteHotness::Unknown;
    };

    // reject callsites without profile data
    let Some(callsite_profile) = profile.callsite_profile(callsite) else {
        return CallsiteHotness::Cold;
    };

    // reject zero callsites
    let total_count = callsite_profile.total_count.get();
    if total_count == 0 {
        return CallsiteHotness::Cold;
    }

    // treat high unknown ratios as cold
    let unknown_count = callsite_profile.unknown_count.get();
    let unknown_ratio = unknown_count as f64 / total_count as f64;
    if unknown_ratio > UNKNOWN_RATIO_MAX {
        return CallsiteHotness::Cold;
    }

    // accept hot callsites based on absolute counts
    if total_count >= HOT_COUNT {
        return CallsiteHotness::Hot;
    }

    // reject cold callsites based on absolute counts
    if total_count <= COLD_COUNT {
        return CallsiteHotness::Cold;
    }

    // require caller entry counts for ratio based classification
    let Some(entry_count) = profile.function_count(caller) else {
        return CallsiteHotness::Unknown;
    };
    let entry_count = entry_count.get();

    // reject zero entry counts
    if entry_count == 0 {
        return CallsiteHotness::Cold;
    }

    // classify based on callsite ratio
    let ratio = total_count as f64 / entry_count as f64;
    if ratio >= HOT_RATIO {
        return CallsiteHotness::Hot;
    }

    if ratio <= COLD_RATIO {
        return CallsiteHotness::Cold;
    }

    CallsiteHotness::Unknown
}

/// Derive hotness from block execution counts.
pub fn block_hotness_from_counts(block_count: u64, entry_count: u64) -> CallsiteHotness {
    // guard against missing counts
    if block_count == 0 {
        return CallsiteHotness::Unknown;
    }

    // classify by absolute counts
    if block_count >= HOT_COUNT {
        return CallsiteHotness::Hot;
    }
    if block_count <= COLD_COUNT {
        return CallsiteHotness::Cold;
    }

    // fall back to ratio based classification
    if entry_count == 0 {
        return CallsiteHotness::Unknown;
    }

    // compare ratios against thresholds
    let ratio = block_count as f64 / entry_count as f64;
    if ratio >= HOT_RATIO {
        return CallsiteHotness::Hot;
    }
    if ratio <= COLD_RATIO {
        return CallsiteHotness::Cold;
    }

    CallsiteHotness::Unknown
}

/// Compute execution counts for blocks using profile data.
pub fn block_execution_counts(
    function: &mir::Function,
    tree: &mir::Tree,
    profile: Option<&mir::Profile>,
) -> HashMap<mir::LocalNodeId<mir::Block>, u64> {
    // return early without profiles
    let Some(profile) = profile else {
        return HashMap::new();
    };

    // seed counts from block profiles
    let mut counts = HashMap::new();
    for &block_id in &function.blocks {
        // read block profile counts when present
        let count = profile
            .block_count(block_id)
            .map(mir::Count::get)
            .unwrap_or(0);

        // keep only non zero counts
        if count > 0 {
            counts.insert(block_id, count);
        }
    }

    // fill in missing counts from incoming edges
    let incoming = incoming_edge_counts(function, tree, profile);
    for &block_id in &function.blocks {
        // skip blocks that already have counts
        if counts.contains_key(&block_id) {
            continue;
        }

        // use incoming edge totals when available
        let Some(count) = incoming.get(&block_id) else {
            continue;
        };
        if *count > 0 {
            counts.insert(block_id, *count);
        }
    }

    counts
}

/// Compute block entry counts from edge profiles.
fn incoming_edge_counts(
    function: &mir::Function,
    tree: &mir::Tree,
    profile: &mir::Profile,
) -> HashMap<mir::LocalNodeId<mir::Block>, u64> {
    // accumulate incoming counts per block
    let mut counts = HashMap::new();

    // scan blocks for profiled edges
    for &block_id in &function.blocks {
        // read the terminator edges for this block
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let edges = terminator_edges(block_id, terminator);

        // accumulate edge counts for each successor
        for (edge, target) in edges {
            // skip when no edge profile exists
            let Some(count) = profile.edge_count(&edge).map(mir::Count::get) else {
                continue;
            };

            // accumulate edge counts
            if count == 0 {
                continue;
            }

            // add to the incoming total
            let entry = counts.entry(target).or_insert(0u64);
            *entry = (*entry).saturating_add(count);
        }
    }

    counts
}

/// Enumerate edges for a terminator.
pub fn terminator_edges(
    source: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
) -> Vec<(mir::Edge, mir::LocalNodeId<mir::Block>)> {
    match terminator {
        mir::Terminator::Error => Vec::new(),
        mir::Terminator::Jump { target, .. } => {
            target.block.block().map_or_else(Vec::new, |target| {
                vec![(mir::Edge::new(source, mir::Successor::Jump, target), target)]
            })
        }
        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            let mut edges = Vec::with_capacity(2);

            // then edge
            if let Some(target) = then_target.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::BranchThen, target),
                    target,
                ));
            }

            // else edge
            if let Some(target) = else_target.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::BranchElse, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            let mut edges = Vec::with_capacity(2);

            // success edge
            if let Some(target) = success.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::CheckSuccess, target),
                    target,
                ));
            }

            // failure edge
            if let Some(target) = failure.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::CheckFailure, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::NewZeroedTry {
            success, failure, ..
        }
        | mir::Terminator::NewUninitTry {
            success, failure, ..
        }
        | mir::Terminator::NewSliceZeroedTry {
            success, failure, ..
        }
        | mir::Terminator::NewSliceUninitTry {
            success, failure, ..
        } => {
            let mut edges = Vec::with_capacity(2);

            if let Some(target) = success.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::TrySuccess, target),
                    target,
                ));
            }

            if let Some(target) = failure.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::TryFailure, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::Switch { default, cases, .. } => {
            let mut edges = Vec::with_capacity(cases.len() + 1);

            // default edge
            if let Some(target) = default.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::SwitchDefault, target),
                    target,
                ));
            }

            // case edges
            for case in cases {
                let Some(value) = case.value.integer() else {
                    continue;
                };
                let Some(target) = case.target.block.block() else {
                    continue;
                };

                edges.push((
                    mir::Edge::new(source, mir::Successor::SwitchCase { value }, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::Yield { resume, unwind, .. } => {
            let mut edges = Vec::with_capacity(2);

            if let Some(target) = resume.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::YieldResume, target),
                    target,
                ));
            }

            if let Some(unwind) = unwind
                && let Some(target) = unwind.block.block()
            {
                edges.push((
                    mir::Edge::new(source, mir::Successor::YieldUnwind, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::Call { target, unwind, .. }
        | mir::Terminator::CallIndirect { target, unwind, .. }
        | mir::Terminator::CallVirtual { target, unwind, .. }
        | mir::Terminator::CallDynamic { target, unwind, .. } => {
            let mut edges = Vec::with_capacity(2);

            if let Some(target) = target.block.block() {
                edges.push((
                    mir::Edge::new(source, mir::Successor::CallReturn, target),
                    target,
                ));
            }

            if let Some(unwind) = unwind
                && let Some(target) = unwind.block.block()
            {
                edges.push((
                    mir::Edge::new(source, mir::Successor::CallUnwind, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::Return { .. }
        | mir::Terminator::Panic { .. }
        | mir::Terminator::ResumeUnwind
        | mir::Terminator::Trap { .. }
        | mir::Terminator::Unreachable
        | mir::Terminator::TailCall { .. }
        | mir::Terminator::TailCallVirtual { .. }
        | mir::Terminator::TailCallDynamic { .. }
        | mir::Terminator::TailCallIndirect { .. } => Vec::new(),
    }
}
