use serde::Deserialize;

use super::super::{StackAttachmentSourceJson, StackAttachmentSourceOptions};

/// Binding configuration options.
#[derive(Debug, Clone, Default)]
pub struct StackBindingOptions {
    /// Bound source.
    pub source: Option<StackAttachmentSourceOptions>,
}

impl StackBindingOptions {
    /// Inherit unset binding settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if let Some(parent_source) = &parent.source {
            if let Some(source) = self.source.as_mut() {
                source.extend_from(parent_source);
            } else {
                self.source = Some(parent_source.clone());
            }
        }
    }
}

impl From<&StackBindingJson> for StackBindingOptions {
    fn from(json: &StackBindingJson) -> Self {
        Self {
            source: json.source.as_ref().map(StackAttachmentSourceOptions::from),
        }
    }
}

/// Environment variable transport options.
#[derive(Debug, Clone, Default)]
pub struct StackEnvVarOptions {
    /// Literal string value.
    pub value: Option<String>,
    /// Bound source exposed as an environment variable.
    pub source: Option<StackAttachmentSourceOptions>,
}

impl StackEnvVarOptions {
    /// Inherit unset environment variable settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.value.is_none() {
            self.value = parent.value.clone();
        }

        if let Some(parent_source) = &parent.source {
            if let Some(source) = self.source.as_mut() {
                source.extend_from(parent_source);
            } else {
                self.source = Some(parent_source.clone());
            }
        }
    }
}

impl From<&StackEnvVarJson> for StackEnvVarOptions {
    fn from(json: &StackEnvVarJson) -> Self {
        Self {
            value: json.value.clone(),
            source: json.source.as_ref().map(StackAttachmentSourceOptions::from),
        }
    }
}

/// Mount options.
#[derive(Debug, Clone, Default)]
pub struct StackMountOptions {
    /// Mounted source.
    pub source: Option<StackAttachmentSourceOptions>,
    /// Mount path.
    pub path: Option<String>,
    /// Whether the mount is read only.
    pub read_only: Option<bool>,
}

impl StackMountOptions {
    /// Inherit unset mount settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if let Some(parent_source) = &parent.source {
            if let Some(source) = self.source.as_mut() {
                source.extend_from(parent_source);
            } else {
                self.source = Some(parent_source.clone());
            }
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
            source: json.source.as_ref().map(StackAttachmentSourceOptions::from),
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
    /// Bound source.
    pub source: Option<StackAttachmentSourceJson>,
}

/// Environment variable transport JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackEnvVarJson {
    /// Literal string value.
    pub value: Option<String>,
    /// Bound source exposed as an environment variable.
    pub source: Option<StackAttachmentSourceJson>,
}

/// Mount JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackMountJson {
    /// Mounted source.
    pub source: Option<StackAttachmentSourceJson>,
    /// Mount path.
    pub path: Option<String>,
    /// Whether the mount is read only.
    pub read_only: Option<bool>,
}
