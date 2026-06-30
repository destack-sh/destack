use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::StringId;
use destack_source::ContentId;

/// Loadable native library image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Library {
    /// The native library location.
    pub source: LibrarySource,
    /// Native unwind tables bytes.
    pub unwind: Option<ContentId>,
}

impl Library {
    /// Create one native library image.
    pub fn new(source: LibrarySource, unwind: Option<ContentId>) -> Self {
        Self { source, unwind }
    }

    /// Return all content ids referenced by this library image.
    pub fn content_ids(&self) -> Vec<ContentId> {
        let mut ids = Vec::with_capacity(2);

        if let LibrarySource::Artifact(content) = self.source {
            ids.push(content);
        }

        if let Some(unwind) = self.unwind {
            ids.push(unwind);
        }

        ids
    }
}

/// Native library source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum LibrarySource {
    /// Library is loaded from a process or platform search path.
    Name(StringId),
    /// Library is packaged as a program artifact.
    Artifact(ContentId),
}
