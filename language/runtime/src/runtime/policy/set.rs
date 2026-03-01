use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{PolicyMutation, Rule, RuleId, assert_rule_fault_compatibility};
use destack_workspace as workspace;

/// Runtime policy set for startup rule materialization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Policy {
    /// Ordered startup runtime rules.
    pub rules: Vec<Rule>,
    /// Ordered runtime policy mutations.
    pub mutations: Vec<PolicyMutation>,
}

impl Policy {
    /// Convert workspace static policy rules into one runtime policy set.
    pub fn from_workspace_policy_rules(policy_rules: &[workspace::RuntimePolicyRule]) -> Self {
        let rules = policy_rules
            .iter()
            .enumerate()
            .map(|(index, rule)| Rule::from_workspace_policy_rule(index, rule))
            .collect();

        Self {
            rules,
            mutations: Vec::new(),
        }
    }

    /// Build the startup rule set materialized from runtime rules and mutations.
    pub fn startup_rules(&self) -> Vec<Rule> {
        // keep rule slots in declaration order and track enabled state
        let mut entries: Vec<(Rule, bool)> = Vec::new();
        let mut installed_rule_ids: HashSet<RuleId> = HashSet::new();

        // seed one enabled entry for every declared startup rule
        for rule in &self.rules {
            let inserted = installed_rule_ids.insert(rule.id.clone());
            assert!(
                inserted,
                "runtime rules require unique rule ids: {}",
                rule.id.0
            );

            // enforce a strict fault compatibility matrix for declared rules
            assert_rule_fault_compatibility(rule);

            entries.push((rule.clone(), true));
        }

        // apply startup mutations in order
        for mutation in &self.mutations {
            // install one rule as enabled
            if let PolicyMutation::InstallRule { rule } = mutation {
                let inserted = installed_rule_ids.insert(rule.id.clone());
                assert!(
                    inserted,
                    "runtime installRule requires unique rule ids: {}",
                    rule.id.0
                );

                // enforce a strict fault compatibility matrix for installed rules
                assert_rule_fault_compatibility(rule);

                entries.push((rule.clone(), true));
                continue;
            }

            // remove one matching id
            if let PolicyMutation::RemoveRule { rule_id } = mutation {
                let removed = installed_rule_ids.remove(rule_id);
                assert!(
                    removed,
                    "runtime removeRule requires one installed rule id: {}",
                    rule_id.0
                );

                entries.retain(|(rule, _)| rule.id != *rule_id);
                continue;
            }

            // enable one matching id
            if let PolicyMutation::EnableRule { rule_id } = mutation {
                let mut has_match = false;
                for (rule, is_enabled) in &mut entries {
                    if rule.id == *rule_id {
                        has_match = true;
                        *is_enabled = true;
                    }
                }
                assert!(
                    has_match,
                    "runtime enableRule requires one installed rule id: {}",
                    rule_id.0
                );
                continue;
            }

            // disable one matching id
            if let PolicyMutation::DisableRule { rule_id } = mutation {
                let mut has_match = false;
                for (rule, is_enabled) in &mut entries {
                    if rule.id == *rule_id {
                        has_match = true;
                        *is_enabled = false;
                    }
                }
                assert!(
                    has_match,
                    "runtime disableRule requires one installed rule id: {}",
                    rule_id.0
                );
                continue;
            }
        }

        // return enabled rules in declaration order
        entries
            .into_iter()
            .filter_map(|(rule, is_enabled)| if is_enabled { Some(rule) } else { None })
            .collect()
    }
}
