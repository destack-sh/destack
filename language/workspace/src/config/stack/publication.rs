use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::{
    StackCacheJson, StackCacheOptions, StackProviderJson, StackProviderOptions, merge_metadata,
};
use crate::config::target::TargetOutputName;

/// Publication options.
#[derive(Debug, Clone, Default)]
pub struct StackPublicationOptions {
    /// Source publication input.
    pub source: Option<StackPublicationSourceOptions>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Backing origin configuration.
    pub origin: StackPublicationOriginOptions,
    /// Cache policy for the published content.
    pub cache: StackCacheOptions,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Extra publication arguments.
    pub with: Option<Value>,
}

impl StackPublicationOptions {
    /// Inherit unset publication settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if let Some(parent_source) = &parent.source {
            if let Some(source) = self.source.as_mut() {
                source.extend_from(parent_source);
            } else {
                self.source = Some(parent_source.clone());
            }
        }
        self.provider.extend_from(&parent.provider);
        if self.with.is_none() {
            self.with = parent.with.clone();
        }

        self.origin.extend_from(&parent.origin);
        self.cache.extend_from(&parent.cache);
        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&StackPublicationJson> for StackPublicationOptions {
    fn from(json: &StackPublicationJson) -> Self {
        Self {
            source: json
                .source
                .as_ref()
                .map(StackPublicationSourceOptions::from),
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            origin: StackPublicationOriginOptions::from(&json.origin),
            cache: StackCacheOptions::from(&json.cache),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Publication source options.
#[derive(Debug, Clone)]
pub enum StackPublicationSourceOptions {
    /// Publish one named asset collection.
    Asset {
        /// Asset collection name.
        asset: String,
        /// Optional named output within the source asset collection.
        output: Option<String>,
    },
    /// Publish one named target output directly.
    Target {
        /// Target name.
        target: String,
        /// Optional named output within the source target.
        output: Option<String>,
    },
}

impl StackPublicationSourceOptions {
    /// Inherit unset source settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        match (self, parent) {
            (
                Self::Asset { output, .. },
                Self::Asset {
                    output: parent_output,
                    ..
                },
            ) => {
                if output.is_none() {
                    *output = parent_output.clone();
                }
            }
            (
                Self::Target { output, .. },
                Self::Target {
                    output: parent_output,
                    ..
                },
            ) => {
                if output.is_none() {
                    *output = parent_output.clone();
                }
            }
            _ => {}
        }
    }
}

impl From<&StackPublicationSourceJson> for StackPublicationSourceOptions {
    fn from(json: &StackPublicationSourceJson) -> Self {
        match json {
            StackPublicationSourceJson::Asset { asset, output } => Self::Asset {
                asset: asset.clone(),
                output: output.clone(),
            },
            StackPublicationSourceJson::Target { target, output } => Self::Target {
                target: target.clone(),
                output: output.map(|output| output.as_str().to_string()),
            },
        }
    }
}

/// Publication origin options.
#[derive(Debug, Clone, Default)]
pub struct StackPublicationOriginOptions {
    /// Internal origin service reference.
    pub service: Option<String>,
    /// External origin endpoint.
    pub endpoint: Option<String>,
}

impl StackPublicationOriginOptions {
    /// Inherit unset origin settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.service.is_none() {
            self.service = parent.service.clone();
        }
        if self.endpoint.is_none() {
            self.endpoint = parent.endpoint.clone();
        }
    }
}

impl From<&StackPublicationOriginJson> for StackPublicationOriginOptions {
    fn from(json: &StackPublicationOriginJson) -> Self {
        Self {
            service: json.service.clone(),
            endpoint: json.endpoint.clone(),
        }
    }
}

/// Publication JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackPublicationJson {
    /// Source publication input.
    pub source: Option<StackPublicationSourceJson>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Backing origin configuration.
    #[serde(default)]
    pub origin: StackPublicationOriginJson,
    /// Cache policy for the published content.
    #[serde(default)]
    pub cache: StackCacheJson,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Extra publication arguments.
    pub with: Option<Value>,
}

/// Publication source JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StackPublicationSourceJson {
    /// Publish one named asset collection.
    Asset {
        /// Asset collection name.
        asset: String,
        /// Optional named output within the source asset collection.
        output: Option<String>,
    },
    /// Publish one named target output directly.
    Target {
        /// Target name.
        target: String,
        /// Optional named output within the source target.
        output: Option<TargetOutputName>,
    },
}

/// Publication origin JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackPublicationOriginJson {
    /// Internal origin service reference.
    pub service: Option<String>,
    /// External origin endpoint.
    pub endpoint: Option<String>,
}
