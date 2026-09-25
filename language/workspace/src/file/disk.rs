use std::collections::HashSet;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use tspp_core::{Blob, BlobId, StableHasher};
use tspp_repository::{Change, Commit, Revision};
use tspp_source::Edit;

use crate::workspace::{State, Workspace};
use crate::{Error, FileSelection};

impl Workspace {
    /// Reload physical workspace state from its complete host filesystem.
    pub fn reload(&self) -> Result<Option<Commit>, Error> {
        let mut state = self.lock()?;
        let before = state.physical.revision();
        let edits = self.repository.scan(&self.root, before)?;
        let commit = self.repository.edit(before, edits)?;

        if commit.before == commit.after {
            return Ok(None);
        }
        self.publish_physical(&mut state, &commit)?;

        Ok(Some(commit))
    }

    /// Reconcile changed physical paths into the workspace.
    pub fn reconcile(&self, paths: Vec<PathBuf>) -> Result<Option<Commit>, Error> {
        let mut state = self.lock()?;
        let logical_paths = paths
            .into_iter()
            .map(|path| self.logical_path(&path).map(PathBuf::from))
            .collect::<Result<Vec<_>, _>>()?;
        let before = state.physical.revision();
        let edits = self
            .repository
            .scan_paths(&self.root, before, logical_paths)?;
        let commit = self.repository.edit(before, edits)?;

        if commit.before == commit.after {
            return Ok(None);
        }
        self.publish_physical(&mut state, &commit)?;

        Ok(Some(commit))
    }

    /// Commit source edits to disk and physical workspace state.
    pub fn edit(&self, revision: Revision, edits: Vec<Edit>) -> Result<Commit, Error> {
        let mut state = self.lock()?;
        let current = state.physical.revision();
        if current != revision {
            return Err(Error::StaleRevision {
                expected: revision,
                current,
            });
        }

        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(edit))
            .collect::<Result<Vec<_>, _>>()?;
        let edits = edits
            .into_iter()
            .map(|edit| self.lower(revision, edit))
            .collect::<Result<Vec<_>, _>>()?;
        let commit = self.repository.edit(revision, edits)?;

        self.persist(&mut state, commit)
    }

    /// Save selected files from one branch to physical workspace state.
    pub fn save_branch(
        &self,
        name: &str,
        revision: Revision,
        physical: Revision,
        files: FileSelection,
    ) -> Result<Commit, Error> {
        let mut state = self.lock()?;
        let current_physical = state.physical.revision();
        if current_physical != physical {
            return Err(Error::StaleRevision {
                expected: physical,
                current: current_physical,
            });
        }

        let current_branch = state.branch(name)?.revision();
        if current_branch != revision {
            return Err(Error::StaleRevision {
                expected: revision,
                current: current_branch,
            });
        }

        let changes = self.changes(physical, revision, files)?;
        let edits = changes.iter().map(Change::forward);
        let commit = self.repository.edit(physical, edits)?;

        self.persist(&mut state, commit)
    }

    /// Restore selected branch files from physical workspace state.
    pub fn restore_branch(
        &self,
        name: &str,
        revision: Revision,
        physical: Revision,
        files: FileSelection,
    ) -> Result<Commit, Error> {
        let mut state = self.lock()?;
        let current_branch = state.branch(name)?.revision();
        if current_branch != revision {
            return Err(Error::StaleRevision {
                expected: revision,
                current: current_branch,
            });
        }

        let current_physical = state.physical.revision();
        if current_physical != physical {
            return Err(Error::StaleRevision {
                expected: physical,
                current: current_physical,
            });
        }

        let changes = self.changes(revision, physical, files)?;
        let edits = changes.iter().map(Change::forward);
        let commit = self.repository.edit(revision, edits)?;
        let after = self.repository.pin(commit.after)?;
        state.branches.insert(name.to_string(), after.clone());
        self.publish(Some(name), &commit, after);

        Ok(commit)
    }

    /// Select canonical changes between two exact revisions.
    fn changes(
        &self,
        before: Revision,
        after: Revision,
        files: FileSelection,
    ) -> Result<Vec<Change>, Error> {
        let mut changes = self.repository.changes(before, after)?;
        let FileSelection::Paths(paths) = files else {
            return Ok(changes);
        };

        let paths = paths
            .into_iter()
            .map(|path| self.logical_path(&self.resolve_path(&path)?))
            .collect::<Result<HashSet<_>, Error>>()?;
        changes.retain(|change| paths.contains(&change.path));

        Ok(changes)
    }

    /// Write canonical changes before advancing physical workspace state.
    fn persist(&self, state: &mut State, commit: Commit) -> Result<Commit, Error> {
        // retain the unpublished revision throughout physical writes
        let _after = self.repository.pin(commit.after)?;

        // select files that have not already reached the requested state
        let mut writes = Vec::new();
        for change in &commit.changes {
            if self.requires_write(change)? {
                writes.push(change);
            }
        }

        // write every changed file while Workspace mutations remain serialized
        for (index, change) in writes.iter().enumerate() {
            if let Err(error) = self.apply(change) {
                return Err(self.rollback(error, &writes[..index]));
            }
        }

        // publish repository state only after every physical write succeeds
        if let Err(error) = self.publish_physical(state, &commit) {
            return Err(self.rollback(error, &writes));
        }

        Ok(commit)
    }

    /// Advance physical workspace state and publish its exact transition.
    fn publish_physical(&self, state: &mut State, commit: &Commit) -> Result<(), Error> {
        let after = self.repository.pin(commit.after)?;
        state.physical = after.clone();
        self.publish(None, commit, after);

        Ok(())
    }

    /// Restore original Blob bindings after a failed operation.
    fn rollback(&self, operation: Error, changes: &[&Change]) -> Error {
        let mut failures = Vec::new();

        // restore every path even when one restoration fails
        for change in changes.iter().rev() {
            let path = self.repository.physical_path(&change.path);
            if let Err(error) = self.replace(&path, change.before) {
                failures.push(error);
            }
        }

        if failures.is_empty() {
            operation
        } else {
            Error::RollbackFailed {
                operation: Box::new(operation),
                failures,
            }
        }
    }

    /// Return whether one physical file still requires its repository change.
    fn requires_write(&self, change: &Change) -> Result<bool, Error> {
        let path = self.repository.physical_path(&change.path);
        let actual = self.physical_blob(&path)?;
        if actual == change.before {
            Ok(true)
        } else if actual == change.after {
            Ok(false)
        } else {
            Err(Error::FileChanged {
                path: path.into_boxed_path(),
                expected: change.before,
                actual,
            })
        }
    }

    /// Identify one physical file without retaining its bytes.
    fn physical_blob(&self, path: &Path) -> Result<Option<Blob>, Error> {
        let mut input = match self.repository.file_system().open(path) {
            Ok(input) => input,
            Err(source) if source.kind() == ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(Error::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };
        let mut hasher = StableHasher::new();
        let byte_len = io::copy(&mut input, &mut hasher).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let blob = Blob::new(BlobId::new(hasher.finish_bytes()), byte_len);

        Ok(Some(blob))
    }

    /// Apply one canonical repository change to disk.
    fn apply(&self, change: &Change) -> Result<(), Error> {
        let path = self.repository.physical_path(&change.path);

        self.replace(&path, change.after)
    }

    /// Replace one physical file with one optional Blob binding.
    fn replace(&self, path: &Path, blob: Option<Blob>) -> Result<(), Error> {
        let Some(blob) = blob else {
            if let Err(source) = self.repository.file_system().remove_file(path)
                && source.kind() != ErrorKind::NotFound
            {
                return Err(Error::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }

            return Ok(());
        };

        self.create_parent(path)?;
        let memory = self.repository.open_blob(blob)?;
        self.repository
            .file_system()
            .write(path, memory.bytes())
            .map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })
    }

    /// Create the parent directory for one physical file path.
    fn create_parent(&self, path: &Path) -> Result<(), Error> {
        let Some(parent) = path.parent() else {
            return Ok(());
        };

        self.repository
            .file_system()
            .create_dir_all(parent)
            .map_err(|source| Error::Io {
                path: parent.to_path_buf(),
                source,
            })
    }
}
