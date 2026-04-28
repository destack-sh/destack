use destack_source::{FileContent, FileType, Uri};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{EmitFormat, SourceMapArtifact};

/// Formal target output group kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetOutputKind {
    /// Per-module library output.
    #[default]
    Module,
    /// Primary runnable entry output.
    Entry,
    /// Primary document output.
    Document,
    /// Declaration or type surface.
    Types,
    /// Asset collection emitted by this target.
    Assets,
    /// Build manifest or output index.
    Manifest,
    /// Source maps or debug maps.
    Maps,
    /// Binary or wasm payload.
    Binary,
    /// Debug symbol output.
    Debug,
    /// Additional metadata output.
    Metadata,
}

/// Builtin target output name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum TargetOutputName {
    /// Per-module library output.
    Module,
    /// Primary runnable entry output.
    Entry,
    /// Primary document output.
    Document,
    /// Declaration or type surface.
    Types,
    /// Asset collection emitted by this target.
    Assets,
    /// Build manifest or output index.
    Manifest,
    /// Source maps or debug maps.
    Maps,
    /// Binary or wasm payload.
    Binary,
    /// Debug symbol output.
    Debug,
    /// Additional metadata output.
    Metadata,
}

impl TargetOutputName {
    /// Return the stable output name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Entry => "entry",
            Self::Document => "document",
            Self::Types => "types",
            Self::Assets => "assets",
            Self::Manifest => "manifest",
            Self::Maps => "maps",
            Self::Binary => "binary",
            Self::Debug => "debug",
            Self::Metadata => "metadata",
        }
    }

    /// Return the semantic kind for this builtin output.
    pub fn kind(&self) -> TargetOutputKind {
        match self {
            Self::Module => TargetOutputKind::Module,
            Self::Entry => TargetOutputKind::Entry,
            Self::Document => TargetOutputKind::Document,
            Self::Types => TargetOutputKind::Types,
            Self::Assets => TargetOutputKind::Assets,
            Self::Manifest => TargetOutputKind::Manifest,
            Self::Maps => TargetOutputKind::Maps,
            Self::Binary => TargetOutputKind::Binary,
            Self::Debug => TargetOutputKind::Debug,
            Self::Metadata => TargetOutputKind::Metadata,
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

/// One derived output file payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputContent {
    /// Text output.
    Text {
        /// The emitted text.
        code: String,
        /// The emitted file type.
        file_type: FileType,
    },
    /// JSON output.
    Json {
        /// The serialized JSON text.
        content: String,
        /// The parsed JSON value.
        value: serde_json::Value,
        /// The emitted file type.
        file_type: FileType,
    },
    /// Binary output.
    Binary {
        /// The emitted bytes.
        bytes: Vec<u8>,
        /// The emitted file type.
        file_type: FileType,
    },
}

impl OutputContent {
    /// Normalize one emitted text payload to the canonical file form.
    fn normalize_text_payload(mut text: String) -> String {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }

        text
    }

    /// Create JavaScript content.
    pub fn javascript(code: String) -> Self {
        Self::Text {
            code: Self::normalize_text_payload(code),
            file_type: FileType::JavaScript,
        }
    }

    /// Create TypeScript content.
    pub fn typescript(code: String) -> Self {
        Self::Text {
            code: Self::normalize_text_payload(code),
            file_type: FileType::TypeScript,
        }
    }

    /// Create TypeScript declaration content.
    pub fn declaration(code: String) -> Self {
        Self::Text {
            code: Self::normalize_text_payload(code),
            file_type: FileType::TypeScriptDeclaration,
        }
    }

    /// Create HTML content.
    pub fn html(code: String) -> Self {
        Self::Text {
            code: Self::normalize_text_payload(code),
            file_type: FileType::Html,
        }
    }

    /// Create JSON content.
    pub fn json(content: String, value: serde_json::Value, file_type: FileType) -> Self {
        Self::Json {
            content: Self::normalize_text_payload(content),
            value,
            file_type,
        }
    }

    /// Create source map content.
    pub fn source_map(value: &SourceMapArtifact) -> Result<Self, serde_json::Error> {
        let content = serde_json::to_string(value)?;
        let value = value.to_json_value()?;
        Ok(Self::json(content, value, FileType::SourceMap))
    }

    /// Create WebAssembly content.
    pub fn wasm(bytes: Vec<u8>) -> Self {
        Self::Binary {
            bytes,
            file_type: FileType::Wasm,
        }
    }

    /// Create object content.
    pub fn object(bytes: Vec<u8>) -> Self {
        Self::Binary {
            bytes,
            file_type: FileType::Object,
        }
    }

    /// Return the file type for this content.
    pub fn file_type(&self) -> FileType {
        match self {
            Self::Text { file_type, .. }
            | Self::Json { file_type, .. }
            | Self::Binary { file_type, .. } => *file_type,
        }
    }

    /// Convert this content to file content.
    pub fn to_file_content(&self) -> FileContent {
        match self {
            Self::Text { code, .. } => FileContent::Text {
                content: code.clone(),
            },
            Self::Json { content, .. } => FileContent::Text {
                content: content.clone(),
            },
            Self::Binary { bytes, .. } => FileContent::Binary {
                content: bytes.clone(),
            },
        }
    }
}

/// One derived output file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFile {
    /// The output URI.
    pub uri: Uri,
    /// The output content.
    pub content: OutputContent,
    /// The related source URI when one exists.
    pub source: Option<Uri>,
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
}
