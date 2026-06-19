use std::path::Path;
use std::sync::Arc;

use dashmap::mapref::entry::Entry;
use destack_core::TreapRoot;
use destack_source::{Content, FileId, FileType};

use crate::repository::{
    Edit, FileEntry, Ref, Repository, RepositoryError, Revision, RevisionEntry, RevisionState,
    normalize_logical_path,
};

impl Repository {
    /// Create one new ref pointing at another ref's current revision.
    pub fn fork_ref(&self, from: &Ref, to: Ref) -> Result<Revision, RepositoryError> {
        let revision = self.current(from)?;
        self.refs.insert(to, revision);

        Ok(revision)
    }

    /// Set one ref to one existing revision identity.
    pub fn set_ref(
        &self,
        reference: &Ref,
        revision: Revision,
    ) -> Result<Revision, RepositoryError> {
        let _revision = self.revision(revision)?;
        self.refs.insert(reference.clone(), revision);

        Ok(revision)
    }

    /// Advance one ref when it still points at the expected revision.
    pub fn advance_ref(
        &self,
        reference: &Ref,
        from: Revision,
        to: Revision,
    ) -> Result<bool, RepositoryError> {
        let did_advance = match self.refs.entry(reference.clone()) {
            // reject stale base revisions
            Entry::Occupied(entry) if *entry.get() != from => false,

            // publish the requested revision
            Entry::Occupied(mut entry) => {
                let _revision = self.revision(to)?;
                entry.insert(to);
                true
            }

            // missing refs are repository state errors
            Entry::Vacant(_entry) => {
                return Err(RepositoryError::MissingRef {
                    reference: reference.clone(),
                });
            }
        };

        Ok(did_advance)
    }

    /// Fork one base revision with edits and publish one anonymous revision.
    pub fn fork_with_edits<I>(
        &self,
        base_revision_id: Revision,
        edits: I,
    ) -> Result<Revision, RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        let base_revision = self.revision(base_revision_id)?;
        let edits = edits.into_iter().collect::<Vec<_>>();
        let files = self.apply_edits(base_revision.files(), edits)?;
        let revision = Arc::new(RevisionState::new(
            files,
            Arc::clone(&base_revision.environment),
            [base_revision_id],
        ));
        let revision_id = revision.revision();

        match self.revisions.entry(revision_id) {
            // merge branch-local bases for identical source states
            Entry::Occupied(entry) => {
                entry.get().state().add_bases([base_revision_id]);
            }

            // publish a new source state
            Entry::Vacant(entry) => {
                entry.insert(Arc::new(RevisionEntry::new(revision)));
            }
        }

        Ok(revision_id)
    }
    /// Load one workspace file payload from the attached file system.
    pub fn load_workspace_file_content(&self, path: &Path) -> Result<Content, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(path);
        let file_system = self.file_system();

        // binary
        if file_type.is_binary() {
            let content = file_system
                .read(path)
                .map_err(|error| RepositoryError::FileSystem {
                    operation: "read",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

            Ok(Content::Binary { content })
        }
        // text
        else {
            let content =
                file_system
                    .read_to_string(path)
                    .map_err(|error| RepositoryError::FileSystem {
                        operation: "read_to_string",
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    })?;

            Ok(Content::Text { content })
        }
    }

    /// Apply edits to one file bindings.
    fn apply_edits<I>(&self, mut files: TreapRoot, edits: I) -> Result<TreapRoot, RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        for edit in edits {
            match edit {
                // write the requested file payload
                Edit::AddFile {
                    logical_path,
                    content,
                } => {
                    let logical_path = normalize_logical_path(&logical_path);
                    let file_id = FileId::from_logical_str(&logical_path);
                    if self.files.entries.contains(files, &file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: logical_path });
                    }

                    let logical_path = self.intern_logical_path(logical_path);
                    let content = self.intern_content(content)?;
                    files = self.files.entries.insert(
                        files,
                        file_id,
                        FileEntry::loaded(logical_path, content),
                    );
                }

                // set the requested file payload
                Edit::SetFile {
                    logical_path,
                    content,
                } => {
                    let logical_path = normalize_logical_path(&logical_path);
                    let file_id = FileId::from_logical_str(&logical_path);
                    let logical_path = self.intern_logical_path(logical_path);
                    let content = self.intern_content(content)?;
                    files = self.files.entries.insert(
                        files,
                        file_id,
                        FileEntry::loaded(logical_path, content),
                    );
                }

                // remove the requested file payload
                Edit::RemoveFile { logical_path } => {
                    let logical_path = normalize_logical_path(&logical_path);
                    let file_id = FileId::from_logical_str(&logical_path);
                    if !self.files.entries.contains(files, &file_id) {
                        return Err(RepositoryError::MissingFile { path: logical_path });
                    }

                    files = self.files.entries.remove(files, &file_id);
                }

                // move one existing file binding
                Edit::MoveFile { from, to } => {
                    if from == to {
                        continue;
                    }

                    let from = normalize_logical_path(&from);
                    let to = normalize_logical_path(&to);
                    let from_file_id = FileId::from_logical_str(&from);
                    if !self.files.entries.contains(files, &from_file_id) {
                        return Err(RepositoryError::MissingFile { path: from });
                    }
                    let Some(from_file) = self.files.entries.get(files, &from_file_id) else {
                        return Err(RepositoryError::MissingFile { path: from });
                    };
                    let to_file_id = FileId::from_logical_str(&to);
                    if self.files.entries.contains(files, &to_file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: to });
                    }

                    files = self.files.entries.remove(files, &from_file_id);
                    let to = self.intern_logical_path(to);
                    let to_file = FileEntry::loaded(to, from_file.content_id);

                    files = self.files.entries.insert(files, to_file_id, to_file);
                }
            }
        }

        Ok(files)
    }
}
