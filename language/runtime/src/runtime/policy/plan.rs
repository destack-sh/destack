use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
use destack_workspace::ExecutionMode;

use super::{Effect, Hook, Policy, Rule, matches_selector};

/// Immutable runtime rule plan compiled from runtime options.
#[derive(Debug, Clone)]
pub(crate) struct PolicyPlan {
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Runtime rules loaded from runtime options.
    rules: Vec<Rule>,
}

impl PolicyPlan {
    /// Create a runtime rule plan from one explicit policy.
    pub(crate) fn from_policy(mode: ExecutionMode, policy: &Policy) -> Self {
        Self {
            mode,
            rules: policy.startup_rules(),
        }
    }

    /// Return the total number of configured rules.
    pub(crate) fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Return matching effect rule indexes for one hook in declaration order.
    pub(crate) fn matching_effect_rule_indexes(
        &self,
        descriptor: Option<BindingDescriptor>,
        engine: Option<BindingEngine>,
        agent_id: Option<u64>,
        hook: Hook,
    ) -> Vec<usize> {
        let mut indexes = Vec::new();

        // collect all matching fault rules in declaration order
        for (index, rule) in self.rules.iter().enumerate() {
            if !matches_selector(&rule.when, descriptor, self.mode, engine, agent_id) {
                continue;
            }

            if !matches!(rule.action, Effect::Fault { .. }) {
                continue;
            }

            if !matches_rule_hook(rule, hook) {
                continue;
            }

            indexes.push(index);
        }

        indexes
    }
}

/// Return true when one rule is enabled for one hook.
fn matches_rule_hook(rule: &Rule, expected: Hook) -> bool {
    let Some(trigger) = &rule.trigger else {
        return false;
    };

    trigger.on == expected
}

#[cfg(test)]
mod tests {
    use super::PolicyPlan;
    use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
    use crate::runtime::policy::{
        Effect, Fault, FaultTarget, FaultType, Hook, Policy, PolicyMutation, Rule, RuleId,
        Selector, Trigger,
    };
    use destack_workspace::ExecutionMode;

    /// Ensures effect matching returns all matching indexes in declaration order.
    #[test]
    fn test_matching_effect_rule_indexes_returns_all_indexes() {
        // prepare a plan with two matching effect rules
        let policy = Policy {
            mutations: vec![
                PolicyMutation::InstallRule {
                    rule: effect_rule("a", Hook::BindingBefore),
                },
                PolicyMutation::InstallRule {
                    rule: effect_rule("b", Hook::BindingBefore),
                },
            ],
            ..Policy::default()
        };
        let plan = PolicyPlan::from_policy(ExecutionMode::Fast, &policy);
        let descriptor = BindingDescriptor::pure("destack.test.rule", "()");

        // matching semantics should collect both indexes
        let indexes = plan.matching_effect_rule_indexes(
            Some(descriptor),
            Some(BindingEngine::Vm),
            None,
            Hook::BindingBefore,
        );
        assert_eq!(indexes, vec![0, 1]);
    }

    /// Ensures effect matching supports top-level startup rules.
    #[test]
    fn test_matching_effect_rule_indexes_supports_top_level_rules() {
        // prepare a plan with two matching startup rules
        let policy = Policy {
            rules: vec![
                effect_rule("a", Hook::BindingBefore),
                effect_rule("b", Hook::BindingBefore),
            ],
            ..Policy::default()
        };
        let plan = PolicyPlan::from_policy(ExecutionMode::Fast, &policy);
        let descriptor = BindingDescriptor::pure("destack.test.rule", "()");

        // matching semantics should collect both startup rule indexes
        let indexes = plan.matching_effect_rule_indexes(
            Some(descriptor),
            Some(BindingEngine::Vm),
            None,
            Hook::BindingBefore,
        );
        assert_eq!(indexes, vec![0, 1]);
    }

    /// Ensures hook matching skips rules that do not include the current hook.
    #[test]
    fn test_matching_effect_rule_indexes_respects_hook() {
        // prepare one rule for another hook and one for binding before
        let policy = Policy {
            mutations: vec![
                PolicyMutation::InstallRule {
                    rule: effect_rule("a", Hook::SchedulerDequeue),
                },
                PolicyMutation::InstallRule {
                    rule: effect_rule("b", Hook::BindingBefore),
                },
            ],
            ..Policy::default()
        };
        let plan = PolicyPlan::from_policy(ExecutionMode::Fast, &policy);
        let descriptor = BindingDescriptor::pure("destack.test.rule", "()");

        // binding before should skip index 0 and choose index 1
        let indexes = plan.matching_effect_rule_indexes(
            Some(descriptor),
            Some(BindingEngine::Native),
            None,
            Hook::BindingBefore,
        );
        assert_eq!(indexes, vec![1]);
    }

    /// Ensures startup mutations update the top-level startup rule set in order.
    #[test]
    fn test_matching_effect_rule_indexes_applies_mutations_to_top_level_rules() {
        // prepare one startup rule and disable then enable it through mutations
        let rule = effect_rule("toggle", Hook::BindingBefore);
        let policy = Policy {
            rules: vec![rule.clone()],
            mutations: vec![
                PolicyMutation::DisableRule {
                    rule_id: rule.id.clone(),
                },
                PolicyMutation::EnableRule {
                    rule_id: rule.id.clone(),
                },
            ],
            ..Policy::default()
        };
        let plan = PolicyPlan::from_policy(ExecutionMode::Fast, &policy);
        let descriptor = BindingDescriptor::pure("destack.test.rule", "()");

        // the final enabled state should keep the rule active
        let indexes = plan.matching_effect_rule_indexes(
            Some(descriptor),
            Some(BindingEngine::Native),
            None,
            Hook::BindingBefore,
        );
        assert_eq!(indexes, vec![0]);
    }

    /// Ensures startup rule materialization rejects duplicate top-level rule identifiers.
    #[test]
    #[should_panic(expected = "runtime rules require unique rule ids")]
    fn test_matching_effect_rule_indexes_panics_on_duplicate_top_level_rule_ids() {
        // prepare a plan with duplicate startup rule ids
        let policy = Policy {
            rules: vec![
                effect_rule("duplicate", Hook::BindingBefore),
                effect_rule("duplicate", Hook::BindingBefore),
            ],
            ..Policy::default()
        };

        // building the plan should fail loudly
        let _plan = PolicyPlan::from_policy(ExecutionMode::Fast, &policy);
    }

    /// Ensures startup rule materialization rejects duplicate rule identifiers.
    #[test]
    #[should_panic(expected = "runtime installRule requires unique rule ids")]
    fn test_matching_effect_rule_indexes_panics_on_duplicate_rule_ids() {
        // prepare a plan with duplicate install ids
        let policy = Policy {
            mutations: vec![
                PolicyMutation::InstallRule {
                    rule: effect_rule("duplicate", Hook::BindingBefore),
                },
                PolicyMutation::InstallRule {
                    rule: effect_rule("duplicate", Hook::BindingBefore),
                },
            ],
            ..Policy::default()
        };

        // building the plan should fail loudly
        let _plan = PolicyPlan::from_policy(ExecutionMode::Fast, &policy);
    }

    /// Ensures startup rule materialization rejects unknown enable identifiers.
    #[test]
    #[should_panic(expected = "runtime enableRule requires one installed rule id")]
    fn test_matching_effect_rule_indexes_panics_on_unknown_enable_rule() {
        // prepare a plan with one unknown enable mutation
        let policy = Policy {
            mutations: vec![PolicyMutation::EnableRule {
                rule_id: RuleId("missing".to_string()),
            }],
            ..Policy::default()
        };

        // building the plan should fail loudly
        let _plan = PolicyPlan::from_policy(ExecutionMode::Fast, &policy);
    }

    /// Build one minimal effect rule for tests.
    fn effect_rule(id_suffix: &str, on: Hook) -> Rule {
        Rule {
            id: RuleId(format!("test.rule.{id_suffix}")),
            when: Selector::default(),
            action: Effect::Fault {
                fault: Fault {
                    target: FaultTarget::Call {},
                    fault_type: FaultType::Drop {},
                },
            },
            trigger: Some(Trigger {
                on,
                activation_window: None,
                lifetime: None,
                activation_ppm: None,
                probability_ppm: None,
                max_occurrences: None,
                cooldown_ns: None,
                burst: None,
                interval_hits: None,
                skip_hits: None,
            }),
        }
    }
}
