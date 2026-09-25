use serde::{Deserialize, Serialize};
use tspp_core::Blob;
use tspp_serde::Reflect;
use tspp_source::{FileType, Uri};

use crate::SourceMap;

/// One opaque linker input for a target.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct Asset {
    /// The asset file type.
    pub file_type: FileType,
    /// The exact asset bytes.
    pub blob: Blob,
    /// The source module URI when one exists.
    pub source: Option<Uri>,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
}

impl Asset {
    /// Create one asset.
    pub fn new(
        file_type: FileType,
        blob: Blob,
        source: Option<Uri>,
        map: Option<SourceMap>,
    ) -> Self {
        Self {
            file_type,
            blob,
            source,
            map,
        }
    }

    /// Return every Blob referenced by this asset.
    pub fn blobs(&self) -> Vec<Blob> {
        vec![self.blob]
    }
}
