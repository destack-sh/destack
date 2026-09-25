use std::sync::Arc;

use dashmap::mapref::entry::Entry;
use tspp_artifact::{ArtifactDependency, SourceDependency};
use tspp_core::{Blob, TreapRoot};
use tspp_source::FileId;

use crate::repository::{
    Change, Commit, Delta, Discovery, FileEntry, Repository, RepositoryError, Revision,
    RevisionEntry, RevisionState, normalize_logical_path,
};

/// One atomic mutation inside one repository edit batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// Add one file Blob.
    AddFile {
        /// The workspace logical path.
        logical_path: String,
        /// The exact file Blob.
        blob: Blob,
    },
    /// Set one file Blob.
    SetFile {
        /// The workspace logical path.
        logical_path: String,
        /// The exact file Blob.
        blob: Blob,
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
    /// Build one file add edit.
    pub fn add_file(path: impl AsRef<str>, blob: Blob) -> Self {
        Self::AddFile {
            logical_path: normalize_logical_path(path),
            blob,
        }
    }

    /// Build one file set edit.
    pub fn set_file(path: impl AsRef<str>, blob: Blob) -> Self {
        Self::SetFile {
            logical_path: normalize_logical_path(path),
            blob,
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
}

impl Repository {
    /// Apply edits to one immutable revision.
    pub fn edit<I>(&self, before: Revision, edits: I) -> Result<Commit, RepositoryError>
    where
        I: IntoIterator<Item = Edit>,
    {
        let before_state = self.revision(before)?;
        let before_files = before_state.files();
        let (after_files, changed_files, mut delta) = self.apply(before_files, edits)?;

        // retain the exact revision when every file binding is unchanged
        if after_files == before_files {
            return Ok(Commit {
                before,
                after: before,
                changes: Vec::new(),
            });
        }

        // retain one artifact selection while computing its invalidation
        let selection = before_state.artifacts.read();

        // classify referenced configuration changes using the evaluated package index
        let packages = match before_state.cache.packages.get() {
            Some(Ok(packages)) => Some(packages),
            Some(Err(_)) | None => None,
        };
        let discovery = if packages.is_some_and(|packages| {
            changed_files
                .iter()
                .any(|file| packages.is_configuration_file(*file))
        }) {
            Discovery::Config
        } else {
            delta.discovery()
        };

        // retain discovery results while their inputs remain unchanged
        let (mut package_result, mut module_result) = match (discovery, packages) {
            (Discovery::None, Some(packages)) => (
                Some(Ok(packages.clone())),
                before_state.cache.modules.get().cloned(),
            ),
            _ => (None, None),
        };

        // invalidate recorded discovery observations
        if discovery != Discovery::None || packages.is_none() {
            let mut observations = selection
                .dependencies()
                .filter_map(|dependency| match dependency {
                    ArtifactDependency::Source(source)
                        if !matches!(source, SourceDependency::File { .. }) =>
                    {
                        Some(*source)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            observations.sort_unstable();
            observations.dedup();

            // compare file discovery when the preceding package configuration is available
            if let (Discovery::Paths, Some(packages)) = (discovery, packages) {
                let before_packages = packages.package_ids().collect::<Vec<_>>();
                let after_packages = self
                    .package_index_for_files(before, after_files)
                    .map(Arc::new);
                let listing = self.file_tree.entries(after_files);
                let after_modules = match &after_packages {
                    Ok(packages) => self
                        .module_index_for_files(before, &listing, packages)
                        .map(Arc::new),
                    Err(error) => Err(error.clone()),
                };

                // compare successful discovery results and retain failures for subsequent reads
                if let (Ok(after_packages), Ok(after_modules)) = (&after_packages, &after_modules) {
                    let after_package_ids = after_packages.package_ids().collect::<Vec<_>>();
                    let module_ids = after_modules.module_ids().collect::<Vec<_>>();
                    let package_set = SourceDependency::packages(&after_package_ids);
                    let module_set = SourceDependency::modules(&module_ids);

                    // retain only observations affected by the new file listing
                    observations.retain(|source| match *source {
                        SourceDependency::File { .. } => false,
                        SourceDependency::Package { .. } => before_packages != after_package_ids,
                        SourceDependency::Packages { .. } => *source != package_set,
                        SourceDependency::Module { module, .. } => {
                            *source != after_modules.dependency(module)
                        }
                        SourceDependency::Modules { .. } => *source != module_set,
                        SourceDependency::PackageModules { package, .. } => {
                            *source
                                != SourceDependency::package_modules(
                                    package,
                                    after_modules.package_module_ids(package),
                                )
                        }
                        SourceDependency::ModulePath { file, .. } => {
                            changed_files.binary_search(&file).is_ok()
                        }
                    });
                }

                package_result = Some(after_packages);
                module_result = Some(after_modules);
            }

            delta.extend(observations);
        }

        // fork candidates and mark only observations reached by this edit
        let artifacts = selection.fork(delta.invalidated(), self.artifact_table());
        drop(selection);
        let after_state = Arc::new(RevisionState::new(
            after_files,
            before_state.environment.clone(),
            artifacts,
        ));

        let after = after_state.revision();

        // retain existing derived state for identical repository revisions
        let after_state = match self.revisions.entry(after) {
            Entry::Occupied(entry) => {
                let existing = entry.get().state();
                let learned = after_state.artifacts.read();
                existing.artifacts.write().adopt(&learned)?;

                existing
            }
            Entry::Vacant(entry) => {
                entry.insert(Arc::new(RevisionEntry::new(after_state.clone())));

                after_state
            }
        };

        // retain discovery results evaluated for the resulting file tree
        if let Some(result) = package_result {
            after_state.cache.packages.get_or_init(|| result);
        }
        if let Some(result) = module_result {
            after_state.cache.modules.get_or_init(|| result);
        }

        // project canonical changes only from files touched by this edit batch
        let mut changes = changed_files
            .into_iter()
            .filter_map(|file| {
                let before = self.file_tree.get(before_files, &file);
                let after = self.file_tree.get(after_files, &file);

                Change::between(self, file, before, after)
            })
            .collect::<Vec<_>>();
        changes.sort_unstable_by(|left, right| left.path.cmp(&right.path));

        Ok(Commit {
            before,
            after,
            changes,
        })
    }

    /// Apply edits to one file tree and collect their incremental consequences.
    fn apply<I>(
        &self,
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
            match edit {
                // add one new file binding
                Edit::AddFile { logical_path, blob } => {
                    self.require_blob(blob)?;

                    let logical_path = normalize_logical_path(&logical_path);
                    let file = FileId::from_logical_str(&logical_path);
                    if self.file_tree.contains(files, &file) {
                        return Err(RepositoryError::FileAlreadyExists { path: logical_path });
                    }

                    let change = if is_package_config_path(&logical_path) {
                        Discovery::Config
                    } else {
                        Discovery::Paths
                    };
                    discovery = discovery.merge(change);
                    changed_files.push(file);

                    let logical_path = self.intern_logical_path(logical_path);
                    files = self
                        .file_tree
                        .insert(files, file, FileEntry::new(logical_path, blob));
                }

                // replace one file binding
                Edit::SetFile { logical_path, blob } => {
                    self.require_blob(blob)?;

                    let logical_path = normalize_logical_path(&logical_path);
                    let file = FileId::from_logical_str(&logical_path);
                    let previous = self.file_tree.get(files, &file);

                    // skip the unchanged binding
                    if previous.is_some_and(|previous| previous.blob == blob) {
                        continue;
                    }
                    let change = if is_package_config_path(&logical_path) {
                        Discovery::Config
                    } else if previous.is_none() {
                        Discovery::Paths
                    } else {
                        Discovery::None
                    };
                    discovery = discovery.merge(change);

                    if let Some(previous) = previous {
                        invalidated.push(SourceDependency::file(file, previous.blob.id));
                    }
                    changed_files.push(file);

                    let logical_path = self.intern_logical_path(logical_path);
                    files = self
                        .file_tree
                        .insert(files, file, FileEntry::new(logical_path, blob));
                }

                // remove one existing file binding
                Edit::RemoveFile { logical_path } => {
                    let logical_path = normalize_logical_path(&logical_path);
                    let file = FileId::from_logical_str(&logical_path);
                    let previous = self.file_tree.get(files, &file).ok_or_else(|| {
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
                    invalidated.push(SourceDependency::file(file, previous.blob.id));
                    changed_files.push(file);

                    files = self.file_tree.remove(files, &file);
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
                        .file_tree
                        .get(files, &source)
                        .ok_or(RepositoryError::MissingFile { path: from })?;
                    let destination = FileId::from_logical_str(&to);
                    if self.file_tree.contains(files, &destination) {
                        return Err(RepositoryError::FileAlreadyExists { path: to });
                    }

                    discovery = discovery.merge(change);
                    invalidated.push(SourceDependency::file(source, source_entry.blob.id));
                    changed_files.extend([source, destination]);

                    files = self.file_tree.remove(files, &source);
                    let to = self.intern_logical_path(to);
                    let destination_entry = FileEntry::new(to, source_entry.blob);
                    files = self.file_tree.insert(files, destination, destination_entry);
                }
            }
        }

        changed_files.sort_unstable();
        changed_files.dedup();

        let delta = Delta::new(invalidated, discovery);

        Ok((files, changed_files, delta))
    }
}

/// Return whether one logical path names a package configuration file.
fn is_package_config_path(logical_path: &str) -> bool {
    logical_path.rsplit('/').next() == Some("destack.json")
}
