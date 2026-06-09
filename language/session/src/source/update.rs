use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_repository::{Ref, Revision};
use destack_source::{FileContent, FileId, Span, Uri, apply_file_edit};

use crate::{Change, Session, SessionError};

/// One text range in byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// One text replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Replaced byte range.
    pub range: TextRange,
    /// Replacement text.
    pub text: String,
}

/// One edit accepted by a session update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Replace or create one text file.
    SetText {
        /// Repository or session relative path.
        path: PathBuf,
        /// Full text content.
        text: String,
    },
    /// Apply text replacements to one tracked text file.
    EditText {
        /// Repository or session relative path.
        path: PathBuf,
        /// Text replacements.
        edits: Vec<TextEdit>,
    },
    /// Replace or create one binary file.
    SetBytes {
        /// Repository or session relative path.
        path: PathBuf,
        /// Full binary content.
        bytes: Vec<u8>,
    },
    /// Remove one file.
    Remove {
        /// Repository or session relative path.
        path: PathBuf,
    },
    /// Move one file.
    Move {
        /// Repository or session relative path.
        from: PathBuf,
        /// Destination repository or session relative path.
        to: PathBuf,
    },
}

impl Edit {
    /// Return the single file path affected by this edit.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::SetText { path, .. }
            | Self::EditText { path, .. }
            | Self::SetBytes { path, .. }
            | Self::Remove { path } => Some(path.as_path()),
            Self::Move { .. } => None,
        }
    }

    /// Return whether this edit removes its target file.
    pub fn is_remove(&self) -> bool {
        matches!(self, Self::Remove { .. })
    }

    /// Return full text content when this edit sets text directly.
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::SetText { text, .. } => Some(text),
            _ => None,
        }
    }
}

/// One committed edit batch.
#[derive(Debug, Clone)]
pub struct Commit {
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Changed files.
    pub changes: Vec<Change>,
}

impl Session {
    /// Edit files through one ref.
    pub fn edit(&self, reference: &Ref, edits: Vec<Edit>) -> Result<Commit, SessionError> {
        let before = self.revision(reference)?;

        self.edit_at(reference, before, edits)
    }

    /// Edit files when one ref still points at one revision.
    pub fn edit_at(
        &self,
        reference: &Ref,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, SessionError> {
        let current = self.revision(reference)?;
        if current != revision {
            return Err(SessionError::StaleRevision {
                reference: reference.clone(),
                expected: revision,
                current,
            });
        }

        self.commit_edits(reference, revision, edits)
    }

    /// Commit edits through one ref at one known revision.
    fn commit_edits(
        &self,
        reference: &Ref,
        before: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, SessionError> {
        let repository = self.repository();
        // build repository edits before pinning outputs
        let edits = self.repository_edits_for_edits(before, edits)?;
        let file_ids = repository_file_ids(&edits);
        let before_pin = repository.pin(before)?;
        let revision = repository.commit_edits(before, edits)?;
        let revision_pin = repository.pin(revision)?;

        // publish when the ref still points at the edited base
        self.publish_revision(reference, before, revision)?;

        let files =
            self.project_file_changes(before_pin.revision(), revision_pin.revision(), file_ids)?;

        Ok(Commit {
            before: before_pin.revision(),
            after: revision_pin.revision(),
            changes: files,
        })
    }

    /// Build repository edits from session edits.
    fn repository_edits_for_edits(
        &self,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Vec<destack_repository::Edit>, SessionError> {
        let mut repository_edits = Vec::new();

        // lower each session edit into repository edits
        for edit in edits {
            self.push_edit(revision, edit, &mut repository_edits)?;
        }

        Ok(repository_edits)
    }

    /// Add one session edit to one repository edit list.
    fn push_edit(
        &self,
        revision: Revision,
        edit: Edit,
        repository_edits: &mut Vec<destack_repository::Edit>,
    ) -> Result<(), SessionError> {
        match edit {
            Edit::SetText { path, text } => {
                let path = self.repository_path(&path);
                repository_edits.push(destack_repository::Edit::set_text(path, text));
            }
            Edit::EditText { path, edits } => {
                let path = self.repository_path(&path);
                let text = self.apply_text_edits(revision, &path, edits)?;
                repository_edits.push(destack_repository::Edit::set_text(path, text));
            }
            Edit::SetBytes { path, bytes } => {
                let path = self.repository_path(&path);
                repository_edits.push(destack_repository::Edit::SetFile {
                    logical_path: path,
                    content: FileContent::Binary { content: bytes },
                });
            }
            Edit::Remove { path } => {
                let path = self.repository_path(&path);
                repository_edits.push(destack_repository::Edit::remove_file(path));
            }
            Edit::Move { from, to } => {
                let from = self.repository_path(&from);
                let to = self.repository_path(&to);
                repository_edits.push(destack_repository::Edit::move_file(from, to));
            }
        }

        Ok(())
    }

    /// Apply text edits to one tracked file.
    fn apply_text_edits(
        &self,
        revision: Revision,
        path: &str,
        edits: Vec<TextEdit>,
    ) -> Result<String, SessionError> {
        let file_id = FileId::from_logical_str(path);

        // read the current tracked text
        let file = self
            .repository()
            .file(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        // lower session edits into source text edits
        let edits = edits
            .into_iter()
            .map(|edit| {
                destack_source::Edit::replace(
                    Span::new(file_id, edit.range.start, edit.range.end),
                    edit.text,
                )
            })
            .collect();

        // materialize the updated text
        let file_edit = destack_source::FileEdit::with_edits(file_id, edits);
        let text = apply_file_edit(file.as_ref(), &file_edit)?;

        Ok(text)
    }

    /// Project repository file changes into session file changes.
    pub(crate) fn project_file_changes(
        &self,
        before: Revision,
        revision: Revision,
        file_changes: Vec<FileId>,
    ) -> Result<Vec<Change>, SessionError> {
        let mut files = Vec::new();
        let repository = self.repository();

        // project changed repository files into session change payloads
        let mut seen_file_ids = HashSet::new();
        for file_id in file_changes {
            if !seen_file_ids.insert(file_id) {
                continue;
            }

            let file = repository
                .file(revision, file_id)
                .map_err(SessionError::from)?;

            // live files carry their current repository image
            if let Some(file) = file {
                let module_id = repository
                    .module_id_for_file(revision, file_id)
                    .map_err(SessionError::from)?;
                let path = file.path.as_deref().ok_or_else(|| SessionError::Internal {
                    detail: format!("updated repository file has no path: {file_id:?}"),
                })?;
                let uri = Uri::from_file_path(path);

                files.push(Change::Updated {
                    module_id,
                    file_id,
                    uri,
                    file,
                });

                continue;
            }

            let previous_file = repository
                .file(before, file_id)
                .map_err(SessionError::from)?
                .ok_or(SessionError::FileNotTracked { file_id })?;
            let module_id = repository
                .module_id_for_file(before, file_id)
                .map_err(SessionError::from)?;
            let path = previous_file
                .path
                .as_deref()
                .ok_or_else(|| SessionError::Internal {
                    detail: format!("removed repository file has no path: {file_id:?}"),
                })?;
            // removed files still need a uri so clients can clear diagnostics
            let uri = Uri::from_file_path(path);

            files.push(Change::Removed {
                module_id,
                file_id,
                uri,
            });
        }

        Ok(files)
    }
}

/// Return file ids affected by repository edits.
fn repository_file_ids(edits: &[destack_repository::Edit]) -> Vec<FileId> {
    let mut file_ids = Vec::new();
    let mut seen_file_ids = HashSet::new();

    // collect changed file ids in edit order
    for edit in edits {
        for file_id in edit.affected_file_ids() {
            if seen_file_ids.insert(file_id) {
                file_ids.push(file_id);
            }
        }
    }

    file_ids
}
