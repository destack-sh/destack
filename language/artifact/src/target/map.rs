use serde::{Deserialize, Serialize};

/// The stable source map version.
pub const SOURCE_MAP_VERSION: u32 = 3;

/// One generated or linked source map payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMapArtifact {
    /// The source map version.
    pub version: u32,
    /// The generated file name when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The source root when one exists.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "sourceRoot"
    )]
    pub source_root: Option<String>,
    /// The mapped source names.
    pub sources: Vec<String>,
    /// The embedded source contents when they exist.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "sourcesContent"
    )]
    pub sources_content: Option<Vec<Option<String>>>,
    /// The recorded symbol names.
    pub names: Vec<String>,
    /// The VLQ mapping payload.
    pub mappings: String,
    /// The debug id when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "debugId")]
    pub debug_id: Option<String>,
}

impl SourceMapArtifact {
    /// Create one empty source map for a single source.
    pub fn empty(source: String) -> Self {
        Self {
            version: SOURCE_MAP_VERSION,
            file: None,
            source_root: None,
            sources: vec![source],
            sources_content: None,
            names: Vec::new(),
            mappings: String::new(),
            debug_id: None,
        }
    }

    /// Serialize this source map to JSON.
    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}
