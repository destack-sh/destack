use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

/// Asset collection options.
#[derive(Debug, Clone, Default)]
pub struct StackAssetOptions {
    /// Local source directory or built asset root.
    pub source: Option<String>,
    /// Default build or cooking profile.
    pub profile: Option<String>,
    /// Named derived outputs.
    pub outputs: IndexMap<String, StackAssetOutputOptions>,
    /// Extra asset arguments.
    pub with: Option<Value>,
}

impl StackAssetOptions {
    /// Inherit unset asset settings from one parent config.
    pub fn extend_from(&mut self, parent: &Self) {
        if self.source.is_none() {
            self.source = parent.source.clone();
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

impl From<&StackAssetJson> for StackAssetOptions {
    fn from(json: &StackAssetJson) -> Self {
        Self {
            source: json.source.clone(),
            profile: json.profile.clone(),
            outputs: json
                .outputs
                .as_ref()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|(name, output)| (name.clone(), StackAssetOutputOptions::from(output)))
                        .collect()
                })
                .unwrap_or_default(),
            with: json.with.clone(),
        }
    }
}

/// Asset output options.
#[derive(Debug, Clone, Default)]
pub struct StackAssetOutputOptions {
    /// Derived output kind.
    pub kind: Option<StackAssetOutputKind>,
    /// Source members included in this output.
    pub r#match: Vec<String>,
    /// Optional build or cooking profile override.
    pub profile: Option<String>,
    /// Extra output arguments.
    pub with: Option<Value>,
}

impl StackAssetOutputOptions {
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

impl From<&StackAssetOutputJson> for StackAssetOutputOptions {
    fn from(json: &StackAssetOutputJson) -> Self {
        Self {
            kind: json.kind.map(StackAssetOutputKind::from),
            r#match: json.r#match.clone().unwrap_or_default(),
            profile: json.profile.clone(),
            with: json.with.clone(),
        }
    }
}

/// Asset output kind options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackAssetOutputKind {
    /// Plain published files.
    Files,
    /// Grouped runtime bundle or pack.
    Bundle,
    /// Catalog or lookup manifest.
    Catalog,
    /// Preview or thumbnail output.
    Preview,
    /// Streaming playback output.
    Stream,
}

impl From<StackAssetOutputKindJson> for StackAssetOutputKind {
    fn from(json: StackAssetOutputKindJson) -> Self {
        match json {
            StackAssetOutputKindJson::Files => Self::Files,
            StackAssetOutputKindJson::Bundle => Self::Bundle,
            StackAssetOutputKindJson::Catalog => Self::Catalog,
            StackAssetOutputKindJson::Preview => Self::Preview,
            StackAssetOutputKindJson::Stream => Self::Stream,
        }
    }
}

/// Asset reference JSON.
#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum StackAssetRefsJson {
    /// One named asset collection reference.
    One(String),
    /// Many named asset collection references.
    Many(Vec<String>),
}

impl StackAssetRefsJson {
    /// Return the referenced asset collection names.
    pub fn names(&self) -> Vec<String> {
        match self {
            Self::One(name) => vec![name.clone()],
            Self::Many(names) => names.clone(),
        }
    }
}

/// Asset collection JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackAssetJson {
    /// Local source directory or built asset root.
    pub source: Option<String>,
    /// Default build or cooking profile.
    pub profile: Option<String>,
    /// Named derived outputs.
    pub outputs: Option<IndexMap<String, StackAssetOutputJson>>,
    /// Extra asset arguments.
    pub with: Option<Value>,
}

/// Asset output JSON.
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StackAssetOutputJson {
    /// Derived output kind.
    pub kind: Option<StackAssetOutputKindJson>,
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
pub enum StackAssetOutputKindJson {
    /// Plain published files.
    Files,
    /// Grouped runtime bundle or pack.
    Bundle,
    /// Catalog or lookup manifest.
    Catalog,
    /// Preview or thumbnail output.
    Preview,
    /// Streaming playback output.
    Stream,
}
