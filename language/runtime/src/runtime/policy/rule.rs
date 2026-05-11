use serde::{Deserialize, Serialize};

use crate::runtime::binding::{RuntimeAccess, RuntimeWorld};
use destack_workspace::ReplayPayloadMode;

use super::{Fault, Hook, RuntimeSelector, Trigger};

/// Stable identifier for one runtime rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleId(pub String);

impl RuleId {
    /// Create one rule identifier from one string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Custom action payload routed to user-defined handlers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomAction {
    /// Stable custom action handler key.
    pub handler: String,
    /// Optional custom action payload.
    pub payload: Option<String>,
}

impl CustomAction {
    /// Create one custom action without payload.
    pub fn new(handler: impl Into<String>) -> Self {
        Self {
            handler: handler.into(),
            payload: None,
        }
    }

    /// Attach one custom payload string.
    pub fn payload(mut self, payload: impl Into<String>) -> Self {
        self.payload = Some(payload.into());
        self
    }
}

/// Runtime rule action payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleAction {
    /// Set the matching binding world.
    SetWorld {
        /// The selected world for matching bindings.
        world: RuntimeWorld,
    },
    /// Set the matching binding access mode.
    SetAccess {
        /// The selected access mode for matching bindings.
        access: RuntimeAccess,
    },
    /// Set the matching binding replay payload policy.
    SetReplay {
        /// The selected replay payload mode for matching bindings.
        payload: ReplayPayloadMode,
    },
    /// Apply one runtime fault action.
    Fault {
        /// Fault payload for this rule.
        fault: Fault,
    },
    /// Apply one user-defined custom action.
    Custom {
        /// Custom action payload.
        custom: CustomAction,
    },
}

impl RuleAction {
    /// Create one world decision action.
    pub fn set_world(world: RuntimeWorld) -> Self {
        Self::SetWorld { world }
    }

    /// Create one access decision action.
    pub fn set_access(access: RuntimeAccess) -> Self {
        Self::SetAccess { access }
    }

    /// Create one replay decision action.
    pub fn set_replay(payload: ReplayPayloadMode) -> Self {
        Self::SetReplay { payload }
    }

    /// Create one fault action.
    pub fn fault(fault: Fault) -> Self {
        Self::Fault { fault }
    }

    /// Create one custom action.
    pub fn custom(custom: CustomAction) -> Self {
        Self::Custom { custom }
    }
}

/// One runtime rule for binding decisions or triggered actions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rule {
    /// Stable rule identifier.
    pub id: RuleId,
    /// Whether this rule is enabled.
    pub enabled: bool,
    /// Optional selector for this rule.
    pub when: Option<RuntimeSelector>,
    /// Action payload for this rule.
    pub action: RuleAction,
    /// Trigger controls for action rules.
    /// Binding rules should leave this empty.
    pub trigger: Option<Trigger>,
}

impl Rule {
    /// Create one enabled rule with no selector and no trigger.
    pub fn new(id: impl Into<String>, action: RuleAction) -> Self {
        Self {
            id: RuleId::new(id),
            enabled: true,
            when: None,
            action,
            trigger: None,
        }
    }

    /// Attach one call selector to this rule.
    pub fn when(mut self, selector: RuntimeSelector) -> Self {
        self.when = Some(selector);
        self
    }

    /// Attach one trigger to this rule.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = Some(trigger);
        self
    }

    /// Set rule enabled state.
    pub fn enabled(mut self, is_enabled: bool) -> Self {
        self.enabled = is_enabled;
        self
    }

    /// Create one enabled access decision rule.
    pub fn access(id: impl Into<String>, selector: RuntimeSelector, access: RuntimeAccess) -> Self {
        Self::new(id, RuleAction::set_access(access)).when(selector)
    }

    /// Create one enabled world decision rule.
    pub fn world(id: impl Into<String>, selector: RuntimeSelector, world: RuntimeWorld) -> Self {
        Self::new(id, RuleAction::set_world(world)).when(selector)
    }

    /// Create one enabled replay decision rule.
    pub fn replay(
        id: impl Into<String>,
        selector: RuntimeSelector,
        payload: ReplayPayloadMode,
    ) -> Self {
        Self::new(id, RuleAction::set_replay(payload)).when(selector)
    }

    /// Create one enabled fault rule with one trigger.
    pub fn fault(id: impl Into<String>, fault: Fault, trigger: Trigger) -> Self {
        Self::new(id, RuleAction::fault(fault)).trigger(trigger)
    }

    /// Create one enabled custom-action rule with one trigger.
    pub fn custom(id: impl Into<String>, custom: CustomAction, trigger: Trigger) -> Self {
        Self::new(id, RuleAction::custom(custom)).trigger(trigger)
    }

    /// Return true when this rule trigger matches one hook.
    pub fn matches_hook(&self, hook: Hook) -> bool {
        let Some(trigger) = &self.trigger else {
            return false;
        };

        trigger.on == hook
    }
}
