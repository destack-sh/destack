use indexmap::IndexMap;
use serde::Deserialize;

use super::{
    DependencyJsonMap, DependencyMap, dependency_options_from_json, validate_dependency_json_map,
};

/// Named source graph tag options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagOptions {
    /// Human-readable tag description.
    pub description: Option<String>,
    /// Tag labels.
    pub labels: IndexMap<String, String>,
    /// Tag names included before this tag.
    pub extends: Vec<String>,
    /// Dependencies enabled by this tag.
    pub dependencies: DependencyMap,
}

impl TagOptions {
    /// Convert from one JSON tag.
    pub fn from_json(json: &TagJson) -> Self {
        Self {
            description: json.description.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            extends: json
                .extends
                .as_ref()
                .map(TagExtends::names)
                .unwrap_or_default(),
            dependencies: dependency_options_from_json(&json.dependencies),
        }
    }
}

/// Source graph tag JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TagJson {
    /// Human-readable tag description.
    pub description: Option<String>,
    /// Tag labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Tag names included before this tag.
    pub extends: Option<TagExtends>,
    /// Dependencies enabled by this tag.
    pub dependencies: Option<DependencyJsonMap>,
}

impl TagJson {
    /// Validate one source graph tag declaration.
    pub fn validate(&self) -> Result<(), String> {
        validate_dependency_json_map(self.dependencies.as_ref())
    }
}

/// Tag extends field from `destack.json`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum TagExtends {
    /// Extend one tag.
    One(String),
    /// Extend many tags in order.
    Many(Vec<String>),
}

impl TagExtends {
    /// Return the referenced tag names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(tag) => vec![tag.clone()],
            Self::Many(tags) => tags.clone(),
        }
    }
}
