use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_serde::Reflect;

use crate::diagnostic::RuntimeResult;

use super::{ActionSelector, Subject, SubjectSelector, TargetSelector};

/// Stable identifier for one runtime rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect)]
pub struct RuleId(pub String);

impl RuleId {
    /// Create one rule identifier from one string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Runtime policy decision payload.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    /// Allow the matching action.
    #[default]
    Allow,
    /// Deny the matching action.
    Deny,
}

/// One runtime policy rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Rule {
    /// Stable rule identifier.
    pub id: RuleId,
    /// Whether this rule is enabled.
    pub enabled: bool,
    /// Subject selector for this rule.
    pub subject: SubjectSelector,
    /// Action selector for this rule.
    pub action: ActionSelector,
    /// Target selector for this rule.
    pub target: TargetSelector,
    /// Decision selected by this rule.
    pub decision: Decision,
}

impl Rule {
    /// Create one enabled rule with no selectors.
    pub fn new(id: impl Into<String>, decision: Decision) -> Self {
        Self {
            id: RuleId::new(id),
            enabled: true,
            subject: SubjectSelector::default(),
            action: ActionSelector::default(),
            target: TargetSelector::default(),
            decision,
        }
    }

    /// Attach one subject selector to this rule.
    pub fn subject(mut self, selector: SubjectSelector) -> Self {
        self.subject = selector;
        self
    }

    /// Attach one action selector to this rule.
    pub fn action(mut self, selector: ActionSelector) -> Self {
        self.action = selector;
        self
    }

    /// Attach one target selector to this rule.
    pub fn target(mut self, selector: TargetSelector) -> Self {
        self.target = selector;
        self
    }

    /// Set rule enabled state.
    pub fn enabled(mut self, is_enabled: bool) -> Self {
        self.enabled = is_enabled;
        self
    }

    /// Create one enabled allow rule.
    pub fn allow(id: impl Into<String>, selector: ActionSelector) -> Self {
        Self::new(id, Decision::Allow).action(selector)
    }

    /// Create one enabled deny rule.
    pub fn deny(id: impl Into<String>, selector: ActionSelector) -> Self {
        Self::new(id, Decision::Deny).action(selector)
    }

    /// Return true when this rule matches one binding call.
    pub(crate) fn matches(
        &self,
        subject: Subject<'_>,
        program: &program::Program,
        binding: &program::Binding,
    ) -> RuntimeResult<bool> {
        if !self.subject.matches(subject)? {
            return Ok(false);
        }

        if !self.action.matches(program, binding)? {
            return Ok(false);
        }

        self.target.matches_binding()
    }
}
