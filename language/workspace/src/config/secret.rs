use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::source::{SourceLocation, SourceOptions, SourceSelectorOptions};
use crate::config::stacks::{StackProviderJson, StackProviderOptions, merge_metadata};

/// Secret source format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecretSourceFormat {
    /// Infer one format from source metadata.
    #[default]
    Auto,
    /// Read one plain text payload.
    Text,
    /// Parse dotenv key value pairs.
    Env,
    /// Read opaque binary bytes.
    Binary,
}

impl From<SecretSourceFormatJson> for SecretSourceFormat {
    fn from(json: SecretSourceFormatJson) -> Self {
        match json {
            SecretSourceFormatJson::Auto => Self::Auto,
            SecretSourceFormatJson::Text => Self::Text,
            SecretSourceFormatJson::Env => Self::Env,
            SecretSourceFormatJson::Binary => Self::Binary,
        }
    }
}

/// Secret source selector options.
pub type SecretSourceSelectorOptions = SourceSelectorOptions;

impl From<&SecretSourceSelectorJson> for SourceSelectorOptions {
    fn from(json: &SecretSourceSelectorJson) -> Self {
        Self {
            key: json.key.clone(),
            prefix: None,
        }
    }
}

/// Secret source options.
pub type SecretSourceOptions = SourceOptions<SecretSourceFormat>;

impl From<&SecretSourceJson> for SecretSourceOptions {
    fn from(json: &SecretSourceJson) -> Self {
        match json {
            SecretSourceJson::File {
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
                format: format.map(SecretSourceFormat::from).unwrap_or_default(),
                optional: *optional,
                with: with.clone(),
            },
            SecretSourceJson::ProcessEnvironment {
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
                format: format.map(SecretSourceFormat::from).unwrap_or_default(),
                optional: *optional,
                with: with.clone(),
            },
            SecretSourceJson::Remote {
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
                format: format.map(SecretSourceFormat::from).unwrap_or_default(),
                optional: *optional,
                with: with.clone(),
            },
        }
    }
}

/// Secret options.
#[derive(Debug, Clone, Default)]
pub struct SecretOptions {
    /// Source declaration for external, file backed, or ambient secrets.
    pub source: Option<SecretSourceOptions>,
    /// Selection labels.
    pub labels: IndexMap<String, String>,
    /// Non-identifying metadata.
    pub annotations: IndexMap<String, String>,
    /// Provider attachment.
    pub provider: StackProviderOptions,
    /// Extra secret arguments.
    pub with: Option<Value>,
}

impl SecretOptions {
    /// Inherit unset secret settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if let Some(parent_source) = &parent.source {
            if let Some(source) = self.source.as_mut() {
                SecretSourceOptions::extend_from(source, parent_source);
            } else {
                self.source = Some((*parent_source).clone());
            }
        }

        self.provider.extend_from(&parent.provider);

        if self.with.is_none() {
            self.with = parent.with.clone();
        }

        merge_metadata(&mut self.labels, &parent.labels);
        merge_metadata(&mut self.annotations, &parent.annotations);
    }
}

impl From<&SecretJson> for SecretOptions {
    fn from(json: &SecretJson) -> Self {
        Self {
            source: json.source.as_ref().map(SecretSourceOptions::from),
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

/// Convert secret declarations into normalized options.
pub fn secret_options_from_json(
    json: &Option<IndexMap<String, SecretJson>>,
) -> IndexMap<String, SecretOptions> {
    json.as_ref()
        .map(|secrets| {
            secrets
                .iter()
                .map(|(name, secret)| (name.clone(), SecretOptions::from(secret)))
                .collect()
        })
        .unwrap_or_default()
}

/// Inherit one secret map from a parent config.
pub fn extend_secret_options(
    current: &mut IndexMap<String, SecretOptions>,
    parent: &IndexMap<String, SecretOptions>,
) {
    for (name, secret) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(secret);
        } else {
            current.insert(name.clone(), secret.clone());
        }
    }
}

/// Secret source format JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum SecretSourceFormatJson {
    /// Infer one format from source metadata.
    Auto,
    /// Read one plain text payload.
    Text,
    /// Parse dotenv key value pairs.
    Env,
    /// Read opaque binary bytes.
    Binary,
}

/// Secret source selector JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SecretSourceSelectorJson {
    /// Selected key within the source.
    pub key: Option<String>,
}

/// Secret source JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "location", rename_all = "camelCase")]
pub enum SecretSourceJson {
    /// Load one secret from one local file.
    File {
        /// File system path for the source.
        path: String,
        /// Selected projection within the source.
        selector: Option<SecretSourceSelectorJson>,
        /// Declared source format.
        format: Option<SecretSourceFormatJson>,
        /// Whether missing source material is allowed.
        #[serde(default)]
        optional: bool,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read one secret from ambient process environment.
    ProcessEnvironment {
        /// Selected projection within the source.
        selector: Option<SecretSourceSelectorJson>,
        /// Declared source format.
        format: Option<SecretSourceFormatJson>,
        /// Whether missing source material is allowed.
        #[serde(default)]
        optional: bool,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Load one secret from one external store reference.
    Remote {
        /// Remote source reference.
        reference: String,
        /// Selected projection within the source.
        selector: Option<SecretSourceSelectorJson>,
        /// Declared source format.
        format: Option<SecretSourceFormatJson>,
        /// Whether missing source material is allowed.
        #[serde(default)]
        optional: bool,
        /// Extra source arguments.
        with: Option<Value>,
    },
}

/// A passive secret reference node.
///
/// Inputs: one external source declaration.
/// Outputs: injectible secret bindings.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SecretJson {
    /// Source declaration for external, file backed, or ambient secrets.
    pub source: Option<SecretSourceJson>,
    /// Selection labels.
    pub labels: Option<IndexMap<String, String>>,
    /// Non-identifying metadata.
    pub annotations: Option<IndexMap<String, String>>,
    /// Provider attachment.
    pub provider: Option<StackProviderJson>,
    /// Extra secret arguments.
    pub with: Option<Value>,
}
