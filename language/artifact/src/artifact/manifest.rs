use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// One public build manifest for one linked target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BuildManifest {
    /// The primary entry path when one exists.
    pub index: Option<String>,
    /// The emitted file records for this target.
    pub files: Vec<BuildManifestFile>,
}

/// One public build manifest file record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BuildManifestFile {
    /// The emitted output path, relative to the output root when possible.
    pub path: String,
    /// The public file kind.
    pub r#type: BuildManifestFileType,
    /// The emitted file loader.
    pub loader: BuildManifestLoader,
    /// The logical chunk name when one exists.
    pub name: Option<String>,
    /// The source input that produced this file when one exists.
    pub input: Option<String>,
    /// Whether this file is one entry output.
    pub is_entry: Option<bool>,
    /// Whether this file is one dynamic entry output.
    pub is_dynamic_entry: Option<bool>,
    /// Imported chunks or external specifiers.
    pub imports: Vec<String>,
    /// Dynamically imported chunks or external specifiers.
    pub dynamic_imports: Vec<String>,
    /// Associated emitted stylesheets.
    pub stylesheets: Vec<String>,
}

/// One public build manifest file kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum BuildManifestFileType {
    /// One emitted JS chunk.
    Chunk,
    /// One emitted asset sidecar.
    Asset,
    /// One emitted binary file.
    Binary,
}

/// One public build manifest loader name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum BuildManifestLoader {
    /// JavaScript output.
    Js,
    /// CSS output.
    Css,
    /// Source map output.
    Map,
    /// JSON output.
    Json,
    /// WebAssembly output.
    Wasm,
    /// Native object output.
    Object,
    /// Generic emitted asset output.
    Asset,
}

/// JSON build manifest document.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildManifestDocument<'manifest> {
    /// The primary entry path when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<&'manifest str>,
    /// The emitted file records for this target.
    files: Vec<BuildManifestFileDocument<'manifest>>,
}

/// JSON build manifest file record.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildManifestFileDocument<'file> {
    /// The emitted output path.
    path: &'file str,
    /// The public file kind.
    r#type: &'static str,
    /// The emitted file loader.
    loader: &'static str,
    /// The logical chunk name when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'file str>,
    /// The source input that produced this file when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    input: Option<&'file str>,
    /// Whether this file is one entry output.
    #[serde(skip_serializing_if = "Option::is_none")]
    is_entry: Option<bool>,
    /// Whether this file is one dynamic entry output.
    #[serde(skip_serializing_if = "Option::is_none")]
    is_dynamic_entry: Option<bool>,
    /// Imported chunks or external specifiers.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    imports: &'file Vec<String>,
    /// Dynamically imported chunks or external specifiers.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dynamic_imports: &'file Vec<String>,
    /// Associated emitted stylesheets.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stylesheets: &'file Vec<String>,
}

impl BuildManifest {
    /// Serialize this build manifest to its public JSON document.
    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(BuildManifestDocument::from(self))
    }
}

impl BuildManifestFileType {
    /// Return the public manifest file type name.
    const fn name(self) -> &'static str {
        match self {
            Self::Chunk => "chunk",
            Self::Asset => "asset",
            Self::Binary => "binary",
        }
    }
}

impl BuildManifestLoader {
    /// Return the public manifest loader name.
    const fn name(self) -> &'static str {
        match self {
            Self::Js => "js",
            Self::Css => "css",
            Self::Map => "map",
            Self::Json => "json",
            Self::Wasm => "wasm",
            Self::Object => "object",
            Self::Asset => "asset",
        }
    }
}

impl<'manifest> From<&'manifest BuildManifest> for BuildManifestDocument<'manifest> {
    /// Build one JSON document from a build manifest.
    fn from(manifest: &'manifest BuildManifest) -> Self {
        let files = manifest
            .files
            .iter()
            .map(BuildManifestFileDocument::from)
            .collect();

        Self {
            index: manifest.index.as_deref(),
            files,
        }
    }
}

impl<'file> From<&'file BuildManifestFile> for BuildManifestFileDocument<'file> {
    /// Build one JSON document from a build manifest file.
    fn from(file: &'file BuildManifestFile) -> Self {
        Self {
            path: &file.path,
            r#type: file.r#type.name(),
            loader: file.loader.name(),
            name: file.name.as_deref(),
            input: file.input.as_deref(),
            is_entry: file.is_entry,
            is_dynamic_entry: file.is_dynamic_entry,
            imports: &file.imports,
            dynamic_imports: &file.dynamic_imports,
            stylesheets: &file.stylesheets,
        }
    }
}
