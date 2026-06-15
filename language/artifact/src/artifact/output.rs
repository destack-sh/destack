use destack_source::{ContentId, FileType, Uri};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{EmitFormat, Host, Platform, Runtime};

/// Builtin target output name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetOutputName {
    /// Per-module library output.
    Module,
    /// Primary runnable entry output.
    Entry,
    /// Declaration or type surface.
    Types,
    /// Asset collection emitted by this target.
    Assets,
    /// Build manifest or output index.
    Manifest,
    /// Source maps or debug maps.
    Maps,
    /// Native object or wasm payload.
    #[serde(alias = "binary")]
    Native,
}

impl TargetOutputName {
    /// Return the stable output name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Entry => "entry",
            Self::Types => "types",
            Self::Assets => "assets",
            Self::Manifest => "manifest",
            Self::Maps => "maps",
            Self::Native => "native",
        }
    }
}

/// The assembly mode for one package output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PackageAssembly {
    /// Per-module outputs without target-level assembly.
    #[default]
    PreserveModules,
    /// One assembled output file.
    SingleFile,
    /// Multiple assembled output files.
    Chunked,
}

impl PackageAssembly {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    /// The output URI.
    pub uri: Uri,
    /// The emitted file type.
    pub file_type: FileType,
    /// The output content identity.
    pub content: ContentId,
    /// The related source URI when one exists.
    pub source: Option<Uri>,
}

impl OutputFile {
    /// Create one derived output file.
    pub fn new(uri: Uri, file_type: FileType, content: ContentId, source: Option<Uri>) -> Self {
        Self {
            uri,
            file_type,
            content,
            source,
        }
    }
}

/// One package target output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageOutput {
    /// The emitted artifact family.
    pub emit: EmitFormat,
    /// The target-level assembly mode.
    pub assembly: PackageAssembly,
    /// The named output groups.
    pub outputs: IndexMap<TargetOutputName, Vec<OutputFile>>,
}

impl PackageOutput {
    /// Create one package output.
    pub fn new(
        emit: EmitFormat,
        assembly: PackageAssembly,
        outputs: IndexMap<TargetOutputName, Vec<OutputFile>>,
    ) -> Self {
        Self {
            emit,
            assembly,
            outputs,
        }
    }

    /// Return an iterator over all output files.
    pub fn files(&self) -> impl Iterator<Item = &OutputFile> {
        self.outputs.values().flatten()
    }

    /// Return all content ids referenced by this package output.
    pub fn content_ids(&self) -> Vec<ContentId> {
        self.files().map(|file| file.content).collect()
    }
}

/// One linked product output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductOutput {
    /// The product image manifest.
    pub manifest: ProductManifest,
    /// The files in this product image.
    pub files: Vec<OutputFile>,
}

impl ProductOutput {
    /// Create one product output.
    pub fn new(manifest: ProductManifest, files: Vec<OutputFile>) -> Self {
        Self { manifest, files }
    }

    /// Return all content ids referenced by this product output.
    pub fn content_ids(&self) -> Vec<ContentId> {
        self.files.iter().map(|file| file.content).collect()
    }
}

/// Manifest for one product image.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductManifest {
    /// The configured product name.
    pub product: String,
    /// The product units keyed by product target role.
    pub units: IndexMap<String, ProductUnit>,
    /// The launchable product entries.
    pub entries: Vec<ProductEntry>,
}

impl ProductManifest {
    /// Create one product manifest.
    pub fn new(
        product: String,
        units: IndexMap<String, ProductUnit>,
        entries: Vec<ProductEntry>,
    ) -> Self {
        Self {
            product,
            units,
            entries,
        }
    }
}

/// One unit assembled into a product image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUnit {
    /// The configured target name assembled into this unit.
    pub target: String,
    /// The runtime contract this unit expects.
    pub runtime: Runtime,
    /// The host environment this unit expects.
    pub host: Host,
    /// The platform this unit expects.
    pub platform: Platform,
    /// The unit output files grouped by target output name.
    pub outputs: IndexMap<TargetOutputName, Vec<Uri>>,
}

impl ProductUnit {
    /// Create one product unit.
    pub fn new(
        target: String,
        runtime: Runtime,
        host: Host,
        platform: Platform,
        outputs: IndexMap<TargetOutputName, Vec<Uri>>,
    ) -> Self {
        Self {
            target,
            runtime,
            host,
            platform,
            outputs,
        }
    }
}

/// One launchable entry in a product image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductEntry {
    /// The product unit that owns the entry.
    pub unit: String,
    /// The entry output URI.
    pub uri: Uri,
}

impl ProductEntry {
    /// Create one product entry.
    pub fn new(unit: String, uri: Uri) -> Self {
        Self { unit, uri }
    }
}
