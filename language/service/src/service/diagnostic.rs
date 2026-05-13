use std::collections::HashMap;
use std::sync::Arc;

use destack_source::{Diagnostic, File, FileId, Uri};
use destack_workspace::{Repository, Revision};

use super::LanguageServiceError;

/// Diagnostic snapshot for one file.
#[derive(Debug, Clone)]
pub struct DiagnosticSnapshot {
    /// The current file image used for range conversion.
    pub file: Arc<File>,
    /// Diagnostic uri for this snapshot.
    pub diagnostic_uri: Uri,
    /// Protocol file version for diagnostics when the file is open.
    pub diagnostic_version: Option<i32>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Return diagnostics grouped by primary file.
pub(super) fn diagnostics_by_file(
    repository: &Repository,
    revision: Revision,
) -> Result<HashMap<FileId, Vec<Diagnostic>>, LanguageServiceError> {
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
