use serde::{Deserialize, Serialize};

use destack_workspace as workspace;
use workspace::{ReplayPayloadMode, RuntimeAccess, RuntimeSelector, RuntimeWorld};

use super::{Fault, Hook, Trigger};

/// Stable identifier for one runtime rule.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(pub String);

/// Runtime rule action payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
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
}

impl From<&workspace::RuntimePolicyAction> for Effect {
    fn from(value: &workspace::RuntimePolicyAction) -> Self {
        match value {
            workspace::RuntimePolicyAction::SetAccess { access } => {
                Self::SetAccess { access: *access }
            }
            workspace::RuntimePolicyAction::SetWorld { world } => Self::SetWorld { world: *world },
        }
    }
}

/// One runtime rule for world, access, or fault effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rule {
    /// Stable rule identifier.
    pub id: RuleId,
    /// Whether this rule is enabled.
    pub enabled: bool,
    /// Rule filter clause.
    pub when: RuntimeSelector,
    /// Action payload for this rule.
    pub action: Effect,
    /// Trigger controls for effect rules.
    /// Dispatch rules should leave this empty.
    pub trigger: Option<Trigger>,
}

impl Rule {
    /// Convert one workspace static policy rule into one runtime rule.
    pub fn from_workspace_policy_rule(index: usize, rule: &workspace::RuntimePolicyRule) -> Self {
        Self {
            id: RuleId(format!("dsconfig.runtime.rule.{index}")),
            enabled: true,
            when: rule.when.clone(),
            action: Effect::from(&rule.action),
            trigger: None,
        }
    }

    /// Return true when this rule trigger matches one hook.
    pub fn matches_hook(&self, hook: Hook) -> bool {
        let Some(trigger) = &self.trigger else {
            return false;
        };

        trigger.on == hook
    }
}
