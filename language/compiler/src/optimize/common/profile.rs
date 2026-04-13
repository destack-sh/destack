use std::collections::HashMap;

use destack_mir as mir;

/// Policy thresholds for classifying callsites by profile data.
#[derive(Debug, Clone, PartialEq)]
pub struct CallsiteHotnessPolicy {
    /// Minimum total call count to classify as hot.
    pub hot_count: u64,
    /// Minimum callsite ratio to classify as hot.
    pub hot_ratio: f64,
    /// Maximum total call count to classify as cold.
    pub cold_count: u64,
    /// Maximum callsite ratio to classify as cold.
    pub cold_ratio: f64,
    /// Maximum unknown ratio allowed to classify as hot.
    pub unknown_ratio_max: f64,
    /// True when missing callsite profiles are treated as cold.
    pub missing_callsite_is_cold: bool,
    /// True when missing caller profiles are treated as cold.
    pub missing_caller_is_cold: bool,
    /// Divisor for estimated counts.
    pub estimated_divisor: u64,
    /// Divisor for synthetic counts.
    pub synthetic_divisor: u64,
}

impl CallsiteHotnessPolicy {
    /// Create a new callsite hotness policy.
    pub const fn new(hot_count: u64, hot_ratio: f64) -> Self {
        let cold_count = match hot_count / 5 {
            0 => 1,
            value => value,
        };
        let cold_ratio = hot_ratio * 0.10;

        Self {
            hot_count,
            hot_ratio,
            cold_count,
            cold_ratio,
            unknown_ratio_max: 0.25,
            missing_callsite_is_cold: true,
            missing_caller_is_cold: true,
            estimated_divisor: 2,
            synthetic_divisor: 4,
        }
    }

    /// Default policy for inlining decisions.
    pub const fn inline_default() -> Self {
        Self {
            hot_count: 50,
            hot_ratio: 0.10,
            cold_count: 8,
            cold_ratio: 0.01,
            unknown_ratio_max: 0.25,
            missing_callsite_is_cold: true,
            missing_caller_is_cold: false,
            estimated_divisor: 2,
            synthetic_divisor: 4,
        }
    }

    /// Default policy for argument specialization decisions.
    pub const fn specialize_default() -> Self {
        Self {
            hot_count: 20,
            hot_ratio: 0.20,
            cold_count: 20,
            cold_ratio: 0.20,
            unknown_ratio_max: 0.25,
            missing_callsite_is_cold: true,
            missing_caller_is_cold: true,
            estimated_divisor: 2,
            synthetic_divisor: 4,
        }
    }

    /// Override the cold thresholds for this policy.
    pub const fn with_cold_thresholds(self, cold_count: u64, cold_ratio: f64) -> Self {
        Self {
            cold_count,
            cold_ratio,
            ..self
        }
    }

    /// Return the scaling policy for profile counts.
    pub const fn scaling_policy(&self) -> ProfileScalingPolicy {
        ProfileScalingPolicy {
            estimated_divisor: self.estimated_divisor,
            synthetic_divisor: self.synthetic_divisor,
        }
    }
}

/// Scaling factors for profile counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileScalingPolicy {
    /// Divisor for estimated counts.
    pub estimated_divisor: u64,
    /// Divisor for synthetic counts.
    pub synthetic_divisor: u64,
}

impl ProfileScalingPolicy {
    /// Create a new scaling policy.
    pub const fn new(estimated_divisor: u64, synthetic_divisor: u64) -> Self {
        Self {
            estimated_divisor,
            synthetic_divisor,
        }
    }

    /// Default scaling policy.
    pub const fn default() -> Self {
        Self {
            estimated_divisor: 2,
            synthetic_divisor: 4,
        }
    }
}

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
    profile: Option<&mir::ProfileTable>,
    caller: mir::LocalNodeId<mir::Function>,
    callsite: mir::LocalNodeId<mir::Instruction>,
    policy: &CallsiteHotnessPolicy,
) -> CallsiteHotness {
    // bail out when no profile data is present
    let Some(profile) = profile else {
        return CallsiteHotness::Unknown;
    };

    // reject callsites without profile data
    let Some(callsite_profile) = profile.callsite_profile(callsite) else {
        return if policy.missing_callsite_is_cold {
            CallsiteHotness::Cold
        } else {
            CallsiteHotness::Unknown
        };
    };

    // scale the callsite count for confidence and sampling
    let total_count = scaled_profile_count(
        callsite_profile.total_count,
        profile.source,
        &policy.scaling_policy(),
    );
    if total_count == 0 {
        return CallsiteHotness::Cold;
    }

    // treat high unknown ratios as cold
    let unknown_count = scaled_profile_count(
        callsite_profile.unknown_count,
        profile.source,
        &policy.scaling_policy(),
    );
    let unknown_ratio = unknown_count as f64 / total_count as f64;
    if unknown_ratio > policy.unknown_ratio_max {
        return CallsiteHotness::Cold;
    }

    // accept hot callsites based on absolute counts
    if total_count >= policy.hot_count {
        return CallsiteHotness::Hot;
    }

    // reject cold callsites based on absolute counts
    if total_count <= policy.cold_count {
        return CallsiteHotness::Cold;
    }

    // require caller entry counts for ratio based classification
    let Some(function_profile) = profile.function_profile(caller) else {
        return if policy.missing_caller_is_cold {
            CallsiteHotness::Cold
        } else {
            CallsiteHotness::Unknown
        };
    };

    // reject zero entry counts
    let entry_count = scaled_profile_count(
        function_profile.entry_count,
        profile.source,
        &policy.scaling_policy(),
    );
    if entry_count == 0 {
        return CallsiteHotness::Cold;
    }

    // classify based on callsite ratio
    let ratio = total_count as f64 / entry_count as f64;
    if ratio >= policy.hot_ratio {
        return CallsiteHotness::Hot;
    }

    if ratio <= policy.cold_ratio {
        return CallsiteHotness::Cold;
    }

    CallsiteHotness::Unknown
}

/// Scale a profile count based on source and confidence.
pub fn scaled_profile_count(
    count: mir::ProfileCount,
    source: mir::ProfileSource,
    policy: &ProfileScalingPolicy,
) -> u64 {
    // apply sampling period when needed
    let mut value = count.value;
    if let mir::ProfileSource::Sampled { period } = source {
        value = value.saturating_mul(period);
    }

    // downscale based on confidence
    let scaled = match count.confidence {
        mir::ProfileConfidence::Precise => value,
        mir::ProfileConfidence::Estimated => value / policy.estimated_divisor.max(1),
        mir::ProfileConfidence::Synthetic => value / policy.synthetic_divisor.max(1),
    };

    // keep non zero signal when sampling confidence is low
    if value > 0 && scaled == 0 {
        return 1;
    }

    scaled
}

/// Derive hotness from block execution counts.
pub fn block_hotness_from_counts(
    block_count: u64,
    entry_count: u64,
    policy: &CallsiteHotnessPolicy,
) -> CallsiteHotness {
    // guard against missing counts
    if block_count == 0 {
        return CallsiteHotness::Unknown;
    }

    // classify by absolute counts
    if block_count >= policy.hot_count {
        return CallsiteHotness::Hot;
    }
    if block_count <= policy.cold_count {
        return CallsiteHotness::Cold;
    }

    // fall back to ratio based classification
    if entry_count == 0 {
        return CallsiteHotness::Unknown;
    }

    // compare ratios against thresholds
    let ratio = block_count as f64 / entry_count as f64;
    if ratio >= policy.hot_ratio {
        return CallsiteHotness::Hot;
    }
    if ratio <= policy.cold_ratio {
        return CallsiteHotness::Cold;
    }

    CallsiteHotness::Unknown
}

/// Compute execution counts for blocks using profile data.
pub fn block_execution_counts(
    function: &mir::Function,
    tree: &mir::NodeTree,
    profile: Option<&mir::ProfileTable>,
    policy: &CallsiteHotnessPolicy,
) -> HashMap<mir::LocalNodeId<mir::Block>, u64> {
    // return early without profiles
    let Some(profile) = profile else {
        return HashMap::new();
    };

    // seed counts from block profiles
    let scale = policy.scaling_policy();
    let mut counts = HashMap::new();
    for &block_id in &function.blocks {
        // read block profile counts when present
        let count = if let Some(block_profile) = profile.block_profile(block_id) {
            scaled_profile_count(block_profile.execution_count, profile.source, &scale)
        } else {
            0
        };

        // keep only non zero counts
        if count > 0 {
            counts.insert(block_id, count);
        }
    }

    // fill in missing counts from incoming edges
    let incoming = incoming_edge_counts(function, tree, profile, &scale);
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
    tree: &mir::NodeTree,
    profile: &mir::ProfileTable,
    scale: &ProfileScalingPolicy,
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
        for (edge_key, target) in edges {
            // skip when no edge profile exists
            let Some(edge_profile) = profile.edge_profile(&edge_key) else {
                continue;
            };

            // accumulate scaled edge counts
            let count = scaled_profile_count(edge_profile.count, profile.source, scale);
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
) -> Vec<(mir::EdgeKey, mir::LocalNodeId<mir::Block>)> {
    match terminator {
        mir::Terminator::Error => Vec::new(),
        mir::Terminator::Jump { target, .. } => {
            target.block.block().map_or_else(Vec::new, |target| {
                vec![(
                    mir::EdgeKey::new(source, mir::EdgeKind::Jump, target),
                    target,
                )]
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
                    mir::EdgeKey::new(source, mir::EdgeKind::BranchThen, target),
                    target,
                ));
            }

            // else edge
            if let Some(target) = else_target.block.block() {
                edges.push((
                    mir::EdgeKey::new(source, mir::EdgeKind::BranchElse, target),
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
                    mir::EdgeKey::new(source, mir::EdgeKind::CheckSuccess, target),
                    target,
                ));
            }

            // failure edge
            if let Some(target) = failure.block.block() {
                edges.push((
                    mir::EdgeKey::new(source, mir::EdgeKind::CheckFailure, target),
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
                    mir::EdgeKey::new(source, mir::EdgeKind::SwitchDefault, target),
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
                    mir::EdgeKey::new(source, mir::EdgeKind::SwitchCase { value }, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::Yield { resume, .. } => {
            resume.block.block().map_or_else(Vec::new, |target| {
                vec![(
                    mir::EdgeKey::new(source, mir::EdgeKind::YieldResume, target),
                    target,
                )]
            })
        }
        mir::Terminator::Invoke {
            normal_target,
            unwind_target,
            ..
        }
        | mir::Terminator::InvokeIndirect {
            normal_target,
            unwind_target,
            ..
        }
        | mir::Terminator::InvokeVirtual {
            normal_target,
            unwind_target,
            ..
        }
        | mir::Terminator::InvokeInterface {
            normal_target,
            unwind_target,
            ..
        } => {
            let mut edges = Vec::with_capacity(2);

            // normal edge
            if let Some(target) = normal_target.block.block() {
                edges.push((
                    mir::EdgeKey::new(source, mir::EdgeKind::CallNormal, target),
                    target,
                ));
            }

            // unwind edge
            if let Some(target) = unwind_target.block.block() {
                edges.push((
                    mir::EdgeKey::new(source, mir::EdgeKind::CallUnwind, target),
                    target,
                ));
            }

            edges
        }
        mir::Terminator::Return { .. }
        | mir::Terminator::Throw { .. }
        | mir::Terminator::Trap { .. }
        | mir::Terminator::Unreachable
        | mir::Terminator::TailCall { .. }
        | mir::Terminator::TailCallVirtual { .. }
        | mir::Terminator::TailCallInterface { .. }
        | mir::Terminator::TailCallIndirect { .. } => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sampled profile counts scale by the sampling period.
    #[test]
    fn test_callsite_hotness_scales_sampled_counts() {
        let policy = CallsiteHotnessPolicy::new(50, 0.10);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Sampled { period: 10 });
        let caller = mir::LocalNodeId::<mir::Function>::new(1);
        let callsite = mir::LocalNodeId::<mir::Instruction>::new(2);
        profile.functions.insert(
            caller,
            mir::FunctionProfile {
                entry_count: mir::ProfileCount::new(10, mir::ProfileConfidence::Precise),
            },
        );
        profile.callsites.insert(
            callsite,
            mir::CallSiteProfile {
                total_count: mir::ProfileCount::new(6, mir::ProfileConfidence::Precise),
                targets: Vec::new(),
                unknown_count: mir::ProfileCount::new(0, mir::ProfileConfidence::Precise),
            },
        );

        let hotness = callsite_hotness(Some(&profile), caller, callsite, &policy);
        assert_eq!(hotness, CallsiteHotness::Hot);
    }

    /// Unknown target ratios block hotness.
    #[test]
    fn test_callsite_hotness_blocks_high_unknown_ratio() {
        let policy = CallsiteHotnessPolicy::new(10, 0.10);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let caller = mir::LocalNodeId::<mir::Function>::new(1);
        let callsite = mir::LocalNodeId::<mir::Instruction>::new(2);
        profile.functions.insert(
            caller,
            mir::FunctionProfile {
                entry_count: mir::ProfileCount::new(100, mir::ProfileConfidence::Precise),
            },
        );
        profile.callsites.insert(
            callsite,
            mir::CallSiteProfile {
                total_count: mir::ProfileCount::new(20, mir::ProfileConfidence::Precise),
                targets: Vec::new(),
                unknown_count: mir::ProfileCount::new(10, mir::ProfileConfidence::Precise),
            },
        );

        let hotness = callsite_hotness(Some(&profile), caller, callsite, &policy);
        assert_eq!(hotness, CallsiteHotness::Cold);
    }

    /// Estimated confidence lowers the effective count.
    #[test]
    fn test_callsite_hotness_scales_estimated_confidence() {
        let policy = CallsiteHotnessPolicy::new(10, 0.10).with_cold_thresholds(10, 0.10);
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let caller = mir::LocalNodeId::<mir::Function>::new(1);
        let callsite = mir::LocalNodeId::<mir::Instruction>::new(2);
        profile.functions.insert(
            caller,
            mir::FunctionProfile {
                entry_count: mir::ProfileCount::new(100, mir::ProfileConfidence::Precise),
            },
        );
        profile.callsites.insert(
            callsite,
            mir::CallSiteProfile {
                total_count: mir::ProfileCount::new(18, mir::ProfileConfidence::Estimated),
                targets: Vec::new(),
                unknown_count: mir::ProfileCount::new(0, mir::ProfileConfidence::Precise),
            },
        );

        let hotness = callsite_hotness(Some(&profile), caller, callsite, &policy);
        assert_eq!(hotness, CallsiteHotness::Cold);
    }

    /// Warm callsites return unknown without strong signals.
    #[test]
    fn test_callsite_hotness_returns_unknown_for_warm() {
        let policy = CallsiteHotnessPolicy::inline_default();
        let mut profile = mir::ProfileTable::new(mir::ProfileSource::Instrumentation);
        let caller = mir::LocalNodeId::<mir::Function>::new(1);
        let callsite = mir::LocalNodeId::<mir::Instruction>::new(2);
        profile.functions.insert(
            caller,
            mir::FunctionProfile {
                entry_count: mir::ProfileCount::new(100, mir::ProfileConfidence::Precise),
            },
        );
        profile.callsites.insert(
            callsite,
            mir::CallSiteProfile {
                total_count: mir::ProfileCount::new(9, mir::ProfileConfidence::Precise),
                targets: Vec::new(),
                unknown_count: mir::ProfileCount::new(0, mir::ProfileConfidence::Precise),
            },
        );

        let hotness = callsite_hotness(Some(&profile), caller, callsite, &policy);
        assert_eq!(hotness, CallsiteHotness::Unknown);
    }

    /// Estimated counts retain a non zero signal after scaling.
    #[test]
    fn test_scaled_profile_count_clamps_non_zero() {
        let policy = ProfileScalingPolicy::new(10, 10);
        let count = mir::ProfileCount::new(1, mir::ProfileConfidence::Estimated);

        let scaled = scaled_profile_count(count, mir::ProfileSource::Instrumentation, &policy);
        assert_eq!(scaled, 1);
    }
}
