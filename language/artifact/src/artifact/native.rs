use destack_source::{ContentId, FileType};
use serde::{Deserialize, Serialize};

use crate::SourceMapArtifact;

/// One emitted native output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeOutput {
    /// The emitted output file type.
    pub file_type: FileType,
    /// The emitted native payload content identity.
    pub content: ContentId,
    /// The emitted source map payload when one exists.
    pub source_map: Option<SourceMapArtifact>,
}

impl NativeOutput {
    /// Create one native output.
    pub fn new(
        file_type: FileType,
        content: ContentId,
        source_map: Option<SourceMapArtifact>,
    ) -> Self {
        Self {
            file_type,
            content,
            source_map,
        }
    }

    /// Create one native object output.
    pub fn object(content: ContentId) -> Self {
        Self {
            file_type: FileType::Object,
            content,
            source_map: None,
        }
    }

    /// Create one wasm output.
    pub fn wasm(content: ContentId, source_map: Option<SourceMapArtifact>) -> Self {
        Self {
            file_type: FileType::Wasm,
            content,
            source_map,
        }
    }

    /// Return all content ids referenced by this native output.
    pub fn content_ids(&self) -> Vec<ContentId> {
        vec![self.content]
    }
}
