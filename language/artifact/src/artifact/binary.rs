use destack_source::FileType;
use serde::{Deserialize, Serialize};

use crate::{ScriptOutput, SourceMapArtifact};

/// One generated module output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleOutput {
    /// One generated script output.
    Script(ScriptOutput),
    /// One generated binary output.
    Binary(BinaryOutput),
}

/// One generated binary output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryOutput {
    /// The generated output file type.
    pub file_type: FileType,
    /// The generated binary payload bytes.
    pub bytes: Vec<u8>,
    /// The generated source map payload when one exists.
    pub source_map: Option<SourceMapArtifact>,
}

impl BinaryOutput {
    /// Create one native object output.
    pub fn object(bytes: Vec<u8>) -> Self {
        Self {
            file_type: FileType::Object,
            bytes,
            source_map: None,
        }
    }

    /// Create one wasm output.
    pub fn wasm(bytes: Vec<u8>, source_map: Option<SourceMapArtifact>) -> Self {
        Self {
            file_type: FileType::Wasm,
            bytes,
            source_map,
        }
    }
}
