use std::hash::{Hash, Hasher};

use destack_artifact::{Host, Platform, Runtime};
use destack_source::matches as glob_matches;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

/// Active source graph and runtime selection conditions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionSet {
    /// Active source graph modes.
    pub modes: IndexSet<String>,
    /// Active source graph roles.
    pub roles: IndexSet<String>,
    /// Active optional features.
    pub features: IndexSet<String>,
    /// Active source graph tags.
    pub tags: IndexSet<String>,
    /// Active build target.
    pub target: Option<String>,
    /// Active product.
    pub product: Option<String>,
    /// Active target platform.
    pub platform: Option<Platform>,
    /// Active host environment.
    pub host: Option<Host>,
    /// Active runtime.
    pub runtime: Option<Runtime>,
}

impl Hash for ConditionSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_condition_names(&self.modes, state);
        hash_condition_names(&self.roles, state);
        hash_condition_names(&self.features, state);
        hash_condition_names(&self.tags, state);
        self.target.hash(state);
        self.product.hash(state);
        self.platform.hash(state);
        self.host.hash(state);
        self.runtime.hash(state);
    }
}

impl ConditionSet {
    /// Return whether this set contains one mode.
    pub fn contains_mode(&self, name: &str) -> bool {
        self.modes.contains(name)
    }

    /// Return whether this set contains one role.
    pub fn contains_role(&self, name: &str) -> bool {
        self.roles.contains(name)
    }

    /// Return whether this set contains one feature.
    pub fn contains_feature(&self, name: &str) -> bool {
        self.features.contains(name)
    }

    /// Return whether this set contains one tag.
    pub fn contains_tag(&self, name: &str) -> bool {
        self.tags.contains(name)
    }
}

/// Named condition declarations from `destack.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionOptions {
    /// Named source graph modes.
    pub modes: IndexMap<String, super::ModeOptions>,
    /// Named source graph roles.
    pub roles: IndexMap<String, super::RoleOptions>,
    /// Named optional source graph features.
    pub features: IndexMap<String, super::FeatureOptions>,
    /// Named source graph tags.
    pub tags: IndexMap<String, super::TagOptions>,
    /// Named file condition aliases.
    pub aliases: IndexMap<String, ConditionGate>,
}

/// Condition declarations JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConditionOptionsJson {
    /// Named source graph modes.
    pub modes: Option<IndexMap<String, super::ModeJson>>,
    /// Named source graph roles.
    pub roles: Option<IndexMap<String, super::RoleJson>>,
    /// Named optional source graph features.
    pub features: Option<IndexMap<String, super::FeatureJson>>,
    /// Named source graph tags.
    pub tags: Option<IndexMap<String, super::TagJson>>,
    /// Named file condition aliases.
    pub aliases: Option<IndexMap<String, ConditionGateJson>>,
}

/// Condition gate JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConditionGateJson {
    /// Active mode selector.
    pub mode: Option<ConditionSelectorJson>,
    /// Active role selector.
    pub role: Option<ConditionSelectorJson>,
    /// Active feature selector.
    pub feature: Option<ConditionSelectorJson>,
    /// Active tag selector.
    pub tag: Option<ConditionSelectorJson>,
    /// Active build target selector.
    pub target: Option<ConditionSelectorJson>,
    /// Active product selector.
    pub product: Option<ConditionSelectorJson>,
    /// Active target platform selector.
    pub platform: Option<ConditionSelectorJson>,
    /// Active host environment selector.
    pub host: Option<ConditionSelectorJson>,
    /// Active runtime selector.
    pub runtime: Option<ConditionSelectorJson>,
}

/// Predicate over active source graph and runtime conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ConditionGate {
    /// Active mode selector.
    pub mode: Option<ConditionSelector>,
    /// Active role selector.
    pub role: Option<ConditionSelector>,
    /// Active feature selector.
    pub feature: Option<ConditionSelector>,
    /// Active tag selector.
    pub tag: Option<ConditionSelector>,
    /// Active build target selector.
    pub target: Option<ConditionSelector>,
    /// Active product selector.
    pub product: Option<ConditionSelector>,
    /// Active target platform selector.
    pub platform: Option<ConditionSelector>,
    /// Active host environment selector.
    pub host: Option<ConditionSelector>,
    /// Active runtime selector.
    pub runtime: Option<ConditionSelector>,
}

impl ConditionGate {
    /// Create a gate that matches one active mode.
    pub fn mode(name: impl Into<String>) -> Self {
        Self {
            mode: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Create a gate that matches one active role.
    pub fn role(name: impl Into<String>) -> Self {
        Self {
            role: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Create a gate that matches one active feature.
    pub fn feature(name: impl Into<String>) -> Self {
        Self {
            feature: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Create a gate that matches one active tag.
    pub fn tag(name: impl Into<String>) -> Self {
        Self {
            tag: Some(ConditionSelector::exact(name)),
            ..Self::default()
        }
    }

    /// Convert from one JSON condition gate declaration.
    pub fn from_json(json: &ConditionGateJson) -> Self {
        Self {
            mode: json.mode.as_ref().map(ConditionSelector::from),
            role: json.role.as_ref().map(ConditionSelector::from),
            feature: json.feature.as_ref().map(ConditionSelector::from),
            tag: json.tag.as_ref().map(ConditionSelector::from),
            target: json.target.as_ref().map(ConditionSelector::from),
            product: json.product.as_ref().map(ConditionSelector::from),
            platform: json.platform.as_ref().map(ConditionSelector::from),
            host: json.host.as_ref().map(ConditionSelector::from),
            runtime: json.runtime.as_ref().map(ConditionSelector::from),
        }
    }

    /// Return whether this gate has no selectors.
    pub fn is_empty(&self) -> bool {
        self.mode.as_ref().is_none_or(ConditionSelector::is_empty)
            && self.role.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .feature
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self.tag.as_ref().is_none_or(ConditionSelector::is_empty)
            && self.target.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .product
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self
                .platform
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
            && self.host.as_ref().is_none_or(ConditionSelector::is_empty)
            && self
                .runtime
                .as_ref()
                .is_none_or(ConditionSelector::is_empty)
    }

    /// Return true when this gate matches one active condition set.
    pub fn matches(&self, conditions: &ConditionSet) -> bool {
        // source graph sets
        let source_graph_matches = self.matches_set(&self.mode, &conditions.modes)
            && self.matches_set(&self.role, &conditions.roles)
            && self.matches_set(&self.feature, &conditions.features)
            && self.matches_set(&self.tag, &conditions.tags);

        // profile selectors
        let profile_matches = self.matches_optional(&self.target, conditions.target.as_deref())
            && self.matches_optional(&self.product, conditions.product.as_deref())
            && self.matches_optional(
                &self.platform,
                conditions.platform.map(|platform| platform.canonical_tag()),
            )
            && self.matches_optional(&self.host, conditions.host.map(|host| host.canonical_tag()))
            && self.matches_optional(
                &self.runtime,
                conditions.runtime.map(|runtime| runtime.canonical_tag()),
            );

        source_graph_matches && profile_matches
    }

    /// Return true when an optional selector matches one set axis.
    fn matches_set(&self, selector: &Option<ConditionSelector>, names: &IndexSet<String>) -> bool {
        let Some(selector) = selector else {
            return true;
        };

        selector.matches_any(names.iter().map(String::as_str))
    }

    /// Return true when an optional selector matches one scalar axis.
    fn matches_optional(&self, selector: &Option<ConditionSelector>, name: Option<&str>) -> bool {
        let Some(selector) = selector else {
            return true;
        };
        let Some(name) = name else {
            return false;
        };

        selector.matches(name)
    }
}

/// Return the built-in condition aliases.
pub fn builtin_condition_aliases() -> IndexMap<String, ConditionGate> {
    let mut aliases = IndexMap::new();

    for name in super::builtin_modes().keys() {
        aliases.insert(name.clone(), ConditionGate::mode(name.clone()));
    }

    for name in super::builtin_roles().keys() {
        aliases.insert(name.clone(), ConditionGate::role(name.clone()));
    }

    aliases
}

/// Selector over one active condition axis.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ConditionSelector {
    /// Condition names or glob patterns.
    pub patterns: Vec<String>,
}

impl ConditionSelector {
    /// Create an exact name selector.
    pub fn exact(name: impl Into<String>) -> Self {
        Self {
            patterns: vec![name.into()],
        }
    }

    /// Return whether this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Return whether this selector matches one condition name.
    pub fn matches(&self, name: &str) -> bool {
        self.patterns.is_empty()
            || self
                .patterns
                .iter()
                .any(|pattern| glob_match(pattern, name))
    }

    /// Return whether this selector matches any condition name.
    pub fn matches_any<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> bool {
        self.patterns.is_empty() || names.into_iter().any(|name| self.matches(name))
    }
}

/// Condition selector JSON accepted by configuration files.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ConditionSelectorJson {
    /// Condition name shorthand.
    Pattern(String),
    /// Condition name shorthand list.
    Patterns(Vec<String>),
    /// Structured condition selector.
    Selector(ConditionSelectorJsonObject),
}

/// Structured condition selector JSON accepted by configuration files.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConditionSelectorJsonObject {
    /// Condition name glob patterns.
    pub patterns: Vec<String>,
}

impl From<&ConditionSelectorJson> for ConditionSelector {
    fn from(value: &ConditionSelectorJson) -> Self {
        match value {
            ConditionSelectorJson::Pattern(pattern) => Self {
                patterns: vec![pattern.clone()],
            },
            ConditionSelectorJson::Patterns(patterns) => Self {
                patterns: patterns.clone(),
            },
            ConditionSelectorJson::Selector(selector) => Self {
                patterns: selector.patterns.clone(),
            },
        }
    }
}

/// Hash source graph condition names independent of insertion order.
fn hash_condition_names<H: Hasher>(names: &IndexSet<String>, state: &mut H) {
    let mut names = names.iter().collect::<Vec<_>>();
    names.sort();

    names.len().hash(state);
    names.iter().for_each(|name| name.hash(state));
}

/// Return true when one glob-like pattern matches text.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}

#[cfg(test)]
mod tests {
    use destack_artifact::{Host, Platform, Runtime};

    use super::*;

    #[test]
    fn test_match_condition_gate_across_axes() {
        let mut conditions = ConditionSet::default();
        conditions.modes.insert("test".to_string());
        conditions.features.insert("payments".to_string());
        conditions.target = Some("web".to_string());
        conditions.product = Some("shop".to_string());
        conditions.platform = Some(Platform::Linux);
        conditions.host = Some(Host::Browser);
        conditions.runtime = Some(Runtime::Js);

        // require every selected axis
        let gate = ConditionGate {
            mode: Some(ConditionSelector::exact("test")),
            feature: Some(ConditionSelector::exact("payments")),
            target: Some(ConditionSelector::exact("web")),
            product: Some(ConditionSelector::exact("shop")),
            platform: Some(ConditionSelector::exact("linux")),
            host: Some(ConditionSelector::exact("browser")),
            runtime: Some(ConditionSelector::exact("js")),
            ..ConditionGate::default()
        };

        assert!(gate.matches(&conditions));
    }

    #[test]
    fn test_match_condition_gate_requires_missing_axis() {
        let conditions = ConditionSet::default();
        let gate = ConditionGate {
            host: Some(ConditionSelector::exact("browser")),
            ..ConditionGate::default()
        };

        assert!(!gate.matches(&conditions));
    }
}
