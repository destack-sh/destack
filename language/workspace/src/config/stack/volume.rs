use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::merge_metadata;

/// Volume options.
#[derive(Debug, Clone, Default)]
pub struct StackVolumeOptions {
    /// Capacity or size hint.
    pub size: Option<String>,
    /// Storage class or tier.
    pub class: Option<String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra volume metadata.
    pub config: Option<Value>,
}

impl StackVolumeOptions {
    /// Inherit unset volume settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.size.is_none() {
            self.size = parent.size.clone();
        }
        if self.class.is_none() {
            self.class = parent.class.clone();
        }
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackVolumeJson> for StackVolumeOptions {
    fn from(json: &StackVolumeJson) -> Self {
        Self {
            size: json.size.clone(),
            class: json.class.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json.provider.clone(),
            config: json.config.clone(),
        }
    }
}

/// A passive mountable storage node.
///
/// Inputs: size, class, and provider config.
/// Outputs: mount handles for workloads.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackVolumeJson {
    /// Capacity or size hint.
    pub size: Option<String>,
    /// Storage class or tier.
    pub class: Option<String>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra volume metadata.
    pub config: Option<Value>,
}
