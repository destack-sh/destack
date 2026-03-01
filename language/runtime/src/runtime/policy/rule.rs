use serde::{Deserialize, Serialize};

use crate::runtime::bindings::{BindingBlocking, BindingEffect, BindingEngine, BindingScope};
use destack_workspace as workspace;
use workspace::{ExecutionMode, ReplayPayloadMode, RuntimeAccess, RuntimeWorld};

use super::{Fault, Trigger};

/// Filter clauses for matching runtime bindings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Selector {
    /// Glob selector for full binding names.
    pub binding: Option<String>,
    /// Glob selector for capability names.
    pub capability: Option<String>,
    /// Glob selector for component names.
    pub component: Option<String>,
    /// Glob selector for module names.
    pub module: Option<String>,
    /// Engine selector.
    pub engine: Option<BindingEngine>,
    /// Execution modes selector.
    pub execution_modes: Option<Vec<ExecutionMode>>,
    /// Platform selector.
    pub platforms: Option<Vec<String>>,
    /// Binding scope selector.
    pub scope: Option<BindingScope>,
    /// Binding blocking selector.
    pub blocking: Option<BindingBlocking>,
    /// Binding effect selector.
    pub effect: Option<BindingEffect>,
    /// Agent selector.
    pub agent_ids: Option<Vec<u64>>,
}

impl Selector {
    /// Whether this selector has no filtering clauses.
    pub fn is_empty(&self) -> bool {
        self.binding.is_none()
            && self.capability.is_none()
            && self.component.is_none()
            && self.module.is_none()
            && self.engine.is_none()
            && self.execution_modes.is_none()
            && self.platforms.is_none()
            && self.scope.is_none()
            && self.blocking.is_none()
            && self.effect.is_none()
            && self.agent_ids.is_none()
    }
}

impl From<&workspace::RuntimeFilter> for Selector {
    fn from(value: &workspace::RuntimeFilter) -> Self {
        Self {
            binding: value.binding.clone(),
            capability: value.capability.clone(),
            component: value.component.clone(),
            module: value.module.clone(),
            engine: value.engine,
            execution_modes: value.execution_modes.clone(),
            platforms: value.platforms.clone(),
            scope: value.scope,
            blocking: value.blocking,
            effect: value.effect,
            agent_ids: None,
        }
    }
}

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

/// Runtime mutation action payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PolicyMutation {
    /// Install one runtime rule into the mutable rule set.
    InstallRule {
        /// Rule payload to install.
        rule: Rule,
    },
    /// Remove one runtime rule from the mutable rule set.
    RemoveRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Enable one runtime rule in the mutable rule set.
    EnableRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
    /// Disable one runtime rule in the mutable rule set.
    DisableRule {
        /// Stable rule identifier.
        rule_id: RuleId,
    },
}

/// One runtime rule for world, access, or fault effects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rule {
    /// Stable rule identifier.
    pub id: RuleId,
    /// Rule filter clause.
    pub when: Selector,
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
            when: Selector::from(&rule.when),
            action: Effect::from(&rule.action),
            trigger: None,
        }
    }
}
