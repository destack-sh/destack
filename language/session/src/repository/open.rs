use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileSystem, OverlayFileSystem, Uri};
use parking_lot::RwLock;

/// One tracked open file owned by one workspace root.
#[derive(Debug, Clone)]
struct OpenFile {
    /// The current file path.
    path: PathBuf,
    /// The current file uri.
    uri: Uri,
    /// The current client file version.
    version: i32,
}

/// Open file identities and overlay text for one session.
#[derive(Debug)]
pub(crate) struct OpenFileOverlay {
    /// Overlay filesystem projection for open file content.
    overlay_file_system: Option<Arc<OverlayFileSystem>>,
    /// Tracked open files keyed by normalized path.
    files_by_path: RwLock<HashMap<PathBuf, OpenFile>>,
}

impl OpenFileOverlay {
    /// Create one open file overlay for a session.
    pub(crate) fn new(overlay_file_system: Option<Arc<OverlayFileSystem>>) -> Self {
        Self {
            overlay_file_system,
            files_by_path: RwLock::new(HashMap::new()),
        }
    }

    /// Track one open file by path.
    pub(crate) fn track_open_file(&self, path: &Path, uri: Uri, version: i32) {
        let open_file = OpenFile {
            path: path.to_path_buf(),
            uri,
            version,
        };

        self.files_by_path
            .write()
            .insert(path.to_path_buf(), open_file);
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
    pub(crate) fn contains_open_file(&self, path: &Path) -> bool {
        self.files_by_path.read().contains_key(path)
    }

    /// Return one open-file identity for a path.
    pub(crate) fn open_file_identity(&self, path: &Path) -> Option<(Uri, i32)> {
        let open_files = self.files_by_path.read();
        let entry = open_files.get(path)?;

        Some((entry.uri.clone(), entry.version))
    }

    /// Return one tracked open file for a path.
    pub(crate) fn open_file(&self, path: &Path) -> io::Result<Option<(Uri, i32, String)>> {
        let Some((uri, version)) = self.open_file_identity(path) else {
            return Ok(None);
        };
        let Some(text) = self.open_file_text(path)? else {
            return Ok(None);
        };

        Ok(Some((uri, version, text)))
    }

    /// Return the tracked open-file overlay text for a path when available.
    pub(crate) fn open_file_text(&self, path: &Path) -> io::Result<Option<String>> {
        if !self.contains_open_file(path) {
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
        self.files_by_path
            .read()
            .iter()
            .map(|entry| {
                let (_path, open_file) = entry;
                (
                    open_file.path.clone(),
                    open_file.uri.clone(),
                    open_file.version,
                )
            })
            .collect()
    }

    /// Stop tracking one open file and return its last known state.
    pub(crate) fn untrack_open_file(&self, path: &Path) -> Option<(Uri, i32)> {
        let open_file = self.files_by_path.write().remove(path)?;

        Some((open_file.uri, open_file.version))
    }
}
