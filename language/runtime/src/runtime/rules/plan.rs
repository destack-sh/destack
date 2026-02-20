use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
use destack_workspace::{ExecutionMode, RuntimeAction, RuntimeHook, RuntimeOptions, RuntimeRule};

use super::matches_runtime_filter;

/// Immutable runtime rule plan compiled from runtime options.
#[derive(Debug, Clone)]
pub(crate) struct RuntimeRulePlan {
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Runtime rules loaded from runtime options.
    rules: Vec<RuntimeRule>,
}

impl RuntimeRulePlan {
    /// Create a runtime rule plan from runtime options.
    pub(crate) fn from_runtime_options(options: &RuntimeOptions) -> Self {
        Self {
            mode: options.execution,
            rules: options.rules.clone(),
        }
    }

    /// Return the total number of configured rules.
    pub(crate) fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Return the index of the first matching effect rule for one hook.
    pub(crate) fn first_matching_effect_rule(
        &self,
        descriptor: Option<BindingDescriptor>,
        engine: Option<BindingEngine>,
        hook: RuntimeHook,
    ) -> Option<usize> {
        // apply first match semantics for effect rules
        for (index, rule) in self.rules.iter().enumerate() {
            if !matches_runtime_filter(&rule.when, descriptor, self.mode, engine) {
                continue;
            }

            if !matches!(
                rule.action,
                RuntimeAction::Fault { .. } | RuntimeAction::Control { .. }
            ) {
                continue;
            }

            if !matches_rule_hook(rule, hook) {
                continue;
            }

            return Some(index);
        }

        None
    }
}

/// Return true when one rule is enabled for one hook.
fn matches_rule_hook(rule: &RuntimeRule, expected: RuntimeHook) -> bool {
    let Some(trigger) = &rule.trigger else {
        return false;
    };

    trigger.on == expected
}

#[cfg(test)]
mod tests {
    use super::RuntimeRulePlan;
    use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
    use destack_workspace::{
        RuntimeAction, RuntimeControlEffect, RuntimeFilter, RuntimeHook, RuntimeOptions,
        RuntimeRule, RuntimeTrigger,
    };

    /// Ensures effect matching returns the first matching rule index.
    #[test]
    fn test_first_matching_effect_rule_returns_first_index() {
        // prepare a plan with two matching effect rules
        let options = RuntimeOptions {
            rules: vec![
                effect_rule(RuntimeHook::BindingBefore),
                effect_rule(RuntimeHook::BindingBefore),
            ],
            ..RuntimeOptions::default()
        };
        let plan = RuntimeRulePlan::from_runtime_options(&options);
        let descriptor = BindingDescriptor::pure("destack.test.rule", "()");

        // first match semantics should choose index 0
        let index = plan.first_matching_effect_rule(
            Some(descriptor),
            Some(BindingEngine::Vm),
            RuntimeHook::BindingBefore,
        );
        assert_eq!(index, Some(0));
    }

    /// Ensures hook matching skips rules that do not include the current hook.
    #[test]
    fn test_first_matching_effect_rule_respects_hook() {
        // prepare one rule for another hook and one for binding before
        let options = RuntimeOptions {
            rules: vec![
                effect_rule(RuntimeHook::SchedulerDequeue),
                effect_rule(RuntimeHook::BindingBefore),
            ],
            ..RuntimeOptions::default()
        };
        let plan = RuntimeRulePlan::from_runtime_options(&options);
        let descriptor = BindingDescriptor::pure("destack.test.rule", "()");

        // binding before should skip index 0 and choose index 1
        let index = plan.first_matching_effect_rule(
            Some(descriptor),
            Some(BindingEngine::Native),
            RuntimeHook::BindingBefore,
        );
        assert_eq!(index, Some(1));
    }

    /// Build one minimal effect rule for tests.
    fn effect_rule(on: RuntimeHook) -> RuntimeRule {
        RuntimeRule {
            id: None,
            when: RuntimeFilter::default(),
            action: RuntimeAction::Control {
                control: RuntimeControlEffect::GcCycleCollect {},
            },
            trigger: Some(RuntimeTrigger {
                on,
                probability_ppm: None,
                max_occurrences: None,
                cooldown_ns: None,
                burst: None,
                activation: None,
                lifetime: None,
            }),
        }
    }
}
