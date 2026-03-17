use serde::Deserialize;

/// Binding configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackBindingOptions {
    /// Bound workload name.
    pub workload: Option<String>,
    /// Bound service name.
    pub service: Option<String>,
    /// Bound config name.
    pub config: Option<String>,
    /// Bound secret name.
    pub secret: Option<String>,
    /// Requested output field.
    pub field: Option<String>,
}

impl StackBindingOptions {
    /// Inherit unset binding settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.workload.is_none() {
            self.workload = parent.workload.clone();
        }
        if self.service.is_none() {
            self.service = parent.service.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
        if self.secret.is_none() {
            self.secret = parent.secret.clone();
        }
        if self.field.is_none() {
            self.field = parent.field.clone();
        }
    }
}

impl From<&StackBindingJson> for StackBindingOptions {
    fn from(json: &StackBindingJson) -> Self {
        Self {
            workload: json.workload.clone(),
            service: json.service.clone(),
            config: json.config.clone(),
            secret: json.secret.clone(),
            field: json.field.clone(),
        }
    }
}

/// Environment variable transport options.
#[derive(Debug, Clone, Default)]
pub struct StackEnvVarOptions {
    /// Literal string value.
    pub value: Option<String>,
    /// Binding name to expose as an environment variable.
    pub binding: Option<String>,
}

impl StackEnvVarOptions {
    /// Inherit unset environment variable settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.value.is_none() {
            self.value = parent.value.clone();
        }
        if self.binding.is_none() {
            self.binding = parent.binding.clone();
        }
    }
}

impl From<&StackEnvVarJson> for StackEnvVarOptions {
    fn from(json: &StackEnvVarJson) -> Self {
        Self {
            value: json.value.clone(),
            binding: json.binding.clone(),
        }
    }
}

/// Mount options.
#[derive(Debug, Clone, Default)]
pub struct StackMountOptions {
    /// Mounted volume name.
    pub volume: Option<String>,
    /// Mounted config name.
    pub config: Option<String>,
    /// Mounted secret name.
    pub secret: Option<String>,
    /// Mount path.
    pub path: Option<String>,
    /// Whether the mount is read only.
    pub read_only: Option<bool>,
}

impl StackMountOptions {
    /// Inherit unset mount settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.volume.is_none() {
            self.volume = parent.volume.clone();
        }
        if self.config.is_none() {
            self.config = parent.config.clone();
        }
        if self.secret.is_none() {
            self.secret = parent.secret.clone();
        }
        if self.path.is_none() {
            self.path = parent.path.clone();
        }
        if self.read_only.is_none() {
            self.read_only = parent.read_only;
        }
    }
}

impl From<&StackMountJson> for StackMountOptions {
    fn from(json: &StackMountJson) -> Self {
        Self {
            volume: json.volume.clone(),
            config: json.config.clone(),
            secret: json.secret.clone(),
            path: json.path.clone(),
            read_only: json.read_only,
        }
    }
}

/// Binding configuration JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackBindingJson {
    /// Bound workload name.
    pub workload: Option<String>,
    /// Bound service name.
    pub service: Option<String>,
    /// Bound config name.
    pub config: Option<String>,
    /// Bound secret name.
    pub secret: Option<String>,
    /// Requested output field.
    pub field: Option<String>,
}

/// Environment variable transport JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackEnvVarJson {
    /// Literal string value.
    pub value: Option<String>,
    /// Binding name to expose as an environment variable.
    pub binding: Option<String>,
}

/// Mount JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackMountJson {
    /// Mounted volume name.
    pub volume: Option<String>,
    /// Mounted config name.
    pub config: Option<String>,
    /// Mounted secret name.
    pub secret: Option<String>,
    /// Mount path.
    pub path: Option<String>,
    /// Whether the mount is read only.
    pub read_only: Option<bool>,
}
