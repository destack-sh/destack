use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use destack_source::MemoryFileSystem;
use destack_workspace::{MemoryCacheStore, Session};

/// One shared in memory workspace for suite execution.
#[derive(Debug)]
pub struct SharedMemoryWorkspace {
    /// The shared session.
    session: Arc<Session>,
    /// The shared in memory file system.
    fs: Arc<MemoryFileSystem>,
    /// The root directory prefix for allocated cases.
    root: PathBuf,
    /// The next unique case id.
    next_id: AtomicUsize,
}

impl SharedMemoryWorkspace {
    /// Create one shared in memory workspace rooted at the given path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let fs = Arc::new(MemoryFileSystem::new());
        let session = Arc::new(
            Session::new(root.clone())
                .with_fs(fs.clone())
                .with_cache_store(Arc::new(MemoryCacheStore::new())),
        );

        Self {
            session,
            fs,
            root,
            next_id: AtomicUsize::new(0),
        }
    }

    /// Return the shared session.
    pub fn session(&self) -> Arc<Session> {
        self.session.clone()
    }

    /// Return the shared in memory file system.
    pub fn fs(&self) -> Arc<MemoryFileSystem> {
        self.fs.clone()
    }

    /// Return the workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Allocate one unique case root under the workspace root.
    pub fn allocate_root(&self, label: &str) -> PathBuf {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.root.join(format!("{label}-{id}"))
    }
}
