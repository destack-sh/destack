use std::sync::Arc;

use dashmap::mapref::entry::Entry;
use destack_artifact::SourceDependency;
use destack_core::TreapRoot;
use destack_source::{Content, FileId};

use crate::repository::{
    Change, Commit, Delta, Discovery, FileEntry, Ref, Repository, RepositoryError, Revision,
    RevisionEntry, RevisionState, normalize_logical_path,
};

/// One atomic mutation inside one repository edit batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Add one file with one full content payload.
    AddFile {
        /// The workspace logical path.
        logical_path: String,
        content: Content,
    },
    /// Set one file with one full content payload.
    SetFile {
        /// The workspace logical path.
        logical_path: String,
        content: Content,
    },
    /// Remove one file from the revision file bindings.
    RemoveFile {
        /// The workspace logical path.
        logical_path: String,
    },
    /// Move one file within the revision file bindings.
    MoveFile {
        /// The source workspace logical path.
        from: String,
        /// The destination workspace logical path.
        to: String,
    },
}

impl Edit {
    /// Build one text add edit.
    pub fn add_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::AddFile {
            logical_path: normalize_logical_path(path),
            content: Content::Text {
                content: content.into(),
            },
        }
    }

    /// Build one text set edit.
    pub fn set_text(path: impl AsRef<str>, content: impl Into<String>) -> Self {
        Self::SetFile {
            logical_path: normalize_logical_path(path),
            content: Content::Text {
                content: content.into(),
            },
        }
    }

    /// Build one remove edit.
    pub fn remove_file(path: impl AsRef<str>) -> Self {
        Self::RemoveFile {
            logical_path: normalize_logical_path(path),
        }
    }

    /// Build one move edit.
    pub fn move_file(from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self::MoveFile {
            from: normalize_logical_path(from),
            to: normalize_logical_path(to),
        }
    }

    /// Return concrete file ids changed by this edit.
    pub(crate) fn changed_file_ids(&self) -> impl Iterator<Item = FileId> + '_ {
        let (first, second) = self.changed_logical_paths();
        let first = FileId::from_logical_str(first);
        let second = second.map(FileId::from_logical_str);

        [Some(first), second].into_iter().flatten()
    }

    /// Return logical file paths changed by this edit.
    pub(crate) fn changed_logical_paths(&self) -> (&str, Option<&str>) {
        match self {
            Self::AddFile { logical_path, .. }
            | Self::SetFile { logical_path, .. }
            | Self::RemoveFile { logical_path } => (logical_path, None),
            Self::MoveFile { from, to } => (from, Some(to)),
        }
    }
}

impl Repository {
    /// Apply edits to one immutable revision.
    pub fn edit<I>(&self, before: Revision, edits: I) -> Result<Commit, RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        let before_state = self.revision(before)?;
        let before_files = before_state.files();
        let (after_files, changed_files, mut delta) = self.apply(before, before_files, edits)?;

        // invalidate previous discovery observations only when their values changed
        match delta.discovery() {
            Discovery::None => {}
            Discovery::Paths => {
                let before_packages = self.package_ids(before)?;
                let before_modules = self.module_ids(before)?;
                let after_packages = self.package_index_for_files(before, after_files)?;
                let listing = self.files.entries.entries(after_files);
                let after_modules =
                    self.module_index_for_files(before, &listing, &after_packages)?;
                let mut after_packages = after_packages.package_ids().collect::<Vec<_>>();
                let mut after_modules = after_modules.module_ids().collect::<Vec<_>>();
                after_packages.sort_unstable();
                after_packages.dedup();
                after_modules.sort_unstable();
                after_modules.dedup();

                if before_packages != after_packages {
                    delta.extend([SourceDependency::packages(&before_packages)]);
                }
                if before_modules != after_modules {
                    delta.extend([SourceDependency::modules(&before_modules)]);
                }
            }
            Discovery::Config => {
                let packages = self.package_ids(before)?;
                let modules = self.module_ids(before)?;
                delta.extend([
                    SourceDependency::packages(&packages),
                    SourceDependency::modules(&modules),
                ]);
            }
        }

        // retain sparse artifact bindings and mark only reached observations dirty
        let artifacts = before_state.artifacts.fork(&delta, self.artifact_table())?;
        let after_state = Arc::new(RevisionState::new(
            after_files,
            before_state.environment.clone(),
            artifacts,
        ));
        let after = after_state.revision();

        // retain existing derived state for identical repository revisions
        if after != before {
            match self.revisions.entry(after) {
                Entry::Occupied(_entry) => {}
                Entry::Vacant(entry) => {
                    entry.insert(Arc::new(RevisionEntry::new(after_state)));
                }
            }
        }

        // project canonical changes only from files touched by this edit batch
        let mut changes = changed_files
            .into_iter()
            .filter_map(|file| Change::between(self, before_files, after_files, file))
            .collect::<Vec<_>>();
        changes.sort_unstable_by(|left, right| left.path.cmp(&right.path));

        Ok(Commit {
            before,
            after,
            changes,
        })
    }

    /// Apply edits and atomically advance one repository ref.
    pub fn commit<I>(
        self: &Arc<Self>,
        reference: &Ref,
        before: Revision,
        edits: I,
    ) -> Result<Commit, RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        let current = self.current(reference)?;
        if current != before {
            return Err(RepositoryError::RefChanged {
                reference: reference.clone(),
                expected: before,
                current,
            });
        }

        // retain both revisions until their ref transition completes
        let _before = self.pin(before)?;
        let commit = self.edit(before, edits)?;
        let _after = self.pin(commit.after)?;
        let was_committed = self.advance_ref(reference, before, commit.after)?;
        if !was_committed {
            let current = self.current(reference)?;

            return Err(RepositoryError::RefChanged {
                reference: reference.clone(),
                expected: before,
                current,
            });
        }

        Ok(commit)
    }

    /// Apply edits to one file tree and collect their incremental consequences.
    fn apply<I>(
        &self,
        before: Revision,
        mut files: TreapRoot,
        edits: I,
    ) -> Result<(TreapRoot, Vec<FileId>, Delta), RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        let mut changed_files = Vec::new();
        let mut invalidated = Vec::new();
        let mut discovery = Discovery::None;

        for edit in edits {
            changed_files.extend(edit.changed_file_ids());

            match edit {
                // add one new file binding
                Edit::AddFile {
                    logical_path,
                    content,
                } => {
                    let logical_path = normalize_logical_path(&logical_path);
                    let file = FileId::from_logical_str(&logical_path);
                    if self.files.entries.contains(files, &file) {
                        return Err(RepositoryError::FileAlreadyExists { path: logical_path });
                    }

                    let change = if is_package_config_path(&logical_path) {
                        Discovery::Config
                    } else {
                        Discovery::Paths
                    };
                    discovery = discovery.merge(change);
                    self.observe_module_path(before, file, &mut invalidated)?;

                    let logical_path = self.intern_logical_path(logical_path);
                    let content = self.intern_content(content)?;
                    files = self.files.entries.insert(
                        files,
                        file,
                        FileEntry::loaded(logical_path, content),
                    );
                }

                // replace one file binding
                Edit::SetFile {
                    logical_path,
                    content,
                } => {
                    let logical_path = normalize_logical_path(&logical_path);
                    let file = FileId::from_logical_str(&logical_path);
                    let previous = self.files.entries.get(files, &file);
                    let change = if is_package_config_path(&logical_path) {
                        Discovery::Config
                    } else if previous.is_none() {
                        Discovery::Paths
                    } else {
                        Discovery::None
                    };
                    discovery = discovery.merge(change);

                    if previous.is_none() {
                        self.observe_module_path(before, file, &mut invalidated)?;
                    }
                    if let Some(previous) = previous {
                        invalidated.push(SourceDependency::file_content(file, previous.content_id));
                    }

                    let logical_path = self.intern_logical_path(logical_path);
                    let content = self.intern_content(content)?;
                    files = self.files.entries.insert(
                        files,
                        file,
                        FileEntry::loaded(logical_path, content),
                    );
                }

                // remove one existing file binding
                Edit::RemoveFile { logical_path } => {
                    let logical_path = normalize_logical_path(&logical_path);
                    let file = FileId::from_logical_str(&logical_path);
                    let previous = self.files.entries.get(files, &file).ok_or_else(|| {
                        RepositoryError::MissingFile {
                            path: logical_path.clone(),
                        }
                    })?;
                    let change = if is_package_config_path(&logical_path) {
                        Discovery::Config
                    } else {
                        Discovery::Paths
                    };
                    discovery = discovery.merge(change);
                    invalidated.push(SourceDependency::file_content(file, previous.content_id));
                    self.observe_module_path(before, file, &mut invalidated)?;

                    files = self.files.entries.remove(files, &file);
                }

                // move one existing file binding
                Edit::MoveFile { from, to } => {
                    if from == to {
                        continue;
                    }

                    let from = normalize_logical_path(&from);
                    let to = normalize_logical_path(&to);
                    let change = if is_package_config_path(&from) || is_package_config_path(&to) {
                        Discovery::Config
                    } else {
                        Discovery::Paths
                    };
                    let source = FileId::from_logical_str(&from);
                    let source_entry = self
                        .files
                        .entries
                        .get(files, &source)
                        .ok_or(RepositoryError::MissingFile { path: from })?;
                    let destination = FileId::from_logical_str(&to);
                    if self.files.entries.contains(files, &destination) {
                        return Err(RepositoryError::FileAlreadyExists { path: to });
                    }

                    discovery = discovery.merge(change);
                    invalidated.push(SourceDependency::file_content(
                        source,
                        source_entry.content_id,
                    ));
                    self.observe_module_path(before, source, &mut invalidated)?;
                    self.observe_module_path(before, destination, &mut invalidated)?;

                    files = self.files.entries.remove(files, &source);
                    let to = self.intern_logical_path(to);
                    let destination_entry = FileEntry::loaded(to, source_entry.content_id);
                    files = self
                        .files
                        .entries
                        .insert(files, destination, destination_entry);
                }
            }
        }

        changed_files.sort_unstable();
        changed_files.dedup();
        let delta = Delta::new(invalidated, discovery);

        Ok((files, changed_files, delta))
    }

    /// Record the previous module resolution of one probed path.
    fn observe_module_path(
        &self,
        revision: Revision,
        file: FileId,
        invalidated: &mut Vec<SourceDependency>,
    ) -> Result<(), RepositoryError> {
        let module = self.module_id_for_file(revision, file)?;
        invalidated.push(SourceDependency::module_path(file, module));

        Ok(())
    }
}

/// Return whether one logical path names a package config file.
fn is_package_config_path(logical_path: &str) -> bool {
    logical_path.rsplit('/').next() == Some("destack.json")
}
