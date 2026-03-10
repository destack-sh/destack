use std::path::{Path, PathBuf};

use crate::CacheStore;

/// Typed cache access for semantic artifacts.
#[derive(Debug)]
pub struct ArtifactStore<'a> {
    /// The underlying cache store.
    pub cache_store: &'a dyn CacheStore,
    /// The workspace cache root.
    pub cache_root: &'a Path,
}

impl<'a> ArtifactStore<'a> {
    /// Create a new artifact store view.
    pub fn new(cache_store: &'a dyn CacheStore, cache_root: &'a Path) -> Self {
        Self {
            cache_store,
            cache_root,
        }
    }

    /// Return the semantic artifact cache root.
    pub fn root(&self) -> PathBuf {
        self.cache_root.join("artifacts")
    }
}
