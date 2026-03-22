use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::source::{SourceLocation, SourceOptions, SourceSelectorOptions};
use crate::config::stacks::{StackProviderJson, StackProviderOptions, merge_metadata};

/// Structured config source format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigSourceFormat {
    /// Infer one format from source metadata.
    #[default]
    Auto,
    /// Parse JSON.
    Json,
    /// Parse YAML.
    Yaml,
    /// Parse TOML.
    Toml,
    /// Parse dotenv key value pairs.
    Env,
    /// Read one plain text payload.
    Text,
}

impl From<ConfigSourceFormatJson> for ConfigSourceFormat {
    fn from(json: ConfigSourceFormatJson) -> Self {
        match json {
            ConfigSourceFormatJson::Auto => Self::Auto,
            ConfigSourceFormatJson::Json => Self::Json,
            ConfigSourceFormatJson::Yaml => Self::Yaml,
            ConfigSourceFormatJson::Toml => Self::Toml,
            ConfigSourceFormatJson::Env => Self::Env,
            ConfigSourceFormatJson::Text => Self::Text,
        }
    }
}

/// Config source selector options.
pub type ConfigSourceSelectorOptions = SourceSelectorOptions;

impl From<&ConfigSourceSelectorJson> for SourceSelectorOptions {
    fn from(json: &ConfigSourceSelectorJson) -> Self {
        Self {
            key: json.key.clone(),
            prefix: json.prefix.clone(),
        }
    }
}

/// Config source options.
pub type ConfigSourceOptions = SourceOptions<ConfigSourceFormat>;

impl From<&ConfigSourceJson> for ConfigSourceOptions {
    fn from(json: &ConfigSourceJson) -> Self {
        match json {
            ConfigSourceJson::File {
                path,
                selector,
                format,
                optional,
                with,
            } => Self {
                location: SourceLocation::File,
                locator: Some(path.clone()),
                selector: selector
                    .as_ref()
                    .map(SourceSelectorOptions::from)
                    .unwrap_or_default(),
                format: format.map(ConfigSourceFormat::from).unwrap_or_default(),
                optional: *optional,
                with: with.clone(),
            },
            ConfigSourceJson::ProcessEnvironment {
                selector,
                format,
                optional,
                with,
            } => Self {
                location: SourceLocation::ProcessEnvironment,
                locator: None,
                selector: selector
                    .as_ref()
                    .map(SourceSelectorOptions::from)
                    .unwrap_or_default(),
                format: format.map(ConfigSourceFormat::from).unwrap_or_default(),
                optional: *optional,
                with: with.clone(),
            },
            ConfigSourceJson::Remote {
                reference,
                selector,
                format,
                optional,
                with,
            } => Self {
                location: SourceLocation::Remote,
                locator: Some(reference.clone()),
                selector: selector
                    .as_ref()
                    .map(SourceSelectorOptions::from)
                    .unwrap_or_default(),
                format: format.map(ConfigSourceFormat::from).unwrap_or_default(),
                optional: *optional,
                with: with.clone(),
            },
        }
    }
}

/// Config options.
#[derive(Debug, Clone, Default)]
pub struct ConfigOptions {
    /// Source declaration for external or file backed config.
    pub source: Option<ConfigSourceOptions>,
    /// Inline config data.
    pub data: IndexMap<String, Value>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Extra config arguments.
    pub with: Option<Value>,
}

impl ConfigOptions {
    /// Inherit unset config settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if let Some(parent_source) = &parent.source {
            if let Some(source) = self.source.as_mut() {
                ConfigSourceOptions::extend_from(source, parent_source);
            } else {
                self.source = Some((*parent_source).clone());
            }
        }

        self.provider.extend_from(&parent.provider);

        if self.with.is_none() {
            self.with = parent.with.clone();
        }

        if self.data.is_empty() {
            self.data = parent.data.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&ConfigJson> for ConfigOptions {
    fn from(json: &ConfigJson) -> Self {
        Self {
            source: json.source.as_ref().map(ConfigSourceOptions::from),
            data: json.data.clone().unwrap_or_default(),
            labels: json.labels.clone().unwrap_or_default(),
            annotations: json.annotations.clone().unwrap_or_default(),
            provider: json
                .provider
                .as_ref()
                .map(StackProviderOptions::from)
                .unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Convert config declarations into normalized options.
pub fn config_options_from_json(
    json: &Option<IndexMap<String, ConfigJson>>,
) -> IndexMap<String, ConfigOptions> {
    json.as_ref()
        .map(|configs| {
            configs
                .iter()
                .map(|(name, config)| (name.clone(), ConfigOptions::from(config)))
                .collect()
        })
        .unwrap_or_default()
}

/// Inherit one config map from a parent config.
pub fn extend_config_options(
    current: &mut IndexMap<String, ConfigOptions>,
    parent: &IndexMap<String, ConfigOptions>,
) {
    for (name, config) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(config);
        } else {
            current.insert(name.clone(), config.clone());
        }
    }
}

/// Config source format JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum ConfigSourceFormatJson {
    /// Infer one format from source metadata.
    Auto,
    /// Parse JSON.
    Json,
    /// Parse YAML.
    Yaml,
    /// Parse TOML.
    Toml,
    /// Parse dotenv key value pairs.
    Env,
    /// Read one plain text payload.
    Text,
}

/// Config source selector JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConfigSourceSelectorJson {
    /// Optional selected key within the source.
    pub key: Option<String>,
    /// Optional key prefix within the source.
    pub prefix: Option<String>,
}

/// Config source JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "location", rename_all = "camelCase")]
pub enum ConfigSourceJson {
    /// Load config from one local file.
    File {
        /// File system path for the source.
        path: String,
        /// Optional selected projection within the source.
        selector: Option<ConfigSourceSelectorJson>,
        /// Declared source format.
        format: Option<ConfigSourceFormatJson>,
        /// Whether missing source material is allowed.
        #[serde(default)]
        optional: bool,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read config from ambient process environment.
    ProcessEnvironment {
        /// Optional selected projection within the source.
        selector: Option<ConfigSourceSelectorJson>,
        /// Declared source format.
        format: Option<ConfigSourceFormatJson>,
        /// Whether missing source material is allowed.
        #[serde(default)]
        optional: bool,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Load config from one external store reference.
    Remote {
        /// Remote source reference.
        reference: String,
        /// Optional selected projection within the source.
        selector: Option<ConfigSourceSelectorJson>,
        /// Declared source format.
        format: Option<ConfigSourceFormatJson>,
        /// Whether missing source material is allowed.
        #[serde(default)]
        optional: bool,
        /// Extra source arguments.
        with: Option<Value>,
    },
}

/// A passive configuration node.
///
/// Inputs: inline data or one external source declaration.
/// Outputs: mountable or bindable configuration handles.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ConfigJson {
    /// Source declaration for external or file backed config.
    pub source: Option<ConfigSourceJson>,
    /// Inline config data.
    pub data: Option<IndexMap<String, Value>>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Extra config arguments.
    pub with: Option<Value>,
}
