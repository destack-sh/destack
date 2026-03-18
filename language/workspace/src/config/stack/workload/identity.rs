use serde::Deserialize;
use serde_json::Value;

/// Workload identity options.
#[derive(Debug, Clone, Default)]
pub struct StackIdentityOptions {
    /// Stable workload identity name.
    pub name: Option<String>,
    /// Identity audience names.
    pub audiences: Vec<String>,
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra identity metadata.
    pub config: Option<Value>,
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
        if self.provider.is_none() {
            self.provider = parent.provider.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
    }
}

impl From<&StackIdentityJson> for StackIdentityOptions {
    fn from(json: &StackIdentityJson) -> Self {
        Self {
            name: json.name.clone(),
            audiences: json.audiences.clone().unwrap_or_default(),
            provider: json.provider.clone(),
            config: json.config.clone(),
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
    /// Provider-specific lowering overrides.
    pub provider: Option<Value>,
    /// Extra identity metadata.
    pub config: Option<Value>,
}
