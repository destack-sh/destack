use indexmap::IndexMap;
use serde::Deserialize;

use super::{
    DependencyJsonMap, DependencyMap, dependency_options_from_json, validate_dependency_json_map,
};

/// Named source graph feature options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureOptions {
    /// Human-readable feature description.
    pub description: Option<String>,
    /// Feature labels.
    pub labels: IndexMap<String, String>,
    /// Feature names included before this feature.
    pub extends: Vec<String>,
    /// Dependencies enabled by this feature.
    pub dependencies: DependencyMap,
}

impl FeatureOptions {
    /// Convert from one JSON feature.
    pub fn from_json(json: &FeatureJson) -> Self {
        Self {
            description: json.description.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            extends: json
                .extends
                .as_ref()
                .map(FeatureExtends::names)
                .unwrap_or_default(),
            dependencies: dependency_options_from_json(&json.dependencies),
        }
    }
}

/// Source graph feature JSON from `destack.json`.
#[derive(Debug, Clone, Default, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FeatureJson {
    /// Human-readable feature description.
    pub description: Option<String>,
    /// Feature labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Feature names included before this feature.
    pub extends: Option<FeatureExtends>,
    /// Dependencies enabled by this feature.
    pub dependencies: Option<DependencyJsonMap>,
}

impl FeatureJson {
    /// Validate one source graph feature declaration.
    pub fn validate(&self) -> Result<(), String> {
        validate_dependency_json_map(self.dependencies.as_ref())
    }
}

/// Feature extends field from `destack.json`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum FeatureExtends {
    /// Extend one feature.
    One(String),
    /// Extend many features in order.
    Many(Vec<String>),
}

impl FeatureExtends {
    /// Return the referenced feature names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(feature) => vec![feature.clone()],
            Self::Many(features) => features.clone(),
        }
    }
}
