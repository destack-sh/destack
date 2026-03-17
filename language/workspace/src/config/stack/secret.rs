use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::merge_metadata;

/// Secret options.
#[derive(Debug, Clone, Default)]
pub struct StackSecretOptions {
    /// External secret reference.
    pub reference: Option<String>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra secret metadata.
    pub config: Option<Value>,
}

impl StackSecretOptions {
    /// Inherit unset secret settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.reference.is_none() {
            self.reference = parent.reference.clone();
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

impl From<&StackSecretJson> for StackSecretOptions {
    fn from(json: &StackSecretJson) -> Self {
        Self {
            reference: json.reference.clone(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json.provider.clone(),
            config: json.config.clone(),
        }
    }
}

/// A passive secret reference node.
///
/// Inputs: one external secret reference.
/// Outputs: injectible secret bindings.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackSecretJson {
    /// External secret reference.
    pub reference: Option<String>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra secret metadata.
    pub config: Option<Value>,
}
