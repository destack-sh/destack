use std::path::{Path, PathBuf};

use crate::CacheStore;

/// Typed cache access for generated outputs.
#[derive(Debug)]
pub struct OutputStore<'a> {
    /// The underlying cache store.
    pub cache_store: &'a dyn CacheStore,
    /// The workspace cache root.
    pub cache_root: &'a Path,
}

impl<'a> OutputStore<'a> {
    /// Create a new output store view.
    pub fn new(cache_store: &'a dyn CacheStore, cache_root: &'a Path) -> Self {
        Self {
            cache_store,
            cache_root,
        }
    }

    /// Return the output cache root.
    pub fn root(&self) -> PathBuf {
        self.cache_root.join("outputs")
    }
}
