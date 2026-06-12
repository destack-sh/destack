use destack_source::FileType;
use serde::{Deserialize, Serialize};

use crate::{JsOutput, SourceMapArtifact};

/// One generated module output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleOutput {
    /// One generated JS output.
    Js(Box<JsOutput>),
    /// One generated native output.
    Native(Box<NativeOutput>),
}

/// One generated native output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeOutput {
    /// The generated output file type.
    pub file_type: FileType,
    /// The generated native payload bytes.
    pub bytes: Vec<u8>,
    /// The generated source map payload when one exists.
    pub source_map: Option<SourceMapArtifact>,
}

impl NativeOutput {
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
