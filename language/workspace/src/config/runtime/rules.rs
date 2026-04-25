use serde::{Deserialize, Serialize};

use super::super::policy::{ReplayPayloadMode, ReplayPayloadModeJson};
use super::{RuntimeSelector, RuntimeSelectorJson};

/// Runtime world selection for external bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuntimeWorld {
    /// Use host-backed platform bindings.
    #[default]
    Host,
    /// Use simulation-backed platform bindings.
    Simulation,
}

/// Runtime access policy for binding execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum RuntimeAccess {
    /// Allow the binding call.
    #[default]
    Allow,
    /// Deny the binding call.
    Deny,
}
/// Static runtime rule for the Destack config.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeRule {
    /// Rule filter clause.
    pub when: RuntimeSelector,
    /// Access decision for matching bindings.
    pub access: Option<RuntimeAccess>,
    /// World decision for matching bindings.
    pub world: Option<RuntimeWorld>,
    /// Replay payload decision for matching bindings.
    pub replay: Option<ReplayPayloadMode>,
}

/// Runtime world selection for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeWorldJson {
    /// Use host-backed platform bindings.
    Host,
    /// Use simulation-backed platform bindings.
    Simulation,
}

impl From<RuntimeWorldJson> for RuntimeWorld {
    fn from(value: RuntimeWorldJson) -> Self {
        match value {
            RuntimeWorldJson::Host => RuntimeWorld::Host,
            RuntimeWorldJson::Simulation => RuntimeWorld::Simulation,
        }
    }
}

/// Runtime default access policy for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RuntimeAccessJson {
    /// Allow matching binding calls.
    Allow,
    /// Deny matching binding calls.
    Deny,
}

impl From<RuntimeAccessJson> for RuntimeAccess {
    fn from(value: RuntimeAccessJson) -> Self {
        match value {
            RuntimeAccessJson::Allow => RuntimeAccess::Allow,
            RuntimeAccessJson::Deny => RuntimeAccess::Deny,
        }
    }
}
/// Runtime static rule for JSON deserialization.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRuleJson {
    /// Rule filter clause.
    pub when: RuntimeRuleWhenJson,
    /// Access decision for matching bindings.
    pub access: Option<RuntimeAccessJson>,
    /// World decision for matching bindings.
    pub world: Option<RuntimeWorldJson>,
    /// Replay payload decision for matching bindings.
    pub replay: Option<ReplayPayloadModeJson>,
}

/// Runtime rule selector clause for JSON deserialization.
#[allow(clippy::large_enum_variant)] // NOTE #Performance #Cleanup: keep the json selector surface flat until this schema settles
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum RuntimeRuleWhenJson {
    /// Shorthand binding glob selector.
    Binding(String),
    /// Shorthand function glob selector.
    Function(String),
    /// Full selector object.
    Selector(RuntimeSelectorJson),
}

impl From<&RuntimeRuleWhenJson> for RuntimeSelector {
    fn from(value: &RuntimeRuleWhenJson) -> Self {
        match value {
            RuntimeRuleWhenJson::Binding(pattern) => RuntimeSelector::binding(pattern.clone()),
            RuntimeRuleWhenJson::Function(pattern) => RuntimeSelector::function(pattern.clone()),
            RuntimeRuleWhenJson::Selector(selector) => RuntimeSelector::from(selector),
        }
    }
}

impl From<&RuntimeRuleJson> for RuntimeRule {
    fn from(value: &RuntimeRuleJson) -> Self {
        Self {
            when: RuntimeSelector::from(&value.when),
            access: value.access.map(RuntimeAccess::from),
            world: value.world.map(RuntimeWorld::from),
            replay: value.replay.map(ReplayPayloadMode::from),
        }
    }
}
