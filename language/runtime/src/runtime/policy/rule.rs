use serde::{Deserialize, Serialize};

use destack_workspace as workspace;
use workspace::{ReplayPayloadMode, RuntimeAccess, RuntimeSelector, RuntimeWorld};

use super::{Fault, Hook, Trigger};

/// Stable identifier for one runtime rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuleId(pub String);

impl RuleId {
    /// Create one rule identifier from one string value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Custom effect payload routed to user-defined handlers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomEffect {
    /// Stable custom effect handler key.
    pub handler: String,
    /// Optional custom effect payload.
    pub payload: Option<String>,
}

impl CustomEffect {
    /// Create one custom effect without payload.
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
pub enum Effect {
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
    /// Apply one runtime fault effect.
    Fault {
        /// Fault payload for this rule.
        fault: Fault,
    },
    /// Apply one user-defined custom effect.
    Custom {
        /// Custom effect payload.
        custom: CustomEffect,
    },
}

impl Effect {
    /// Create one world-dispatch effect.
    pub fn set_world(world: RuntimeWorld) -> Self {
        Self::SetWorld { world }
    }

    /// Create one access-dispatch effect.
    pub fn set_access(access: RuntimeAccess) -> Self {
        Self::SetAccess { access }
    }

    /// Create one replay-dispatch effect.
    pub fn set_replay(payload: ReplayPayloadMode) -> Self {
        Self::SetReplay { payload }
    }

    /// Create one fault effect.
    pub fn fault(fault: Fault) -> Self {
        Self::Fault { fault }
    }

    /// Create one custom effect.
    pub fn custom(custom: CustomEffect) -> Self {
        Self::Custom { custom }
    }
}

/// One runtime rule for world, access, or fault effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rule {
    /// Stable rule identifier.
    pub id: RuleId,
    /// Whether this rule is enabled.
    pub enabled: bool,
    /// Optional call-plane selector.
    pub when: Option<RuntimeSelector>,
    /// Action payload for this rule.
    pub action: Effect,
    /// Trigger controls for effect rules.
    /// Dispatch rules should leave this empty.
    pub trigger: Option<Trigger>,
}

impl Rule {
    /// Create one enabled rule with no selector and no trigger.
    pub fn new(id: impl Into<String>, action: Effect) -> Self {
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

    /// Create one enabled access-dispatch rule.
    pub fn access(id: impl Into<String>, selector: RuntimeSelector, access: RuntimeAccess) -> Self {
        Self::new(id, Effect::set_access(access)).when(selector)
    }

    /// Create one enabled world-dispatch rule.
    pub fn world(id: impl Into<String>, selector: RuntimeSelector, world: RuntimeWorld) -> Self {
        Self::new(id, Effect::set_world(world)).when(selector)
    }

    /// Create one enabled replay-dispatch rule.
    pub fn replay(
        id: impl Into<String>,
        selector: RuntimeSelector,
        payload: ReplayPayloadMode,
    ) -> Self {
        Self::new(id, Effect::set_replay(payload)).when(selector)
    }

    /// Create one enabled fault rule with one trigger.
    pub fn fault(id: impl Into<String>, fault: Fault, trigger: Trigger) -> Self {
        Self::new(id, Effect::fault(fault)).trigger(trigger)
    }

    /// Create one enabled custom-effect rule with one trigger.
    pub fn custom(id: impl Into<String>, custom: CustomEffect, trigger: Trigger) -> Self {
        Self::new(id, Effect::custom(custom)).trigger(trigger)
    }

    /// Convert one workspace static rule into runtime rules.
    pub fn from_workspace_rule(index: usize, rule: &workspace::RuntimeRule) -> Vec<Self> {
        let mut rules = Vec::new();

        if let Some(access) = rule.access {
            rules.push(Self {
                id: RuleId(format!("dsconfig.runtime.rule.{index}.access")),
                enabled: true,
                when: Some(rule.when.clone()),
                action: Effect::SetAccess { access },
                trigger: None,
            });
        }

        if let Some(world) = rule.world {
            rules.push(Self {
                id: RuleId(format!("dsconfig.runtime.rule.{index}.world")),
                enabled: true,
                when: Some(rule.when.clone()),
                action: Effect::SetWorld { world },
                trigger: None,
            });
        }

        if let Some(payload) = rule.replay {
            rules.push(Self {
                id: RuleId(format!("dsconfig.runtime.rule.{index}.replay")),
                enabled: true,
                when: Some(rule.when.clone()),
                action: Effect::SetReplay { payload },
                trigger: None,
            });
        }

        rules
    }

    /// Return true when this rule trigger matches one hook.
    pub fn matches_hook(&self, hook: Hook) -> bool {
        let Some(trigger) = &self.trigger else {
            return false;
        };

        trigger.on == hook
    }
}
