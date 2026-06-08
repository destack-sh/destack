use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{DestackLayoutOverride, Environment, Settings};
use destack_session::open_repository_from_fs;
use destack_source::{
    FileSystem, OverlayFileSystem, PhysicalFileSystem, TemporaryPhysicalFileSystem, Uri,
};

use crate::{FileChange, UpdateBatch, Workspace};

/// Test harness for workspace integration tests.
#[derive(Debug)]
pub(super) struct TestWorkspace {
    /// Temporary filesystem root.
    pub fs: TemporaryPhysicalFileSystem,
    /// Workspace under test.
    pub workspace: Workspace,
    /// Workspace roots registered in the workspace.
    pub roots: Vec<PathBuf>,
}

impl TestWorkspace {
    /// Create a new harness rooted at a temporary source root.
    pub(super) fn new(prefix: &str) -> Self {
        Self::new_with_roots(prefix, 1)
    }

    /// Create a new harness with explicit root count.
    pub(super) fn new_with_roots(prefix: &str, root_count: usize) -> Self {
        // create a temporary filesystem root for test files
        let fs = TemporaryPhysicalFileSystem::new_with_prefix(prefix);
        let roots = build_roots(&fs, root_count.max(1));
        let root = roots[0].clone();

        // create a repository with an overlay over physical fs
        let overlay = Arc::new(OverlayFileSystem::with_inner(Arc::new(
            PhysicalFileSystem::new(),
        )));
        let repository = Arc::new(
            open_repository_from_fs(
                root.clone(),
                overlay.clone(),
                Environment::capture_process(),
                Settings::default(),
                DestackLayoutOverride::default(),
            )
            .expect("failed to import repository from overlay fs"),
        );
        let workspace = Workspace::new(repository.clone(), Some(overlay), roots.clone(), 1, None)
            .expect("expected workspace");

        Self {
            fs,
            workspace,
            roots,
        }
    }

    /// Resolve a path under the temporary root.
    pub(super) fn path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        self.path_for_root(0, path)
    }

    /// Resolve a path under a specific source root.
    pub(super) fn path_for_root(&self, root_index: usize, path: impl AsRef<Path>) -> PathBuf {
        let root = self
            .roots
            .get(root_index)
            .unwrap_or_else(|| panic!("missing root index: {root_index}"));
        root.join(path.as_ref())
    }

    /// Build a source uri for a path.
    pub(super) fn uri_for_path(&self, path: &Path) -> Uri {
        Uri::from_file_path(path)
    }

    /// Write text under the default root.
    pub(super) fn write_text(&self, path: impl AsRef<Path>, source: &str) -> PathBuf {
        let path = self.path_for(path);
        self.fs
            .write_text(&path, source)
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
        path
    }

    /// Apply a text source update for a path.
    pub(super) fn apply_text(&self, path: &Path, source: &str) -> UpdateBatch {
        self.workspace
            .apply_file(
                path,
                FileChange::Text {
                    content: source.to_string(),
                },
            )
            .unwrap_or_else(|error| panic!("failed file update for {}: {error}", path.display()))
    }
}

/// Build source roots for a test harness.
fn build_roots(fs: &TemporaryPhysicalFileSystem, root_count: usize) -> Vec<PathBuf> {
    // keep a single root at the fs root for common cases
    if root_count == 1 {
        return vec![fs.root().to_path_buf()];
    }

    // create one sub root per source root
    let mut roots = Vec::new();
    for index in 0..root_count {
        let root = fs.path_for(format!("root-{index}"));
        std::fs::create_dir_all(&root)
            .unwrap_or_else(|error| panic!("failed to create root {}: {error}", root.display()));
        roots.push(root);
    }

    roots
}
