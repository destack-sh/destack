use destack_source::{FileContent, FileType, Uri};
use serde::{Deserialize, Serialize};

use crate::EmitFormat;

/// One derived output entry payload.
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
    /// Create JavaScript content.
    pub fn javascript(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::JavaScript,
        }
    }

    /// Create TypeScript content.
    pub fn typescript(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::TypeScript,
        }
    }

    /// Create TypeScript declaration content.
    pub fn declaration(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::TypeScriptDeclaration,
        }
    }

    /// Create HTML content.
    pub fn html(code: String) -> Self {
        Self::Text {
            code,
            file_type: FileType::Html,
        }
    }

    /// Create source map content.
    pub fn source_map(value: serde_json::Value) -> Self {
        let content = serde_json::to_string(&value).unwrap_or_default();
        Self::Json {
            content,
            value,
            file_type: FileType::SourceMap,
        }
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
            Self::Json { content, value, .. } => FileContent::Json {
                content: content.clone(),
                value: value.clone(),
            },
            Self::Binary { bytes, .. } => FileContent::Binary {
                content: bytes.clone(),
            },
        }
    }
}

/// One derived output entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEntry {
    /// The output URI.
    pub uri: Uri,
    /// The output content.
    pub content: OutputContent,
    /// The related source URI when one exists.
    pub source: Option<Uri>,
}

/// One module target output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleOutput {
    /// The emitted artifact family.
    pub emit: EmitFormat,
    /// The output entries.
    pub entries: Vec<OutputEntry>,
}

impl ModuleOutput {
    /// Create one module output.
    pub fn new(emit: EmitFormat, entries: Vec<OutputEntry>) -> Self {
        Self { emit, entries }
    }
}

/// One package target output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackageOutput {
    /// The emitted artifact family.
    pub emit: EmitFormat,
    /// Whether this output was assembled across modules.
    pub assembled: bool,
    /// The output entries.
    pub entries: Vec<OutputEntry>,
}

impl PackageOutput {
    /// Create one package output.
    pub fn new(emit: EmitFormat, assembled: bool, entries: Vec<OutputEntry>) -> Self {
        Self {
            emit,
            assembled,
            entries,
        }
    }
}
