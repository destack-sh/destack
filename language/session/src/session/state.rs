use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileSystem, OverlayFileSystem, Uri};
use parking_lot::RwLock;

/// One tracked open-file identity owned by one workspace root.
#[derive(Debug, Clone)]
struct OpenFileState {
    /// The current file path.
    pub path: PathBuf,
    /// The current file uri.
    pub uri: Uri,
    /// The current client file version.
    pub version: i32,
}

/// One combined overlay and open-file state set for a session.
#[derive(Debug)]
pub(crate) struct SessionState {
    /// Overlay filesystem projection for open file content.
    overlay_file_system: Option<Arc<OverlayFileSystem>>,
    /// Tracked open-file state keyed by normalized path.
    open_files_by_path: RwLock<Arc<HashMap<PathBuf, OpenFileState>>>,
}

impl SessionState {
    /// Create one file state set for a session.
    pub(crate) fn new(overlay_file_system: Option<Arc<OverlayFileSystem>>) -> Self {
        Self {
            overlay_file_system,
            open_files_by_path: RwLock::new(Arc::new(HashMap::new())),
        }
    }

    /// Track one open file by path.
    pub(crate) fn set_open_file(&self, path: &Path, uri: Uri, version: i32) {
        let key = canonical_path_or_original(path);
        let state = OpenFileState {
            path: path.to_path_buf(),
            uri,
            version,
        };
        let current = self.open_files_by_path.read().clone();
        let mut open_files = (*current).clone();
        open_files.insert(key, state);
        *self.open_files_by_path.write() = Arc::new(open_files);
    }

    /// Set one overlay projection when available.
    pub(crate) fn set_overlay_for_path(&self, path: &Path, text: String) {
        let Some(overlay_file_system) = self.overlay_file_system.as_ref() else {
            return;
        };

        overlay_file_system.set_overlay(path, text);
    }

    /// Remove one overlay projection when available.
    pub(crate) fn remove_overlay_for_path(&self, path: &Path) {
        let Some(overlay_file_system) = self.overlay_file_system.as_ref() else {
            return;
        };

        overlay_file_system.remove_overlay(path);
    }

    /// Return true when a path is tracked as one open file.
    pub(crate) fn has_open_file_for_path(&self, path: &Path) -> bool {
        let key = canonical_path_or_original(path);
        self.open_files_by_path.read().contains_key(&key)
    }

    /// Return one open-file identity for a path.
    pub(crate) fn open_file_identity_for_path(&self, path: &Path) -> Option<(Uri, i32)> {
        let key = canonical_path_or_original(path);
        let current = self.open_files_by_path.read().clone();
        let entry = current.get(&key)?;

        Some((entry.uri.clone(), entry.version))
    }

    /// Return one tracked open-file snapshot for a path.
    pub(crate) fn open_file_for_path(&self, path: &Path) -> io::Result<Option<(Uri, i32, String)>> {
        let Some((uri, version)) = self.open_file_identity_for_path(path) else {
            return Ok(None);
        };
        let Some(text) = self.open_file_text_for_path(path)? else {
            return Ok(None);
        };

        Ok(Some((uri, version, text)))
    }

    /// Return the tracked open-file overlay text for a path when available.
    pub(crate) fn open_file_text_for_path(&self, path: &Path) -> io::Result<Option<String>> {
        if !self.has_open_file_for_path(path) {
            return Ok(None);
        }

        let Some(overlay_file_system) = self.overlay_file_system.as_ref() else {
            return Ok(None);
        };
        let text = overlay_file_system.read_to_string(path)?;

        Ok(Some(text))
    }

    /// Return the tracked open files keyed by path.
    pub(crate) fn open_file_identities(&self) -> Vec<(PathBuf, Uri, i32)> {
        self.open_files_by_path
            .read()
            .iter()
            .map(|entry| {
                let (_path, state) = entry;
                (state.path.clone(), state.uri.clone(), state.version)
            })
            .collect()
    }

    /// Stop tracking one open file and return its last known state.
    pub(crate) fn close_open_file(&self, path: &Path) -> Option<(Uri, i32)> {
        let key = canonical_path_or_original(path);
        let current = self.open_files_by_path.read().clone();
        let state = current.get(&key)?.clone();
        let mut open_files = (*current).clone();
        open_files.remove(&key);
        *self.open_files_by_path.write() = Arc::new(open_files);

        Some((state.uri, state.version))
    }
}

/// Return the canonical path when available, otherwise the original path.
pub fn canonical_path_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
