use std::collections::HashSet;

use destack_repository::{Ref, Revision, RevisionPin};
use destack_source::{
    Content, Edit, FileId, FilePatch, Patch, Span, TextPatch, Uri, apply_file_patch,
};

use crate::{Change, Session, SessionError};

/// One complete source commit awaiting publication.
#[derive(Debug)]
pub struct PreparedCommit<'session> {
    /// The session publishing the commit.
    session: &'session Session,
    /// The ref advanced by publication.
    reference: Ref,
    /// The pinned base revision.
    before: RevisionPin,
    /// The pinned edited revision.
    after: RevisionPin,
    /// The changed files.
    changes: Vec<Change>,
}

impl PreparedCommit<'_> {
    /// Publish the edited revision through its session ref.
    pub fn publish(self) -> Result<Commit, SessionError> {
        let Self {
            session,
            reference,
            before,
            after,
            changes,
        } = self;
        let before = before.revision();
        let after = after.revision();
        session.publish_revision(&reference, before, after)?;

        Ok(Commit {
            before,
            after,
            changes,
        })
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
    /// Patch files through one ref.
    pub fn edit(&self, reference: &Ref, edits: Vec<Edit>) -> Result<Commit, SessionError> {
        let before = self.revision(reference)?;

        self.prepare_edit(reference, before, edits)?.publish()
    }

    /// Patch files when one ref still points at one revision.
    pub fn edit_if_current(
        &self,
        reference: &Ref,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, SessionError> {
        self.prepare_edit(reference, revision, edits)?.publish()
    }

    /// Prepare source edits when one ref still points at one revision.
    pub fn prepare_edit(
        &self,
        reference: &Ref,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<PreparedCommit<'_>, SessionError> {
        let current = self.revision(reference)?;
        if current != revision {
            return Err(SessionError::StaleRevision {
                reference: reference.clone(),
                expected: revision,
                current,
            });
        }

        let repository = self.repository();

        // build repository edits before pinning outputs
        let edits = self.repository_edits_for_edits(revision, edits)?;
        let file_ids = repository_file_ids(&edits);
        let before = repository.pin(revision)?;
        let after = repository.commit_edits(revision, edits)?;
        let after = repository.pin(after)?;

        // project changes while both immutable revisions are pinned
        let changes = self.project_file_changes(before.revision(), after.revision(), file_ids)?;

        Ok(PreparedCommit {
            session: self,
            reference: reference.clone(),
            before,
            after,
            changes,
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
            Edit::EditText { path, patches } => {
                let path = self.repository_path(&path);
                let text = self.apply_text_patches(revision, &path, patches)?;
                repository_edits.push(destack_repository::Edit::set_text(path, text));
            }
            Edit::SetBytes { path, bytes } => {
                let path = self.repository_path(&path);
                repository_edits.push(destack_repository::Edit::SetFile {
                    logical_path: path,
                    content: Content::Binary { content: bytes },
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

    /// Apply text patches to one tracked file.
    fn apply_text_patches(
        &self,
        revision: Revision,
        path: &str,
        patches: Vec<TextPatch>,
    ) -> Result<String, SessionError> {
        let file_id = FileId::from_logical_str(path);

        // read the current tracked text
        let file = self
            .repository()
            .file(revision, file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        // lower file text patches into source patches
        let patches = patches
            .into_iter()
            .map(|patch| {
                Patch::replace(
                    Span::new(file_id, patch.range.start, patch.range.end),
                    patch.text,
                )
            })
            .collect();

        // materialize the updated text
        let file_patch = FilePatch::with_patches(file_id, patches);
        let text = apply_file_patch(file.as_ref(), &file_patch)?;

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
        for file_id in edit.changed_file_ids() {
            if seen_file_ids.insert(file_id) {
                file_ids.push(file_id);
            }
        }
    }

    file_ids
}
