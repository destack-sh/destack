use serde::{Deserialize, Serialize};

use crate::host::binding::BindingRoute;

use super::{ActionSelector, SubjectSelector, TargetSelector};

/// Stable identifier for one runtime rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleId(pub String);

impl RuleId {
    /// Create one rule identifier from one string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Runtime policy decision payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    /// Allow the matching action through one route.
    Allow {
        /// Route selected for matching host actions.
        route: BindingRoute,
    },
    /// Deny the matching action.
    Deny,
}

impl Default for Decision {
    fn default() -> Self {
        Self::allow()
    }
}

impl Decision {
    /// Create one allow decision using the host route.
    pub fn allow() -> Self {
        Self::Allow {
            route: BindingRoute::Host,
        }
    }

    /// Create one allow decision using an explicit route.
    pub fn route(route: BindingRoute) -> Self {
        Self::Allow { route }
    }
}

/// One runtime policy rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
        Self::new(id, Decision::allow()).action(selector)
    }

    /// Create one enabled deny rule.
    pub fn deny(id: impl Into<String>, selector: ActionSelector) -> Self {
        Self::new(id, Decision::Deny).action(selector)
    }

    /// Create one enabled route rule.
    pub fn route(id: impl Into<String>, selector: ActionSelector, route: BindingRoute) -> Self {
        Self::new(id, Decision::route(route)).action(selector)
    }
}
