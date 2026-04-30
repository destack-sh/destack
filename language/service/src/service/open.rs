use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_session::Session;
use destack_source::Uri;

use super::{LanguageService, LanguageServiceError};

impl LanguageService {
    /// Resolve an already opened session for one open file path.
    fn existing_open_session(
        &self,
        path: &Path,
    ) -> Result<Option<Arc<Session>>, LanguageServiceError> {
        if let Some(root) = self.owned_root(path) {
            let session = self
                .roots
                .get(root.as_path())
                .map(|entry| Arc::clone(entry.value()));

            return Ok(session);
        }

        let Some(root) = self.semantic_root(path)? else {
            return Ok(None);
        };
        let session = self
            .roots
            .get(root.as_path())
            .map(|entry| Arc::clone(entry.value()));

        Ok(session)
    }

    /// Return true when a path is open in its owning session.
    pub fn has_open_file(&self, path: &Path) -> bool {
        let Ok(Some(session)) = self.existing_open_session(path) else {
            return false;
        };

        session.contains_open_file(path)
    }

    /// Read one open file from its owning session.
    pub fn read_open_file(
        &self,
        path: &Path,
    ) -> Result<Option<(Uri, i32, String)>, LanguageServiceError> {
        let Some(session) = self.existing_open_session(path)? else {
            return Ok(None);
        };
        let file = session
            .open_file(path)
            .map_err(|error| LanguageServiceError::Internal {
                detail: format!("failed to read open file {}: {error}", path.display()),
            })?;

        Ok(file)
    }

    /// Return the open files keyed by path.
    pub fn open_files(&self) -> Vec<(PathBuf, Uri, i32)> {
        let mut files = Vec::new();

        for entry in self.roots.iter() {
            files.extend(entry.value().open_files());
        }

        files
    }
}
