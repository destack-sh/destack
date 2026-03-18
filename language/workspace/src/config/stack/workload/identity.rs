use serde::Deserialize;
use serde_json::Value;

use super::super::common::{StackProviderJson, StackProviderOptions};

/// Workload identity options.
#[derive(Debug, Clone, Default)]
pub struct StackIdentityOptions {
    /// Stable workload identity name.
    pub name: Option<String>,
    /// Identity audience names.
    pub audiences: Vec<String>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Extra identity arguments.
    pub with: Option<Value>,
}

impl StackIdentityOptions {
    /// Inherit unset identity settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.name.is_none() {
            self.name = parent.name.clone();
        }
        if self.audiences.is_empty() {
            self.audiences = parent.audiences.clone();
        }
        self.provider.extend_from(&parent.provider);
        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&StackIdentityJson> for StackIdentityOptions {
    fn from(json: &StackIdentityJson) -> Self {
        Self {
            name: json.name.clone(),
            audiences: json.audiences.clone().unwrap_or_default(),
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Workload identity JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackIdentityJson {
    /// Stable workload identity name.
    pub name: Option<String>,
    /// Identity audience names.
    pub audiences: Option<Vec<String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Extra identity arguments.
    pub with: Option<Value>,
}
