use std::path::Path;
use std::sync::Arc;

use dashmap::mapref::entry::Entry;
use destack_artifact::SourceDependency;
use destack_core::TreapRoot;
use destack_source::{Content, FileId, FileType};

use crate::repository::{
    Edit, FileEntry, Ref, Repository, RepositoryError, Revision, RevisionEntry, RevisionState,
    SourceDelta, normalize_logical_path,
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
            // reject refs that moved since the caller read them
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
        let (files, mut delta) =
            self.apply_edits(base_revision_id, base_revision.files(), edits)?;
        if delta.is_discovery_changed() {
            let packages = self.package_ids(base_revision_id)?;
            let modules = self.module_ids(base_revision_id)?;
            delta.extend([
                SourceDependency::packages(&packages),
                SourceDependency::modules(&modules),
            ]);
        }
        let artifacts = base_revision
            .artifacts
            .fork(&delta, self.artifact_table())?;
        let revision = Arc::new(RevisionState::new(
            files,
            Arc::clone(&base_revision.environment),
            artifacts,
        ));
        let revision_id = revision.revision();

        // keep no-op edits on the base revision
        if revision_id == base_revision_id {
            return Ok(base_revision_id);
        }

        match self.revisions.entry(revision_id) {
            // retain the existing derived state for identical source states
            Entry::Occupied(_entry) => {}

            // publish one new source state
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

        // read binary payload
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
        // read text payload
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

    /// Record the previous module resolution of one probed path.
    fn observe_module_path(
        &self,
        revision: Revision,
        file: FileId,
        changed_sources: &mut Vec<SourceDependency>,
    ) -> Result<(), RepositoryError> {
        let module = self.module_id_for_file(revision, file)?;
        changed_sources.push(SourceDependency::module_path(file, module));

        Ok(())
    }

    /// Apply edits to one file bindings.
    fn apply_edits<I>(
        &self,
        base_revision: Revision,
        mut files: TreapRoot,
        edits: I,
    ) -> Result<(TreapRoot, SourceDelta), RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        let mut changed_sources = Vec::new();
        let mut is_discovery_changed = false;

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

                    is_discovery_changed = true;
                    self.observe_module_path(base_revision, file_id, &mut changed_sources)?;

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
                    let previous = self.files.entries.get(files, &file_id);
                    let is_existing = previous.is_some();
                    let is_package_config = logical_path.rsplit('/').next() == Some("destack.json");
                    is_discovery_changed |= !is_existing || is_package_config;
                    if !is_existing {
                        self.observe_module_path(base_revision, file_id, &mut changed_sources)?;
                    }
                    if let Some(previous) = previous {
                        changed_sources
                            .push(SourceDependency::file_content(file_id, previous.content_id));
                    }

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
                    let previous = self.files.entries.get(files, &file_id).ok_or_else(|| {
                        RepositoryError::MissingFile {
                            path: logical_path.clone(),
                        }
                    })?;
                    is_discovery_changed = true;
                    changed_sources
                        .push(SourceDependency::file_content(file_id, previous.content_id));
                    self.observe_module_path(base_revision, file_id, &mut changed_sources)?;

                    files = self.files.entries.remove(files, &file_id);
                }

                // move one existing file binding
                Edit::MoveFile { from, to } => {
                    // ignore no-op moves
                    if from == to {
                        continue;
                    }

                    let from = normalize_logical_path(&from);
                    let to = normalize_logical_path(&to);
                    let from_file_id = FileId::from_logical_str(&from);
                    let from_file = self
                        .files
                        .entries
                        .get(files, &from_file_id)
                        .ok_or(RepositoryError::MissingFile { path: from })?;
                    let to_file_id = FileId::from_logical_str(&to);
                    if self.files.entries.contains(files, &to_file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: to });
                    }

                    is_discovery_changed = true;
                    changed_sources.push(SourceDependency::file_content(
                        from_file_id,
                        from_file.content_id,
                    ));
                    self.observe_module_path(base_revision, from_file_id, &mut changed_sources)?;
                    self.observe_module_path(base_revision, to_file_id, &mut changed_sources)?;

                    files = self.files.entries.remove(files, &from_file_id);
                    let to = self.intern_logical_path(to);
                    let to_file = FileEntry::loaded(to, from_file.content_id);

                    files = self.files.entries.insert(files, to_file_id, to_file);
                }
            }
        }

        Ok((
            files,
            SourceDelta::new(changed_sources, is_discovery_changed),
        ))
    }
}
