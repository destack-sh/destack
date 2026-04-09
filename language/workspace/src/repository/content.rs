use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{FileContent, FileContentEntry};
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};

/// The exact identity of one normalized source content payload.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FileContentId(pub u64);

impl Display for FileContentId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "c{:016x}", self.0)
    }
}

impl FileContentId {
    /// Build one content id from one raw hash value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Shared immutable source content storage.
#[derive(Debug, Default)]
pub struct FileContentStore {
    /// Content payloads by exact content identity.
    content_by_id: DashMap<FileContentId, Arc<FileContentEntry>>,
}

impl FileContentStore {
    /// Create one empty content store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern one content payload and return its exact identity.
    pub fn intern(&self, content: FileContent) -> FileContentId {
        let content = normalize_content(content);
        let content_id = content_id_for(&content);

        self.content_by_id
            .entry(content_id)
            .or_insert_with(|| Arc::new(FileContentEntry::new(content)));

        content_id
    }

    /// Get one shared content payload.
    pub fn get(&self, content_id: FileContentId) -> Option<Arc<FileContentEntry>> {
        self.content_by_id
            .get(&content_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Return whether one content payload exists.
    pub fn contains(&self, content_id: FileContentId) -> bool {
        self.content_by_id.contains_key(&content_id)
    }

    /// Retain only the reachable content ids.
    pub fn retain_reachable(&self, reachable: &HashSet<FileContentId>) {
        self.content_by_id
            .retain(|content_id, _| reachable.contains(content_id));
    }
}

fn normalize_content(content: FileContent) -> FileContent {
    match content {
        FileContent::Text { content } => FileContent::Text {
            content: normalize_text(content),
        },
        FileContent::Binary { content } => FileContent::Binary { content },
    }
}

fn normalize_text(content: String) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}

fn content_id_for(content: &FileContent) -> FileContentId {
    let mut hasher = FxHasher::default();

    // variant tag
    match content {
        FileContent::Text { content } => {
            0_u8.hash(&mut hasher);
            content.hash(&mut hasher);
        }
        FileContent::Binary { content } => {
            1_u8.hash(&mut hasher);
            content.hash(&mut hasher);
        }
    }

    FileContentId::new(hasher.finish())
}
