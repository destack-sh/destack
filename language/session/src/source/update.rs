use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_repository::{Edit as RepositoryEdit, Ref, RepositoryChange, Revision};
use destack_source::{
    Edit as SourceTextEdit, FileContent, FileEdit, FileId, Span, Uri, apply_file_edit,
};

use crate::{FileChange, FileUpdate, FileUpdateKind, Session, SessionError};

/// One source text range in byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    /// Inclusive start byte offset.
    pub start: u32,
    /// Exclusive end byte offset.
    pub end: u32,
}

/// One source text replacement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Replaced byte range.
    pub range: TextRange,
    /// Replacement text.
    pub text: String,
}

/// One source edit accepted by a session update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceEdit {
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
        /// Source repository or session relative path.
        from: PathBuf,
        /// Destination repository or session relative path.
        to: PathBuf,
    },
}

/// One source update applied through a session ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUpdate {
    /// Expected base revision when the caller wants compare and swap.
    pub base: Option<Revision>,
    /// Source edits in this atomic update.
    pub edits: Vec<SourceEdit>,
}

/// Result of one source update.
#[derive(Debug, Clone)]
pub struct SourceUpdateResult {
    /// Previous revision.
    pub before: Revision,
    /// Updated revision.
    pub after: Revision,
    /// Changed files.
    pub files: Vec<FileUpdate>,
}

impl Session {
    /// Apply one source update through one ref.
    pub fn update(
        &self,
        reference: &Ref,
        update: SourceUpdate,
    ) -> Result<SourceUpdateResult, SessionError> {
        let repository = self.repository();
        let before = self.revision(reference)?;

        // reject stale compare-and-swap bases
        if let Some(base) = update.base {
            if base != before {
                return Err(SessionError::StaleRevision {
                    reference: reference.clone(),
                    expected: base,
                    current: before,
                });
            }
        }

        // build a repository change before pinning outputs
        let (change, file_ids) = self.change_for_source_update(before, update.edits)?;
        let before_pin = repository.pin(before)?;
        let revision = repository.commit_change(before, change)?;
        let revision_pin = repository.pin(revision)?;

        // publish when the ref still points at the edited base
        let was_published = repository.advance_ref(reference, before, revision)?;
        if !was_published {
            return Err(SessionError::StaleRevision {
                reference: reference.clone(),
                expected: before,
                current: self.revision(reference)?,
            });
        }

        let files =
            self.project_file_updates(before_pin.revision(), revision_pin.revision(), file_ids)?;

        Ok(SourceUpdateResult {
            before: before_pin.revision(),
            after: revision_pin.revision(),
            files,
        })
    }

    /// Apply one explicit file change through one ref.
    pub fn apply_file(
        &self,
        reference: &Ref,
        path: &Path,
        update: FileChange,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let update = SourceUpdate {
            base: None,
            edits: vec![self.source_edit_for_file(path, update)],
        };
        let result = self.update(reference, update)?;

        Ok(result.files)
    }

    /// Build one source edit from one file change.
    fn source_edit_for_file(&self, path: &Path, update: FileChange) -> SourceEdit {
        let path = path.to_path_buf();

        match update {
            FileChange::Text { content } => SourceEdit::SetText {
                path,
                text: content,
            },
            FileChange::Bytes { content } => SourceEdit::SetBytes {
                path,
                bytes: content,
            },
            FileChange::Removed => SourceEdit::Remove { path },
        }
    }

    /// Build repository edits from source edits.
    fn change_for_source_update(
        &self,
        revision: Revision,
        edits: Vec<SourceEdit>,
    ) -> Result<(RepositoryChange, Vec<FileId>), SessionError> {
        let mut change = RepositoryChange::new();
        let mut file_ids = Vec::new();
        let mut seen_file_ids = HashSet::new();

        // lower each source edit into repository edits
        for edit in edits {
            self.push_source_edit(
                revision,
                edit,
                &mut change,
                &mut file_ids,
                &mut seen_file_ids,
            )?;
        }

        Ok((change, file_ids))
    }

    /// Add one source edit to one repository change.
    fn push_source_edit(
        &self,
        revision: Revision,
        edit: SourceEdit,
        change: &mut RepositoryChange,
        file_ids: &mut Vec<FileId>,
        seen_file_ids: &mut HashSet<FileId>,
    ) -> Result<(), SessionError> {
        match edit {
            SourceEdit::SetText { path, text } => {
                let path = self.repository_path(&path);
                self.push_affected_file(&path, file_ids, seen_file_ids);
                change.push(RepositoryEdit::set_text(path, text));
            }
            SourceEdit::EditText { path, edits } => {
                let path = self.repository_path(&path);
                self.push_affected_file(&path, file_ids, seen_file_ids);
                let text = self.apply_text_edits(revision, &path, edits)?;
                change.push(RepositoryEdit::set_text(path, text));
            }
            SourceEdit::SetBytes { path, bytes } => {
                let path = self.repository_path(&path);
                self.push_affected_file(&path, file_ids, seen_file_ids);
                change.push(RepositoryEdit::SetFile {
                    logical_path: path,
                    content: FileContent::Binary { content: bytes },
                });
            }
            SourceEdit::Remove { path } => {
                let path = self.repository_path(&path);
                self.push_affected_file(&path, file_ids, seen_file_ids);
                change.push(RepositoryEdit::remove_file(path));
            }
            SourceEdit::Move { from, to } => {
                let from = self.repository_path(&from);
                let to = self.repository_path(&to);
                self.push_affected_file(&from, file_ids, seen_file_ids);
                self.push_affected_file(&to, file_ids, seen_file_ids);
                change.push(RepositoryEdit::move_file(from, to));
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
                SourceTextEdit::replace(
                    Span::new(file_id, edit.range.start, edit.range.end),
                    edit.text,
                )
            })
            .collect();

        // materialize the updated text
        let file_edit = FileEdit::with_edits(file_id, edits);
        let text = apply_file_edit(file.as_ref(), &file_edit)?;

        Ok(text)
    }

    /// Add one affected file id.
    fn push_affected_file(
        &self,
        path: &str,
        file_ids: &mut Vec<FileId>,
        seen_file_ids: &mut HashSet<FileId>,
    ) {
        let file_id = FileId::from_logical_str(path);
        if seen_file_ids.insert(file_id) {
            file_ids.push(file_id);
        }
    }

    /// Project repository file changes into session file updates.
    pub(crate) fn project_file_updates(
        &self,
        before: Revision,
        revision: Revision,
        file_changes: Vec<FileId>,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let mut files = Vec::new();
        let repository = self.repository();

        // project changed repository files into session update payloads
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
                let kind = FileUpdateKind::for_path(path);

                let uri = Uri::from_file_path(path);

                files.push(FileUpdate::Updated {
                    module_id,
                    file_id,
                    uri,
                    file,
                    kind,
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
            let kind = FileUpdateKind::for_path(path);

            // removed files still need a uri so clients can clear diagnostics
            let uri = Uri::from_file_path(path);

            files.push(FileUpdate::Removed {
                module_id,
                file_id,
                uri,
                kind,
            });
        }

        Ok(files)
    }
}
