use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use destack_repository::{Repository, Revision};
use destack_source::{Diagnostic, File, FileId, Uri};

use crate::diagnostic::Error;
use crate::workspace::Workspace;

/// Diagnostic view for one file.
#[derive(Debug, Clone)]
pub struct DiagnosticView {
    /// The current file image used for range conversion.
    pub file: Arc<File>,
    /// Diagnostic uri for this view.
    pub diagnostic_uri: Uri,
    /// Protocol file version for diagnostics when the file is open.
    pub diagnostic_version: Option<i32>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Return diagnostics grouped by primary file.
pub(crate) fn diagnostics_by_file(
    repository: &Repository,
    revision: Revision,
) -> Result<HashMap<FileId, Vec<Diagnostic>>, Error> {
    let diagnostics = repository.diagnostics(revision, None)?;
    let mut diagnostics_by_file = HashMap::new();

    // group diagnostics by the file that owns the primary label
    for diagnostic in diagnostics.iter() {
        let file_id = diagnostic.primary_label().span.file;
        diagnostics_by_file
            .entry(file_id)
            .or_insert_with(Vec::new)
            .push(diagnostic.clone());
    }

    Ok(diagnostics_by_file)
}

impl Workspace {
    /// Return a current diagnostic view for one file path.
    pub fn file_diagnostics(&self, path: &Path) -> Result<Option<DiagnosticView>, Error> {
        let root = self.root_at(path)?;
        let session = self.snapshot(&root)?;
        let revision = session.revision();
        let repository = session.repository();

        let Some(file_id) = session.file_id(path)? else {
            return Ok(None);
        };
        let file = session.file(file_id)?;
        let diagnostics = diagnostics_by_file(repository, revision)?
            .remove(&file_id)
            .unwrap_or_default();
        let open_file = file.path.as_ref().and_then(|path| self.open_state(path));
        let diagnostic_uri = open_file
            .as_ref()
            .map(|file| file.uri.clone())
            .or_else(|| file.path.as_ref().map(Uri::from_file_path))
            .unwrap_or_else(|| file.uri.clone());
        let diagnostic_version = if let Some(path) = file.path.as_ref() {
            self.open_file_version_in_revision(repository, revision, file_id, path)?
        } else {
            None
        };

        Ok(Some(DiagnosticView {
            file,
            diagnostic_uri,
            diagnostic_version,
            diagnostics,
        }))
    }

    /// Return current diagnostic views for every open root.
    pub fn diagnostics(&self) -> Result<Vec<DiagnosticView>, Error> {
        let mut roots = self.root_paths();
        roots.sort();

        let mut views = Vec::new();
        for root in roots {
            let root_views = self.root_diagnostics(&root)?;

            views.extend(root_views);
        }

        Ok(views)
    }

    /// Return current diagnostic views for one root.
    pub fn root_diagnostics(&self, root: &Path) -> Result<Vec<DiagnosticView>, Error> {
        let session = self.snapshot(root)?;
        let revision = session.revision();
        let repository = session.repository();
        let mut diagnostics_by_file = diagnostics_by_file(repository, revision)?;

        // include open files even when they have no diagnostics
        let mut open_files = HashMap::new();
        for (path, file) in self.open_files_under(root) {
            let Some(file_id) = session.file_id(&path)? else {
                continue;
            };
            let version =
                self.open_file_version_in_revision(repository, revision, file_id, &path)?;

            diagnostics_by_file.entry(file_id).or_insert(Vec::new());
            open_files.insert(file_id, (file.uri, version));
        }

        let mut views = Vec::new();
        for (file_id, diagnostics) in diagnostics_by_file {
            let Ok(file) = session.file(file_id) else {
                continue;
            };
            let open_file = open_files.get(&file_id);
            let diagnostic_uri = open_file
                .map(|(uri, _)| uri.clone())
                .or_else(|| file.path.as_ref().map(Uri::from_file_path))
                .unwrap_or_else(|| file.uri.clone());
            let diagnostic_version = open_file.and_then(|(_, version)| *version);

            views.push(DiagnosticView {
                file,
                diagnostic_uri,
                diagnostic_version,
                diagnostics,
            });
        }

        views.sort_by(|left, right| {
            let left_key = left
                .file
                .path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| left.file.uri.to_string());
            let right_key = right
                .file
                .path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| right.file.uri.to_string());

            left_key.cmp(&right_key)
        });

        Ok(views)
    }
}
