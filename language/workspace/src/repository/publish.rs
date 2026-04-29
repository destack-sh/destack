use std::path::Path;
use std::sync::Arc;

use destack_source::{FileContent, FileId, FileType};
use im::OrdMap;

use crate::repository::{
    Edit, FileEntry, FileSource, Ref, Repository, RepositoryError, Revision, RevisionState,
    normalize_logical_path_str,
};

impl Repository {
    /// Create one new ref pointing at another ref's current revision.
    pub fn fork_ref(&self, from: &Ref, to: Ref) -> Result<Revision, RepositoryError> {
        let revision = self.current(from)?;
        self.refs.insert(to, revision);
        self.prune_unreachable();

        Ok(revision)
    }

    /// Set one ref to one already published revision.
    pub fn set_ref(
        &self,
        reference: &Ref,
        revision: Revision,
    ) -> Result<Revision, RepositoryError> {
        let _revision = self.revision(revision)?;
        self.refs.insert(reference.clone(), revision);
        self.prune_unreachable();

        Ok(revision)
    }

    /// Apply edits to one ref and publish one new revision.
    pub fn apply_to_ref<I>(&self, reference: &Ref, edits: I) -> Result<Revision, RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        let base_revision_id = self.current(reference)?;
        let revision_id = self.fork_with_edits(base_revision_id, edits)?;
        self.refs.insert(reference.clone(), revision_id);
        self.prune_unreachable();

        Ok(revision_id)
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
        let files = base_revision.files.as_ref().clone();
        let files = self.apply_edits(files, edits)?;
        let revision = Arc::new(RevisionState::new(
            Arc::new(files),
            Arc::clone(&base_revision.host),
        ));
        let revision_id = revision.revision();

        self.revisions
            .entry(revision_id)
            .or_insert_with(|| Arc::clone(&revision));

        Ok(revision_id)
    }

    /// Load one workspace file payload from the attached file system.
    pub fn load_workspace_file_content(&self, path: &Path) -> Result<FileContent, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(path);

        if file_type.is_binary() {
            let content = self
                .fs
                .read(path)
                .map_err(|error| RepositoryError::FileSystem {
                    operation: "read",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

            return Ok(FileContent::Binary { content });
        }

        let content =
            self.fs
                .read_to_string(path)
                .map_err(|error| RepositoryError::FileSystem {
                    operation: "read_to_string",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

        Ok(FileContent::Text { content })
    }

    /// Validate one workspace logical path.
    fn validate_workspace_logical_path(&self, logical_path: &str) -> Result<(), RepositoryError> {
        if logical_path.starts_with("builtin://") {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "builtin source is toolchain provided".to_string(),
            });
        }

        if logical_path.contains("://") {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "non workspace source is not writable through repository edits"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// Return one file id for one validated workspace logical path.
    fn file_id_for_workspace_logical_path(
        &self,
        logical_path: &str,
    ) -> Result<FileId, RepositoryError> {
        self.validate_workspace_logical_path(logical_path)?;
        let logical_path = normalize_logical_path_str(logical_path);
        let source = FileSource::workspace(logical_path);

        Ok(self.file_id_for_source(&source))
    }

    /// Apply edits to one file map.
    fn apply_edits<I>(
        &self,
        mut files: OrdMap<FileId, FileEntry>,
        edits: I,
    ) -> Result<OrdMap<FileId, FileEntry>, RepositoryError>
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
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    if files.contains_key(&file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: logical_path });
                    }

                    let content = self.files.intern(content);
                    files.insert(
                        file_id,
                        FileEntry::loaded(FileSource::workspace(logical_path), content),
                    );
                }

                // set the requested file payload
                Edit::SetFile {
                    logical_path,
                    content,
                } => {
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    let content = self.files.intern(content);
                    files.insert(
                        file_id,
                        FileEntry::loaded(FileSource::workspace(logical_path), content),
                    );
                }

                // remove the requested file payload
                Edit::RemoveFile { logical_path } => {
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    if !files.contains_key(&file_id) {
                        return Err(RepositoryError::MissingFile { path: logical_path });
                    }

                    files.remove(&file_id);
                }

                // move one existing file binding
                Edit::MoveFile { from, to } => {
                    if from == to {
                        continue;
                    }

                    let from_file_id = self.file_id_for_workspace_logical_path(&from)?;
                    if !files.contains_key(&from_file_id) {
                        return Err(RepositoryError::MissingFile { path: from });
                    }
                    let Some(from_file) = files.get(&from_file_id).cloned() else {
                        return Err(RepositoryError::MissingFile { path: from });
                    };
                    let to_file_id = self.file_id_for_workspace_logical_path(&to)?;
                    if files.contains_key(&to_file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: to });
                    }

                    files.remove(&from_file_id);
                    let to_file =
                        FileEntry::loaded(FileSource::workspace(to), from_file.content_id);

                    files.insert(to_file_id, to_file);
                }
            }
        }

        Ok(files)
    }
}
