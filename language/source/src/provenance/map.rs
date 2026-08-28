use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// The source map version.
pub const SOURCE_MAP_VERSION: u32 = 3;

/// One regular source map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SourceMap {
    /// The emitted file name when one exists.
    pub file: Option<String>,
    /// The source root when one exists.
    pub source_root: Option<String>,
    /// The original source URLs.
    pub sources: Vec<Option<String>>,
    /// The original source contents when embedded.
    pub sources_content: Option<Vec<Option<String>>>,
    /// The original symbol names.
    pub names: Vec<String>,
    /// The encoded mappings.
    pub mappings: String,
    /// The third-party source indices.
    pub ignore_list: Vec<u32>,
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
    /// The original source URLs.
    sources: &'source [Option<String>],
    /// The original source contents when embedded.
    #[serde(skip_serializing_if = "Option::is_none")]
    sources_content: Option<&'source [Option<String>]>,
    /// The original symbol names.
    names: &'source [String],
    /// The encoded mappings.
    mappings: &'source str,
    /// The third-party source indices.
    #[serde(skip_serializing_if = "Option::is_none")]
    ignore_list: Option<&'source [u32]>,
    /// The debug id when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    debug_id: Option<&'source str>,
}

impl SourceMap {
    /// Create one empty source map for a single source.
    pub fn empty(source: String) -> Self {
        Self {
            file: None,
            source_root: None,
            sources: vec![Some(source)],
            sources_content: None,
            names: Vec::new(),
            mappings: String::new(),
            ignore_list: Vec::new(),
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
            version: SOURCE_MAP_VERSION,
            file: source_map.file.as_deref(),
            source_root: source_map.source_root.as_deref(),
            sources: &source_map.sources,
            sources_content: source_map.sources_content.as_deref(),
            names: &source_map.names,
            mappings: &source_map.mappings,
            ignore_list: (!source_map.ignore_list.is_empty())
                .then_some(source_map.ignore_list.as_slice()),
            debug_id: source_map.debug_id.as_deref(),
        }
    }
}
