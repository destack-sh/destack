use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// The stable source map version.
pub const SOURCE_MAP_VERSION: u32 = 3;

/// One emitted or linked source map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SourceMap {
    /// The source map version.
    pub version: u32,
    /// The emitted file name when one exists.
    pub file: Option<String>,
    /// The source root when one exists.
    pub source_root: Option<String>,
    /// The mapped source names.
    pub sources: Vec<String>,
    /// The embedded source contents when they exist.
    pub sources_content: Option<Vec<Option<String>>>,
    /// The recorded symbol names.
    pub names: Vec<String>,
    /// The VLQ mapping payload.
    pub mappings: String,
    /// The debug id when one exists.
    pub debug_id: Option<String>,
}

/// JSON source map document.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceMapDocument<'source> {
    /// The source map version.
    version: u32,
    /// The emitted file name when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<&'source str>,
    /// The source root when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    source_root: Option<&'source str>,
    /// The mapped source names.
    sources: &'source [String],
    /// The embedded source contents when they exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    sources_content: Option<&'source [Option<String>]>,
    /// The recorded symbol names.
    names: &'source [String],
    /// The VLQ mapping payload.
    mappings: &'source str,
    /// The debug id when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_id: Option<&'source str>,
}

impl SourceMap {
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
        serde_json::to_value(SourceMapDocument::from(self))
    }
}

impl<'source> From<&'source SourceMap> for SourceMapDocument<'source> {
    /// Build one JSON document from a source map.
    fn from(source_map: &'source SourceMap) -> Self {
        Self {
            version: source_map.version,
            file: source_map.file.as_deref(),
            source_root: source_map.source_root.as_deref(),
            sources: &source_map.sources,
            sources_content: source_map.sources_content.as_deref(),
            names: &source_map.names,
            mappings: &source_map.mappings,
            debug_id: source_map.debug_id.as_deref(),
        }
    }
}
