use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::{BindingDescriptor, BindingProvider, BindingRoute, RuntimeAccess};

use super::{Attempt, Decision, Rule, RuleId, Subject};

/// Runtime policy specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Policy {
    /// Default decision when no rule matches.
    #[serde(default)]
    pub default: Decision,
    /// Ordered runtime rules.
    pub rules: Vec<Rule>,
}

#[expect(
    clippy::derivable_impls,
    reason = "policy default spells out the allow invariant"
)]
impl Default for Policy {
    fn default() -> Self {
        Self {
            default: Decision::allow(),
            rules: Vec::new(),
        }
    }
}

/// Binding call policy decision used by hot binding call paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BindingDecision {
    /// Final access decision after policy evaluation.
    pub access: RuntimeAccess,
    /// Final route decision after policy evaluation.
    pub route: BindingRoute,
}

/// Active policy and compiled rule state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PolicyState {
    /// Active policy specification.
    pub spec: Policy,
    /// Enabled rule indices for this policy revision.
    enabled_rule_indices: Vec<usize>,
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
        if self.remove_rule_unchecked(rule_id) {
            return Ok(());
        }

        Err(Self::invalid_policy_error(format!(
            "runtime mutation requires one known rule id: {}",
            rule_id.0
        )))
    }

    /// Enable one rule in this policy.
    pub(crate) fn enable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        if self.set_rule_enabled_unchecked(rule_id, true) {
            return Ok(());
        }

        Err(Self::invalid_policy_error(format!(
            "runtime mutation requires one known rule id: {}",
            rule_id.0
        )))
    }

    /// Disable one rule in this policy.
    pub(crate) fn disable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        if self.set_rule_enabled_unchecked(rule_id, false) {
            return Ok(());
        }

        Err(Self::invalid_policy_error(format!(
            "runtime mutation requires one known rule id: {}",
            rule_id.0
        )))
    }

    /// Replace one rule in this policy with full validation.
    pub(crate) fn replace_rule(&mut self, rule_id: &RuleId, rule: Rule) -> RuntimeResult<()> {
        if rule.id != *rule_id {
            return Err(Self::invalid_policy_error(format!(
                "runtime replace requires replacement id to match: {}",
                rule_id.0
            )));
        }

        if !self.has_rule_id(rule_id) {
            return Err(Self::invalid_policy_error(format!(
                "runtime replace requires one known rule id: {}",
                rule_id.0
            )));
        }

        let is_replaced = self.replace_rule_unchecked(rule_id, rule);
        debug_assert!(is_replaced);

        Ok(())
    }

    /// Return true when one rule id exists in this policy.
    fn has_rule_id(&self, rule_id: &RuleId) -> bool {
        self.rules.iter().any(|rule| rule.id == *rule_id)
    }

    /// Return true when one matching rule id exists and has been replaced.
    fn replace_rule_unchecked(&mut self, rule_id: &RuleId, replacement: Rule) -> bool {
        let Some(rule) = self.rules.iter_mut().find(|rule| rule.id == *rule_id) else {
            return false;
        };

        *rule = replacement;

        true
    }

    /// Return true when one matching rule id exists and has been removed.
    fn remove_rule_unchecked(&mut self, rule_id: &RuleId) -> bool {
        let before_len = self.rules.len();
        self.rules.retain(|rule| rule.id != *rule_id);
        self.rules.len() < before_len
    }

    /// Return true when one matching rule id exists and has been updated.
    fn set_rule_enabled_unchecked(&mut self, rule_id: &RuleId, is_enabled: bool) -> bool {
        let mut is_updated = false;

        for rule in &mut self.rules {
            if rule.id != *rule_id {
                continue;
            }

            rule.enabled = is_enabled;
            is_updated = true;
        }

        is_updated
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
}

impl PolicyState {
    /// Create one active policy.
    pub(crate) fn new(policy: Policy) -> Self {
        let enabled_rule_indices = Self::enabled_rule_indices(&policy);

        Self {
            spec: policy,
            enabled_rule_indices,
        }
    }

    /// Replace active policy.
    pub(crate) fn set_policy(&mut self, policy: Policy) -> RuntimeResult<()> {
        policy.validate()?;
        self.replace_policy(policy);

        Ok(())
    }

    /// Add one rule.
    pub(crate) fn add_rule(&mut self, rule: Rule) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.add_rule(rule)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Remove one rule.
    pub(crate) fn remove_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.remove_rule(rule_id)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Enable one rule.
    pub(crate) fn enable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.enable_rule(rule_id)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Disable one rule.
    pub(crate) fn disable_rule(&mut self, rule_id: &RuleId) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.disable_rule(rule_id)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Replace one rule.
    pub(crate) fn replace_rule(&mut self, rule_id: &RuleId, rule: Rule) -> RuntimeResult<()> {
        let mut next_policy = self.spec.clone();
        next_policy.replace_rule(rule_id, rule)?;
        self.replace_policy(next_policy);

        Ok(())
    }

    /// Decide one binding call.
    pub(crate) fn decide_binding(
        &self,
        subject: Subject<'_>,
        descriptor: BindingDescriptor,
    ) -> BindingDecision {
        let mut decision = self.spec.default.clone();

        for rule_index in &self.enabled_rule_indices {
            let rule = &self.spec.rules[*rule_index];
            let attempt = Attempt {
                binding: Some(descriptor),
            };
            if !super::matches_rule_selectors(rule, subject, attempt) {
                continue;
            }

            decision = rule.decision.clone();
            break;
        }

        Self::binding_decision(decision, descriptor)
    }

    /// Replace active policy and rebuild compiled state.
    fn replace_policy(&mut self, policy: Policy) {
        self.spec = policy;
        self.enabled_rule_indices = Self::enabled_rule_indices(&self.spec);
    }

    /// Return enabled rule indices.
    fn enabled_rule_indices(policy: &Policy) -> Vec<usize> {
        policy
            .rules
            .iter()
            .enumerate()
            .filter_map(|(index, rule)| rule.enabled.then_some(index))
            .collect()
    }

    /// Convert one policy decision into one binding decision.
    fn binding_decision(
        decision: super::Decision,
        descriptor: BindingDescriptor,
    ) -> BindingDecision {
        match decision {
            super::Decision::Allow { route } => {
                let route = if descriptor.provider == BindingProvider::Runtime {
                    BindingRoute::Host
                } else {
                    route
                };

                BindingDecision {
                    access: RuntimeAccess::Allow,
                    route,
                }
            }
            super::Decision::Deny => BindingDecision {
                access: RuntimeAccess::Deny,
                route: BindingRoute::Host,
            },
        }
    }
}
