use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

/// Active source graph and runtime selection conditions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
