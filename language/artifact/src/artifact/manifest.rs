use serde::{Deserialize, Serialize};

/// One public build manifest for one linked target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildManifest {
    /// The primary entry path when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    /// The emitted file records for this target.
    pub files: Vec<BuildManifestFile>,
}

/// One public build manifest file record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildManifestFile {
    /// The emitted output path, relative to the output root when possible.
    pub path: String,
    /// The public file kind.
    pub r#type: BuildManifestFileType,
    /// The emitted file loader.
    pub loader: BuildManifestLoader,
    /// The logical chunk name when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The source input that produced this file when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// Whether this file is one entry output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_entry: Option<bool>,
    /// Whether this file is one dynamic entry output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_dynamic_entry: Option<bool>,
    /// Imported chunks or external specifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<String>,
    /// Dynamically imported chunks or external specifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dynamic_imports: Vec<String>,
    /// Associated emitted stylesheets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stylesheets: Vec<String>,
}

/// One public build manifest file kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildManifestFileType {
    /// One emitted script chunk.
    Chunk,
    /// One emitted asset sidecar.
    Asset,
    /// One emitted binary file.
    Binary,
}

/// One public build manifest loader name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildManifestLoader {
    /// JavaScript output.
    Js,
    /// CSS output.
    Css,
    /// TypeScript output.
    Ts,
    /// Source map output.
    #[serde(rename = "map")]
    Map,
    /// JSON output.
    Json,
    /// TypeScript declaration output.
    #[serde(rename = "dts")]
    Dts,
    /// WebAssembly output.
    Wasm,
    /// Native object output.
    #[serde(rename = "object")]
    Object,
    /// Generic emitted asset output.
    Asset,
}
