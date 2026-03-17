use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::merge_metadata;

/// Config options.
#[derive(Debug, Clone, Default)]
pub struct StackConfigOptions {
    /// Source file path.
    pub file: Option<String>,
    /// Inline config data.
    pub data: IndexMap<String, String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra config metadata.
    pub config: Option<Value>,
}

impl StackConfigOptions {
    /// Inherit unset config settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.file.is_none() {
            self.file = parent.file.clone();
        }
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
        if self.data.is_empty() {
            self.data = parent.data.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackConfigJson> for StackConfigOptions {
    fn from(json: &StackConfigJson) -> Self {
        Self {
            file: json.file.clone(),
            data: json.data.clone().unwrap_or_default(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json.provider.clone(),
            config: json.config.clone(),
        }
    }
}

/// A passive configuration node.
///
/// Inputs: inline data or one file source.
/// Outputs: mountable or bindable configuration handles.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackConfigJson {
    /// Source file path.
    pub file: Option<String>,
    /// Inline config data.
    pub data: Option<IndexMap<String, String>>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra config metadata.
    pub config: Option<Value>,
}
