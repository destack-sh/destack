use destack_source::{ContentId, FileType, Uri};
use serde::{Deserialize, Serialize};

use crate::SourceMap;

/// One opaque linker input for a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// The asset file type.
    pub file_type: FileType,
    /// The asset content identity.
    pub content: ContentId,
    /// The source module URI when one exists.
    pub source: Option<Uri>,
    /// The source map when one exists.
    pub map: Option<SourceMap>,
}

impl Asset {
    /// Create one asset.
    pub fn new(
        file_type: FileType,
        content: ContentId,
        source: Option<Uri>,
        map: Option<SourceMap>,
    ) -> Self {
        Self {
            file_type,
            content,
            source,
            map,
        }
    }

    /// Return all content ids referenced by this asset.
    pub fn content_ids(&self) -> Vec<ContentId> {
        vec![self.content]
    }
}
