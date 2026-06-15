use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{Content, ContentEntry, ContentId};

/// Shared immutable content storage.
#[derive(Debug, Default)]
pub(crate) struct ContentStore {
    /// Content payloads by exact content identity.
    content_by_id: DashMap<ContentId, Arc<ContentEntry>>,
}

impl ContentStore {
    /// Create one empty content store.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Intern one content payload and return its exact identity.
    pub(crate) fn intern(&self, content: Content) -> ContentId {
        let content_id = ContentId::for_content(&content);

        self.content_by_id
            .entry(content_id)
            .or_insert_with(|| Arc::new(ContentEntry::new(content)));

        content_id
    }

    /// Get one shared content payload.
    pub(crate) fn get(&self, content_id: ContentId) -> Option<Arc<ContentEntry>> {
        self.content_by_id
            .get(&content_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Retain only the reachable content ids.
    pub(crate) fn retain_reachable(&self, reachable: &HashSet<ContentId>) {
        self.content_by_id
            .retain(|content_id, _| reachable.contains(content_id));
    }
}
