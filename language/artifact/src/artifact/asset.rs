use destack_core::Blob;
use destack_serde::Reflect;
use destack_source::{FileType, Uri};
use serde::{Deserialize, Serialize};

/// One opaque linker input for a target.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Asset {
    /// The asset file type.
    pub file_type: FileType,
    /// The exact asset bytes.
    pub blob: Blob,
    /// The source module URI when one exists.
    pub source: Option<Uri>,
}

impl Asset {
    /// Create one asset.
    pub fn new(file_type: FileType, blob: Blob, source: Option<Uri>) -> Self {
        Self {
            file_type,
            blob,
            source,
        }
    }

    /// Return every Blob referenced by this asset.
    pub fn blobs(&self) -> Vec<Blob> {
        vec![self.blob]
    }
}
