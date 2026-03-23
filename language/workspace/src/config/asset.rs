use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

use crate::config::target::TargetOutputName;

/// Asset source location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssetSourceLocation {
    /// Read assets from one local path.
    #[default]
    Path,
    /// Read assets from one named target output.
    Target,
}

/// Asset source options.
#[derive(Debug, Clone, Default)]
pub struct AssetSourceOptions {
    /// Source location.
    pub location: AssetSourceLocation,
    /// File system path for `path` sources.
    pub path: Option<String>,
    /// Target name for `target` sources.
    pub target: Option<String>,
    /// Optional named output within the source target.
    pub output: Option<String>,
    /// Extra source arguments.
    pub with: Option<Value>,
}

impl AssetSourceOptions {
    /// Inherit unset source settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.path.is_none() {
            self.path = parent.path.clone();
        }

        if self.target.is_none() {
            self.target = parent.target.clone();
        }

        if self.output.is_none() {
            self.output = parent.output.clone();
        }

        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&AssetSourceJson> for AssetSourceOptions {
    fn from(json: &AssetSourceJson) -> Self {
        match json {
            AssetSourceJson::Path { path, with } => Self {
                location: AssetSourceLocation::Path,
                path: Some(path.clone()),
                target: None,
                output: None,
                with: with.clone(),
            },
            AssetSourceJson::Target {
                target,
                output,
                with,
            } => Self {
                location: AssetSourceLocation::Target,
                path: None,
                target: Some(target.clone()),
                output: output.map(|output| output.as_str().to_string()),
                with: with.clone(),
            },
        }
    }
}

/// Asset collection options.
#[derive(Debug, Clone, Default)]
pub struct AssetOptions {
    /// Local or generated asset source.
    pub source: Option<AssetSourceOptions>,
    /// Default build or cooking profile.
    pub profile: Option<String>,
    /// Named derived outputs.
    pub outputs: IndexMap<String, AssetOutputOptions>,
    /// Extra asset arguments.
    pub with: Option<Value>,
}

impl AssetOptions {
    /// Inherit unset asset settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if let Some(parent_source) = &parent.source {
            if let Some(source) = self.source.as_mut() {
                source.extend_from(parent_source);
            } else {
                self.source = Some(parent_source.clone());
            }
        }

        if self.profile.is_none() {
            self.profile = parent.profile.clone();
        }

        if self.with.is_none() {
            self.with = parent.with.clone();
        }

        for (name, output) in &parent.outputs {
            if let Some(current) = self.outputs.get_mut(name) {
                current.extend_from(output);
            } else {
                self.outputs.insert(name.clone(), output.clone());
            }
        }
    }
}

impl From<&AssetJson> for AssetOptions {
    fn from(json: &AssetJson) -> Self {
        Self {
            source: json.source.as_ref().map(AssetSourceOptions::from),
            profile: json.profile.clone(),
            outputs: json
                .outputs
                .as_ref()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|(name, output)| (name.clone(), AssetOutputOptions::from(output)))
                        .collect()
                })
                .unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Asset output options.
#[derive(Debug, Clone, Default)]
pub struct AssetOutputOptions {
    /// Derived output kind.
    pub kind: Option<AssetOutputKind>,
    /// Source members included in this output.
    pub r#match: Vec<String>,
    /// Optional build or cooking profile override.
    pub profile: Option<String>,
    /// Extra output arguments.
    pub with: Option<Value>,
}

impl AssetOutputOptions {
    /// Inherit unset output settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.kind.is_none() {
            self.kind = parent.kind;
        }
        if self.r#match.is_empty() {
            self.r#match = parent.r#match.clone();
        }
        if self.profile.is_none() {
            self.profile = parent.profile.clone();
        }
        if self.with.is_none() {
            self.with = parent.with.clone();
        }
    }
}

impl From<&AssetOutputJson> for AssetOutputOptions {
    fn from(json: &AssetOutputJson) -> Self {
        Self {
            kind: json.kind.map(AssetOutputKind::from),
            r#match: json.r#match.clone().unwrap_or_default(),
            profile: json.profile.clone(),
            with: json.with.clone(),
        }
    }
}

/// Asset output kind options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetOutputKind {
    /// Plain published files.
    Files,
    /// Grouped runtime bundle or pack.
    Bundle,
    /// Catalog or lookup manifest.
    Catalog,
}

impl From<AssetOutputKindJson> for AssetOutputKind {
    fn from(json: AssetOutputKindJson) -> Self {
        match json {
            AssetOutputKindJson::Files => Self::Files,
            AssetOutputKindJson::Bundle => Self::Bundle,
            AssetOutputKindJson::Catalog => Self::Catalog,
        }
    }
}

/// Asset reference JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum AssetRefsJson {
    /// One named asset collection reference.
    One(String),
    /// Many named asset collection references.
    Many(Vec<String>),
}

impl AssetRefsJson {
    /// Return the referenced asset collection names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(name) => vec![name.clone()],
            Self::Many(names) => names.clone(),
        }
    }
}

/// Convert asset declarations into normalized options.
pub fn asset_options_from_json(
    json: &Option<IndexMap<String, AssetJson>>,
) -> IndexMap<String, AssetOptions> {
    json.as_ref()
        .map(|assets| {
            assets
                .iter()
                .map(|(name, asset)| (name.clone(), AssetOptions::from(asset)))
                .collect()
        })
        .unwrap_or_default()
}

/// Inherit one asset map from a parent config.
pub fn extend_asset_options(
    current: &mut IndexMap<String, AssetOptions>,
    parent: &IndexMap<String, AssetOptions>,
) {
    for (name, asset) in parent {
        if let Some(existing) = current.get_mut(name) {
            existing.extend_from(asset);
        } else {
            current.insert(name.clone(), asset.clone());
        }
    }
}

/// Asset source JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "location", rename_all = "camelCase")]
pub enum AssetSourceJson {
    /// Read assets from one local path.
    Path {
        /// File system path for the source.
        path: String,
        /// Extra source arguments.
        with: Option<Value>,
    },
    /// Read assets from one named target output.
    Target {
        /// Target name for the source.
        target: String,
        /// Optional named output within the source target.
        output: Option<TargetOutputName>,
        /// Extra source arguments.
        with: Option<Value>,
    },
}

/// Asset collection JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AssetJson {
    /// Local or generated asset source.
    pub source: Option<AssetSourceJson>,
    /// Default build or cooking profile.
    pub profile: Option<String>,
    /// Named derived outputs.
    pub outputs: Option<IndexMap<String, AssetOutputJson>>,
    /// Extra asset arguments.
    pub with: Option<Value>,
}

/// Asset output JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AssetOutputJson {
    /// Derived output kind.
    pub kind: Option<AssetOutputKindJson>,
    /// Source members included in this output.
    pub r#match: Option<Vec<String>>,
    /// Optional build or cooking profile override.
    pub profile: Option<String>,
    /// Extra output arguments.
    pub with: Option<Value>,
}

/// Asset output kind JSON.
#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum AssetOutputKindJson {
    /// Plain published files.
    Files,
    /// Grouped runtime bundle or pack.
    Bundle,
    /// Catalog or lookup manifest.
    Catalog,
}
