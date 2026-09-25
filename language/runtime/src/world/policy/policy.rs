use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_serde::Reflect;

use crate::diagnostic::{RuntimeError, RuntimeResult};

use super::{Decision, Rule, RuleId, Subject};

/// Runtime policy specification.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Policy {
    /// Default decision when no rule matches.
    #[serde(default)]
    pub default: Decision,
    /// Ordered runtime rules.
    pub rules: Vec<Rule>,
}

impl Policy {
    /// Validate all policy invariants.
    pub(crate) fn validate(&self) -> RuntimeResult<()> {
        self.validate_unique_rule_ids()
    }

    /// Add one rule into this policy with full validation.
    pub(crate) fn add_rule(&mut self, rule: Rule) -> RuntimeResult<()> {
        if self.has_rule_id(&rule.id) {
            return Err(Self::invalid_policy_error(format!(
                "runtime policy requires unique rule ids: {}",
                rule.id.0
            )));
        }

        self.rules.push(rule);

        Ok(())
    }

    /// Remove one rule from this policy.
    pub(crate) fn remove_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let Some(index) = self.rules.iter().position(|rule| rule.id == *rule_id) else {
            return Err(Self::invalid_policy_error(format!(
                "runtime mutation requires one known rule id: {}",
                rule_id.0
            )));
        };

        self.rules.remove(index);

        Ok(())
    }

    /// Enable one rule in this policy.
    pub(crate) fn enable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let Some(rule) = self.rules.iter_mut().find(|rule| rule.id == *rule_id) else {
            return Err(Self::invalid_policy_error(format!(
                "runtime mutation requires one known rule id: {}",
                rule_id.0
            )));
        };

        rule.enabled = true;

        Ok(())
    }

    /// Disable one rule in this policy.
    pub(crate) fn disable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let Some(rule) = self.rules.iter_mut().find(|rule| rule.id == *rule_id) else {
            return Err(Self::invalid_policy_error(format!(
                "runtime mutation requires one known rule id: {}",
                rule_id.0
            )));
        };

        rule.enabled = false;

        Ok(())
    }

    /// Replace one rule in this policy with full validation.
    pub(crate) fn replace_rule(
        &mut self,
        rule_id: &RuleId,
        replacement: Rule,
    ) -> RuntimeResult<()> {
        if replacement.id != *rule_id {
            return Err(Self::invalid_policy_error(format!(
                "runtime replace requires replacement id to match: {}",
                rule_id.0
            )));
        }

        let Some(rule) = self.rules.iter_mut().find(|rule| rule.id == *rule_id) else {
            return Err(Self::invalid_policy_error(format!(
                "runtime replace requires one known rule id: {}",
                rule_id.0
            )));
        };

        *rule = replacement;

        Ok(())
    }

    /// Return true when one rule id exists in this policy.
    fn has_rule_id(&self, rule_id: &RuleId) -> bool {
        self.rules.iter().any(|rule| rule.id == *rule_id)
    }

    /// Return one invalid-policy error.
    fn invalid_policy_error(message: impl Into<String>) -> Box<RuntimeError> {
        RuntimeError::Internal {
            message: message.into(),
        }
        .boxed()
    }

    /// Validate that this policy contains unique rule ids.
    fn validate_unique_rule_ids(&self) -> RuntimeResult<()> {
        let mut rule_ids: HashSet<RuleId> = HashSet::new();
        for rule in &self.rules {
            if rule_ids.insert(rule.id.clone()) {
                continue;
            }

            return Err(Self::invalid_policy_error(format!(
                "runtime rules require unique rule ids: {}",
                rule.id.0
            )));
        }

        Ok(())
    }

    /// Replace active policy.
    pub(crate) fn replace(&mut self, policy: Policy) -> RuntimeResult<()> {
        policy.validate()?;
        *self = policy;

        Ok(())
    }

    /// Decide one binding call.
    pub(crate) fn decide_binding(
        &self,
        subject: Subject<'_>,
        program: &program::Program,
        binding: &program::Binding,
    ) -> RuntimeResult<Decision> {
        let mut decision = self.default;

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            if !rule.matches(subject, program, binding)? {
                continue;
            }

            decision = rule.decision;
            break;
        }

        Ok(decision)
    }
}
