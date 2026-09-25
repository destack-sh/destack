use serde::{Deserialize, Serialize};
use tspp_core::Blob;
use tspp_serde::Reflect;
use tspp_source::{FileType, Uri};

/// One section of a linked bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum BundleSection {
    /// Per-module library output.
    Module,
    /// Primary runnable entry output.
    Entry,
    /// Asset collection emitted by this target.
    Asset,
    /// Build manifest or output index.
    Manifest,
    /// Source maps or debug maps.
    SourceMap,
    /// Native object or wasm payload.
    Native,
}

impl BundleSection {
    /// Return the stable section name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Entry => "entry",
            Self::Asset => "asset",
            Self::Manifest => "manifest",
            Self::SourceMap => "sourceMap",
            Self::Native => "native",
        }
    }
}

/// The assembly mode for one bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
pub enum BundleMode {
    /// Per-module assets without target-level assembly.
    #[default]
    PreserveModules,
    /// One assembled output file.
    SingleFile,
    /// Multiple assembled output files.
    Chunked,
}

impl BundleMode {
    /// Return the stable assembly tag.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreserveModules => "preserveModules",
            Self::SingleFile => "singleFile",
            Self::Chunked => "chunked",
        }
    }
}

/// One derived output file.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct BundleFile {
    /// The bundle section this file belongs to.
    pub section: BundleSection,
    /// The output URI.
    pub uri: Uri,
    /// The emitted file type.
    pub file_type: FileType,
    /// The exact output bytes.
    pub blob: Blob,
    /// The related source URI when one exists.
    pub source: Option<Uri>,
}

impl BundleFile {
    /// Create one derived output file.
    pub fn new(
        section: BundleSection,
        uri: Uri,
        file_type: FileType,
        blob: Blob,
        source: Option<Uri>,
    ) -> Self {
        Self {
            section,
            uri,
            file_type,
            blob,
            source,
        }
    }
}

/// One linked file graph for one target.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct Bundle {
    /// The target-level assembly mode.
    pub assembly: BundleMode,
    /// The files in this bundle.
    pub files: Vec<BundleFile>,
}

impl Bundle {
    /// Create one bundle.
    pub fn new(assembly: BundleMode, files: Vec<BundleFile>) -> Self {
        Self { assembly, files }
    }

    /// Return an iterator over all output files.
    pub fn files(&self) -> impl Iterator<Item = &BundleFile> {
        self.files.iter()
    }

    /// Return every Blob referenced by this bundle.
    pub fn blobs(&self) -> Vec<Blob> {
        self.files().map(|file| file.blob).collect()
    }
}
