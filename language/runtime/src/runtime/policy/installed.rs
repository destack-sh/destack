use std::collections::HashMap;

use super::Policy;

/// Trigger state key scope for one rule bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TriggerScopeKey {
    /// Agent-local trigger scope.
    Agent(u64),
}

/// Trigger counters for one scope bucket.
#[derive(Debug, Clone, Default)]
struct TriggerBucketState {
    /// Total matched effects seen for this scope.
    pub matched_effects_seen: u64,
}

/// Trigger state for one rule.
#[derive(Debug, Clone, Default)]
struct RuleTriggerState {
    /// Trigger buckets keyed by scope.
    pub buckets: HashMap<TriggerScopeKey, TriggerBucketState>,
}

/// Trigger runtime state for one installed policy.
#[derive(Debug, Clone, Default)]
struct PolicyTriggerRuntimeState {
    /// Policy revision this trigger state belongs to.
    pub revision: u64,
    /// Total matching calls seen across all scopes.
    pub calls_seen: u64,
    /// Per-rule trigger state indexed by compiled rule index.
    pub rules: Vec<RuleTriggerState>,
}

/// Installed policy plus world-local runtime state.
#[derive(Debug, Clone)]
pub(crate) struct InstalledPolicy {
    /// Active policy specification.
    pub policy: Policy,
    /// World-local monotonic policy revision.
    pub revision: u64,
    /// Mutable trigger runtime state.
    trigger: PolicyTriggerRuntimeState,
}

impl InstalledPolicy {
    /// Create one installed policy with default runtime state.
    pub(crate) fn new(policy: Policy) -> Self {
        Self {
            policy,
            revision: 1,
            trigger: PolicyTriggerRuntimeState::default(),
        }
    }

    /// Replace active policy and reset runtime trigger state.
    pub(crate) fn replace_policy(&mut self, policy: Policy) {
        self.policy = policy;
        self.revision = self.revision.saturating_add(1);
        self.trigger = PolicyTriggerRuntimeState::default();
    }

    /// Prepare trigger runtime state for one policy revision.
    pub(crate) fn prepare_trigger_state_for_revision(&mut self, revision: u64, rule_count: usize) {
        if self.revision != revision {
            return;
        }

        if self.trigger.revision == revision {
            return;
        }

        self.trigger.revision = revision;
        self.trigger.calls_seen = 0;
        self.trigger.rules = vec![RuleTriggerState::default(); rule_count];
    }

    /// Record one matching call.
    pub(crate) fn record_call(&mut self) {
        self.trigger.calls_seen = self.trigger.calls_seen.saturating_add(1);
    }

    /// Record matched effect rule indexes for one scope key.
    pub(crate) fn record_matches(&mut self, scope_key: TriggerScopeKey, indexes: &[usize]) {
        if indexes.is_empty() {
            return;
        }

        for index in indexes {
            if let Some(rule_state) = self.trigger.rules.get_mut(*index) {
                let bucket = rule_state
                    .buckets
                    .entry(scope_key)
                    .or_insert_with(TriggerBucketState::default);
                bucket.matched_effects_seen = bucket.matched_effects_seen.saturating_add(1);
            }
        }
    }

    /// Return matched effect totals across all scopes per rule index.
    #[cfg(test)]
    pub(crate) fn matched_effects_seen_totals(&self) -> Vec<u64> {
        self.trigger
            .rules
            .iter()
            .map(|rule_state| {
                rule_state.buckets.values().fold(0u64, |sum, bucket| {
                    sum.saturating_add(bucket.matched_effects_seen)
                })
            })
            .collect()
    }
}
