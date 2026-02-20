use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::bindings::{BindingDescriptor, ExecutionMode, PolicyEngine};
use crate::runtime::rules::matches_runtime_filter;
use destack_workspace::{RuntimeEffect, RuntimeOptions, RuntimeRule};

/// Runtime effect engine for fault, control, and mock rule scaffolding.
#[derive(Debug)]
pub struct RuntimeEffectEngine {
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Runtime rules loaded from runtime options.
    rules: Vec<RuntimeRule>,
    /// Runtime effect counters.
    state: Mutex<RuntimeEffectState>,
}

/// Runtime effect counters.
#[derive(Debug, Default)]
struct RuntimeEffectState {
    /// Total binding calls observed.
    calls_seen: u64,
    /// Matched non-policy effects per rule index.
    matched_effects_seen: Vec<u64>,
}

impl RuntimeEffectEngine {
    /// Create the runtime effect engine from runtime options.
    pub fn from_runtime_options(options: &RuntimeOptions) -> Self {
        // prepare rule counters for the loaded rules
        let counters = vec![0; options.rules.len()];

        Self {
            mode: options.execution.into(),
            rules: options.rules.clone(),
            state: Mutex::new(RuntimeEffectState {
                calls_seen: 0,
                matched_effects_seen: counters,
            }),
        }
    }

    /// Evaluate pre-call runtime effects for one binding invocation.
    pub fn before_binding(
        &self,
        descriptor: BindingDescriptor,
        engine: Option<PolicyEngine>,
    ) -> RuntimeResult<()> {
        // count this call for diagnostics and future trigger state
        let mut state = self.state.lock();
        state.calls_seen = state.calls_seen.saturating_add(1);

        // count matched non-policy effects for future execution wiring
        for (index, rule) in self.rules.iter().enumerate() {
            if !matches_runtime_filter(&rule.when, descriptor, self.mode, engine) {
                continue;
            }

            if let RuntimeEffect::Policy { .. } = rule.effect {
                continue;
            }

            // NOTE #Incomplete: activation, lifetime, and trigger semantics are not wired yet
            state.matched_effects_seen[index] = state.matched_effects_seen[index].saturating_add(1);
        }

        Ok(())
    }

    /// Return the number of configured rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Return the number of non-policy rules.
    pub fn non_policy_rule_count(&self) -> usize {
        self.rules
            .iter()
            .filter(|rule| !matches!(rule.effect, RuntimeEffect::Policy { .. }))
            .count()
    }
}
