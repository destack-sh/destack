use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

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
}

impl Hash for ConditionSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_condition_names(&self.modes, state);
        hash_condition_names(&self.roles, state);
        hash_condition_names(&self.features, state);
        hash_condition_names(&self.tags, state);
        self.target.hash(state);
        self.product.hash(state);
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

/// Hash source graph condition names independent of insertion order.
fn hash_condition_names<H: Hasher>(names: &IndexSet<String>, state: &mut H) {
    let mut names = names.iter().collect::<Vec<_>>();
    names.sort();

    names.len().hash(state);
    names.iter().for_each(|name| name.hash(state));
}

/// Policy selector over active source graph and runtime conditions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ConditionSelector {
    /// Condition names or glob patterns.
    pub patterns: Vec<String>,
}

impl ConditionSelector {
    /// Return whether this selector has no clauses.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

/// Condition selector accepted by static policy JSON.
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

/// Structured condition selector accepted by static policy JSON.
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
