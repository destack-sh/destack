use serde::{Deserialize, Serialize};

use super::rules::{
    RuntimeAccess, RuntimeAccessJson, RuntimeRule, RuntimeRuleJson, RuntimeWorld, RuntimeWorldJson,
};

/// Source for one runtime effect domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum EffectSource {
    /// Resolve facts from the live host.
    #[default]
    Host,
    /// Resolve facts from the active trace log.
    Trace,
    /// Resolve facts from the configured simulation backend.
    Simulation,
    /// Forbid this interaction domain.
    Deny,
}

/// Runtime effect configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct EffectOptions {
    /// Default binding backend when no rule matches.
    pub backend: RuntimeWorld,
    /// Default access policy for bindings without a matching access rule.
    pub access: RuntimeAccess,
    /// Ordered binding routing rules.
    pub rules: Vec<RuntimeRule>,
    /// Source for time facts.
    pub time: EffectSource,
    /// Source for randomness facts.
    pub random: EffectSource,
}

/// Runtime effect configuration for JSON deserialization.
#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct EffectOptionsJson {
    /// Default binding backend when no rule matches.
    pub backend: Option<RuntimeWorldJson>,
    /// Default access policy for bindings without matching access rules.
    pub access: Option<RuntimeAccessJson>,
    /// Ordered binding routing rules.
    pub rules: Option<Vec<RuntimeRuleJson>>,
    /// Source for time facts.
    pub time: Option<EffectSourceJson>,
    /// Source for randomness facts.
    pub random: Option<EffectSourceJson>,
}

impl EffectOptionsJson {
    /// Inherit unset effect settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.backend.is_none() {
            self.backend = parent.backend;
        }

        if self.access.is_none() {
            self.access = parent.access;
        }

        if self.rules.is_none() {
            self.rules = parent.rules.clone();
        }

        if self.time.is_none() {
            self.time = parent.time;
        }

        if self.random.is_none() {
            self.random = parent.random;
        }
    }

    /// Apply effect overrides to one base set of options.
    pub fn apply_to(&self, options: &mut EffectOptions) {
        // defaults
        if let Some(backend) = self.backend {
            options.backend = backend.into();
        }

        if let Some(default_access) = self.access {
            options.access = default_access.into();
        }

        // rules
        if let Some(rules) = &self.rules {
            options.rules = rules.iter().map(RuntimeRule::from).collect();
        }

        // domain sources
        if let Some(time) = &self.time {
            options.time = (*time).into();
        }

        if let Some(random) = &self.random {
            options.random = (*random).into();
        }
    }
}

/// Effect source for JSON deserialization.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EffectSourceJson {
    /// Resolve facts from the live host.
    Host,
    /// Resolve facts from the active trace log.
    Trace,
    /// Resolve facts from the configured simulation backend.
    Simulation,
    /// Forbid this interaction domain.
    Deny,
}

impl From<EffectSourceJson> for EffectSource {
    fn from(value: EffectSourceJson) -> Self {
        match value {
            EffectSourceJson::Host => EffectSource::Host,
            EffectSourceJson::Trace => EffectSource::Trace,
            EffectSourceJson::Simulation => EffectSource::Simulation,
            EffectSourceJson::Deny => EffectSource::Deny,
        }
    }
}
