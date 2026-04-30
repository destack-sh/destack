use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileSystem, OverlayFileSystem, Uri};
use parking_lot::RwLock;

use crate::session::OpenFile;

/// File text projected over filesystem-backed source for one session.
#[derive(Debug)]
pub(crate) struct SessionOverlay {
    /// Overlay filesystem used to read open file text.
    overlay_file_system: Option<Arc<OverlayFileSystem>>,
    /// Open file state keyed by normalized path.
    file_by_path: RwLock<HashMap<PathBuf, OpenFile>>,
}

impl SessionOverlay {
    /// Create an overlay for one session.
    pub(crate) fn new(overlay_file_system: Option<Arc<OverlayFileSystem>>) -> Self {
        Self {
            overlay_file_system,
            file_by_path: RwLock::new(HashMap::new()),
        }
    }

    /// Track one open file by path.
    pub(crate) fn track_file(&self, path: &Path, uri: Uri) {
        // store open file state under the stable repository path
        let path = Self::normalized_path(path);
        let file = OpenFile { uri };

        self.file_by_path.write().insert(path, file);
    }

    /// Set overlay text for one file when available.
    pub(crate) fn set_file_text(&self, path: &Path, text: String) {
        // sessions without an overlay are pure repository sessions
        let Some(overlay_file_system) = self.overlay_file_system.as_ref() else {
            return;
        };
        let path = Self::normalized_path(path);

        overlay_file_system.set_overlay(path.as_path(), text);
    }

    /// Remove overlay text for one file when available.
    pub(crate) fn remove_file_text(&self, path: &Path) {
        // sessions without an overlay are pure repository sessions
        let Some(overlay_file_system) = self.overlay_file_system.as_ref() else {
            return;
        };
        let path = Self::normalized_path(path);

        overlay_file_system.remove_overlay(path.as_path());
    }

    /// Return true when a path is tracked as an open file.
    pub(crate) fn contains_file(&self, path: &Path) -> bool {
        let path = Self::normalized_path(path);

        self.file_by_path.read().contains_key(path.as_path())
    }

    /// Return one open file for a path.
    pub(crate) fn file(&self, path: &Path) -> Option<OpenFile> {
        let path = Self::normalized_path(path);
        let file_by_path = self.file_by_path.read();
        let file = file_by_path.get(path.as_path())?;

        Some(file.clone())
    }

    /// Return one tracked open file for a path.
    pub(crate) fn open_file(&self, path: &Path) -> io::Result<Option<(Uri, String)>> {
        let Some(file) = self.file(path) else {
            return Ok(None);
        };
        let Some(text) = self.file_text(path)? else {
            return Ok(None);
        };

        Ok(Some((file.uri, text)))
    }

    /// Return tracked overlay text for one file when available.
    pub(crate) fn file_text(&self, path: &Path) -> io::Result<Option<String>> {
        // ignore paths outside the overlay
        if !self.contains_file(path) {
            return Ok(None);
        }

        let Some(overlay_file_system) = self.overlay_file_system.as_ref() else {
            return Ok(None);
        };
        let path = Self::normalized_path(path);
        let text = overlay_file_system.read_to_string(path.as_path())?;

        Ok(Some(text))
    }

    /// Return tracked open files keyed by path.
    pub(crate) fn files(&self) -> Vec<(PathBuf, Uri)> {
        self.file_by_path
            .read()
            .iter()
            .map(|entry| {
                let (path, file) = entry;
                (path.clone(), file.uri.clone())
            })
            .collect()
    }

    /// Stop tracking one open file and return its last known state.
    pub(crate) fn untrack_file(&self, path: &Path) -> Option<OpenFile> {
        let path = Self::normalized_path(path);
        let file = self.file_by_path.write().remove(path.as_path())?;

        Some(file)
    }

    /// Return the canonical path when available, otherwise the original path.
    fn normalized_path(path: &Path) -> PathBuf {
        // prefer stable paths for existing files
        match std::fs::canonicalize(path) {
            Ok(path) => path,
            Err(_error) => path.to_path_buf(),
        }
    }
}
