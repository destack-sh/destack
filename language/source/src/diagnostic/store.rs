use std::collections::HashMap;

use parking_lot::RwLock;

use crate::{Diagnostic, FileId, FileVersion};

/// Update payload for a file's diagnostics in the store.
#[derive(Debug, Clone)]
pub struct DiagnosticStoreUpdate {
    /// The file id being updated.
    pub file_id: FileId,
    /// The file version for this update.
    pub file_version: FileVersion,
    /// Diagnostics for the file.
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticStoreUpdate {
    /// Create a new diagnostic store update.
    pub fn new(file_id: FileId, file_version: FileVersion, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            file_id,
            file_version,
            diagnostics,
        }
    }
}

/// Entry for a single file in the diagnostic store.
#[derive(Debug, Clone)]
struct DiagnosticStoreEntry {
    /// The file version captured for this entry.
    file_version: FileVersion,
    /// Diagnostics stored for the file.
    diagnostics: Vec<Diagnostic>,
}

/// A thread safe store of diagnostics keyed by file id.
#[derive(Debug, Default)]
pub struct DiagnosticStore {
    /// Diagnostics keyed by file id.
    entries: RwLock<HashMap<FileId, DiagnosticStoreEntry>>,
}

impl DiagnosticStore {
    /// Create a new diagnostic store.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Apply a batch of diagnostic updates.
    pub fn apply_updates(&self, updates: Vec<DiagnosticStoreUpdate>) {
        // lock the store for updates
        let mut entries = self.entries.write();

        // apply updates while skipping stale versions
        for update in updates {
            if let Some(existing) = entries.get(&update.file_id)
                && update.file_version < existing.file_version
            {
                continue;
            }

            entries.insert(
                update.file_id,
                DiagnosticStoreEntry {
                    file_version: update.file_version,
                    diagnostics: update.diagnostics,
                },
            );
        }
    }

    /// Clear diagnostics for the provided files.
    pub fn clear_files(&self, file_ids: &[FileId]) {
        // lock the store for updates
        let mut entries = self.entries.write();

        // remove entries for each file id
        for file_id in file_ids {
            entries.remove(file_id);
        }
    }

    /// Snapshot diagnostics for a single file.
    pub fn diagnostics_for_file(&self, file_id: FileId) -> Vec<Diagnostic> {
        // snapshot the current entry
        let entries = self.entries.read();
        let entry = entries.get(&file_id);

        // return diagnostics for the file
        match entry {
            Some(entry) => entry.diagnostics.clone(),
            None => Vec::new(),
        }
    }

    /// Snapshot diagnostics grouped by file.
    pub fn snapshot_by_file(&self) -> HashMap<FileId, Vec<Diagnostic>> {
        // snapshot all entries in the store
        let entries = self.entries.read();

        // clone diagnostics per file
        entries
            .iter()
            .map(|(file_id, entry)| (*file_id, entry.diagnostics.clone()))
            .collect()
    }

    /// Snapshot all diagnostics across files.
    pub fn snapshot_all(&self) -> Vec<Diagnostic> {
        // snapshot all entries in the store
        let entries = self.entries.read();

        // collect diagnostics across files
        let mut diagnostics = Vec::new();
        for entry in entries.values() {
            diagnostics.extend(entry.diagnostics.iter().cloned());
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use crate::{Diagnostic, DiagnosticSeverity, FileId, FileVersion, LabeledSpan, Span};

    use super::{DiagnosticStore, DiagnosticStoreUpdate};

    /// Build a diagnostic for a specific file.
    fn make_diagnostic(file_id: FileId, message: &str) -> Diagnostic {
        // build a minimal diagnostic for testing
        let span = Span::empty(file_id);
        let primary_span = LabeledSpan::new(span, "primary");

        Diagnostic {
            code: "D000".to_string(),
            original_code: None,
            severity: DiagnosticSeverity::Error,
            original_severity: None,
            message: message.to_string(),
            file_id,
            primary_span,
            primary_highlight_spans: None,
            secondary_spans: None,
            suggestions: None,
        }
    }

    /// Ensure updates are applied for the current file version.
    #[test]
    fn test_store_applies_updates() {
        // applies updates for matching file versions
        let store = DiagnosticStore::new();
        let file_id = FileId::new(1);
        let diagnostic = make_diagnostic(file_id, "first");
        let update = DiagnosticStoreUpdate::new(file_id, FileVersion::new(1), vec![diagnostic]);

        // apply the update
        store.apply_updates(vec![update]);

        // assert the diagnostics were stored
        let diagnostics = store.diagnostics_for_file(file_id);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "first");
    }

    /// Ensure stale updates do not overwrite newer diagnostics.
    #[test]
    fn test_store_skips_stale_updates() {
        // keeps newer diagnostics when stale updates arrive
        let store = DiagnosticStore::new();
        let file_id = FileId::new(2);
        let newer = make_diagnostic(file_id, "newer");
        let older = make_diagnostic(file_id, "older");

        // apply a newer update
        let update = DiagnosticStoreUpdate::new(file_id, FileVersion::new(2), vec![newer]);
        store.apply_updates(vec![update]);

        // attempt to apply a stale update
        let update = DiagnosticStoreUpdate::new(file_id, FileVersion::new(1), vec![older]);
        store.apply_updates(vec![update]);

        // assert the newer diagnostic remains
        let diagnostics = store.diagnostics_for_file(file_id);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].message, "newer");
    }
}
