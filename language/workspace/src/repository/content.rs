use std::hash::{Hash, Hasher};
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::FileContent;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};

/// The exact identity of one normalized source content payload.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentId(pub u64);

impl std::fmt::Display for ContentId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "c{:016x}", self.0)
    }
}

impl ContentId {
    /// Build one content id from one raw hash value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Shared immutable source content storage.
#[derive(Debug, Default)]
pub struct ContentStore {
    /// Content payloads by exact content identity.
    by_id: DashMap<ContentId, Arc<FileContent>>,
}

impl ContentStore {
    /// Create one empty content store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern one content payload and return its exact identity.
    pub fn intern(&self, content: FileContent) -> ContentId {
        let content = normalize_content(content);
        let content_id = content_id_for(&content);

        self.by_id
            .entry(content_id)
            .or_insert_with(|| Arc::new(content));

        content_id
    }

    /// Get one shared content payload.
    pub fn get(&self, content_id: ContentId) -> Option<Arc<FileContent>> {
        self.by_id
            .get(&content_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Return whether one content payload exists.
    pub fn contains(&self, content_id: ContentId) -> bool {
        self.by_id.contains_key(&content_id)
    }
}

fn normalize_content(content: FileContent) -> FileContent {
    match content {
        FileContent::Text { content } => FileContent::Text {
            content: normalize_text(content),
        },
        FileContent::Json { content, value } => FileContent::Json {
            content: normalize_text(content),
            value,
        },
        FileContent::Binary { content } => FileContent::Binary { content },
        FileContent::Missing => FileContent::Missing,
        FileContent::Unloaded => FileContent::Unloaded,
    }
}

fn normalize_text(content: String) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}

fn content_id_for(content: &FileContent) -> ContentId {
    let mut hasher = FxHasher::default();

    // variant tag
    match content {
        FileContent::Text { content } => {
            0_u8.hash(&mut hasher);
            content.hash(&mut hasher);
        }
        FileContent::Json { content, .. } => {
            1_u8.hash(&mut hasher);
            content.hash(&mut hasher);
        }
        FileContent::Binary { content } => {
            2_u8.hash(&mut hasher);
            content.hash(&mut hasher);
        }
        FileContent::Missing => {
            3_u8.hash(&mut hasher);
        }
        FileContent::Unloaded => {
            4_u8.hash(&mut hasher);
        }
    }

    ContentId::new(hasher.finish())
}
