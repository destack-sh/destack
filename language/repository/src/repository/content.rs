use std::collections::HashSet;
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{Content, ContentEntry, ContentId};

/// Shared immutable in-process content pool.
#[derive(Debug, Default)]
pub(crate) struct ContentPool {
    /// Content payloads by exact content identity.
    content_by_id: DashMap<ContentId, Arc<ContentEntry>>,
}

impl ContentPool {
    /// Create one empty content pool.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Insert one identified content payload and return its shared entry.
    pub(crate) fn insert(&self, content_id: ContentId, content: Content) -> Arc<ContentEntry> {
        let entry = self
            .content_by_id
            .entry(content_id)
            .or_insert_with(|| Arc::new(ContentEntry::new(content)));

        Arc::clone(entry.value())
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
