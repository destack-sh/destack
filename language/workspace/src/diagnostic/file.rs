use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{Repository, Revision};
use destack_source::{Diagnostic, File, FileId, Uri};

use crate::diagnostic::Error;
use crate::workspace::LocalWorkspace;

/// Selection for one diagnostic read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticsRequest {
    /// Return diagnostics for every open root.
    All,
    /// Return diagnostics for one root.
    Root(PathBuf),
    /// Return diagnostics for one file.
    File(PathBuf),
}

/// Diagnostics for one file at one exact semantic revision.
#[derive(Debug, Clone)]
pub struct FileDiagnostics {
    /// The semantic revision containing these diagnostics.
    pub revision: Revision,
    /// The file used for range conversion.
    pub file: Arc<File>,
    /// The URI published to the editor.
    pub uri: Uri,
    /// The editor document version when the file is open.
    pub version: Option<i32>,
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
        let file_id = diagnostic.primary_label().target.file();
        diagnostics_by_file
            .entry(file_id)
            .or_insert_with(Vec::new)
            .push(diagnostic.clone());
    }

    Ok(diagnostics_by_file)
}

impl LocalWorkspace {
    /// Return exact diagnostics for one file path.
    fn diagnose_file(&self, path: &Path) -> Result<Option<FileDiagnostics>, Error> {
        let root = self.root_at(path)?;
        let session = self.pin_session(&root)?;
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
        let uri = open_file
            .as_ref()
            .map(|file| file.uri.clone())
            .or_else(|| file.path.as_ref().map(Uri::from_file_path))
            .unwrap_or_else(|| file.uri.clone());
        let version = if let Some(path) = file.path.as_ref() {
            self.open_file_version_in_revision(repository, revision, file_id, path)?
        } else {
            None
        };

        Ok(Some(FileDiagnostics {
            revision,
            file,
            uri,
            version,
            diagnostics,
        }))
    }

    /// Return exact diagnostics for one root.
    fn diagnose_root(&self, root: &Path) -> Result<Vec<FileDiagnostics>, Error> {
        let session = self.pin_session(root)?;
        let revision = session.revision();
        let repository = session.repository();
        let mut diagnostics_by_file = diagnostics_by_file(repository, revision)?;

        // include open files even when they have no diagnostics
        let mut open_files = HashMap::new();
        for (path, file) in self.open_files_under(root) {
            let Some(file_id) = session.file_id(&path)? else {
                return Err(Error::FileMissing { path });
            };
            let version =
                self.open_file_version_in_revision(repository, revision, file_id, &path)?;

            diagnostics_by_file.entry(file_id).or_insert(Vec::new());
            open_files.insert(file_id, (file.uri, version));
        }

        let mut diagnostics_by_source = Vec::new();
        for (file_id, diagnostics) in diagnostics_by_file {
            let file = session.file(file_id)?;
            let open_file = open_files.get(&file_id);
            let uri = open_file
                .map(|(uri, _)| uri.clone())
                .or_else(|| file.path.as_ref().map(Uri::from_file_path))
                .unwrap_or_else(|| file.uri.clone());
            let version = open_file.and_then(|(_, version)| *version);

            diagnostics_by_source.push(FileDiagnostics {
                revision,
                file,
                uri,
                version,
                diagnostics,
            });
        }

        diagnostics_by_source.sort_by(|left, right| {
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

        Ok(diagnostics_by_source)
    }

    /// Return exact diagnostics selected by one request.
    pub fn diagnose(&self, request: DiagnosticsRequest) -> Result<Vec<FileDiagnostics>, Error> {
        match request {
            DiagnosticsRequest::All => {
                let mut roots = self.root_paths();
                roots.sort();

                let mut diagnostics = Vec::new();
                for root in roots {
                    diagnostics.extend(self.diagnose_root(&root)?);
                }

                Ok(diagnostics)
            }
            DiagnosticsRequest::Root(root) => self.diagnose_root(&root),
            DiagnosticsRequest::File(path) => {
                let diagnostics = self.diagnose_file(&path)?;

                Ok(diagnostics.into_iter().collect())
            }
        }
    }
}
