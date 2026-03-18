use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use super::common::{
    StackCacheJson, StackCacheOptions, StackProviderJson, StackProviderOptions, merge_metadata,
};

/// Publication options.
#[derive(Debug, Clone, Default)]
pub struct StackPublicationOptions {
    /// Source asset collection name.
    pub asset: Option<String>,
    /// Optional named output within the source asset collection.
    pub output: Option<String>,
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
        if self.asset.is_none() {
            self.asset = parent.asset.clone();
        }
        if self.output.is_none() {
            self.output = parent.output.clone();
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
            asset: json.asset.clone(),
            output: json.output.clone(),
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
    /// Source asset collection name.
    pub asset: Option<String>,
    /// Optional named output within the source asset collection.
    pub output: Option<String>,
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
