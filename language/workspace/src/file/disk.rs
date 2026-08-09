use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use destack_repository::{Commit, Revision};
use destack_source::Edit;

use crate::Error;
use crate::workspace::Workspace;

/// Previous filesystem content retained until one commit publishes.
struct Backup {
    /// The source path.
    path: PathBuf,
    /// The previous bytes, or `None` when the path did not exist.
    content: Option<Vec<u8>>,
}

impl Workspace {
    /// Reload this workspace from its host filesystem.
    pub fn reload(&self) -> Result<Option<Commit>, Error> {
        let _write = self.write()?;
        let before = self.revision()?;
        let edits = self.repository.scan(&self.root, before)?;
        let commit = self.advance(before, edits)?;

        if commit.before == commit.after {
            return Ok(None);
        }

        Ok(Some(self.publish(commit)?))
    }

    /// Reconcile changed host paths through this workspace.
    pub fn reconcile(&self, paths: Vec<PathBuf>) -> Result<Option<Commit>, Error> {
        let _write = self.write()?;

        // resolve physical paths against this workspace
        let logical_paths = paths
            .into_iter()
            .map(|path| self.logical_path(&path).map(PathBuf::from))
            .collect::<Result<Vec<_>, _>>()?;
        let before = self.revision()?;
        let edits = self
            .repository
            .scan_paths(&self.root, before, logical_paths)?;
        let commit = self.advance(before, edits)?;

        // publish only an observable repository transition
        if commit.before == commit.after {
            return Ok(None);
        }

        Ok(Some(self.publish(commit)?))
    }

    /// Write one edit to the host filesystem and workspace.
    pub fn write_file(&self, edit: Edit) -> Result<Commit, Error> {
        let _write = self.write()?;
        let edit = self.resolve_edit(edit)?;
        let revision = self.revision()?;

        self.persist(revision, vec![edit])
    }

    /// Write source edits when the current revision still matches.
    pub(crate) fn write_source_edits_if_current(
        &self,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, Error> {
        let _write = self.write()?;
        let edits = edits
            .into_iter()
            .map(|edit| self.resolve_edit(edit))
            .collect::<Result<Vec<_>, _>>()?;

        self.persist(revision, edits)
    }

    /// Commit source edits to disk and one exact workspace revision.
    fn persist(&self, revision: Revision, edits: Vec<Edit>) -> Result<Commit, Error> {
        let backups = self.backups(&edits)?;

        // write every file while the workspace remains locked
        for edit in &edits {
            if let Err(error) = self.write_edit(edit) {
                return Err(self.restore(error, &backups));
            }
        }

        // publish only after every filesystem write succeeds
        match self.commit(revision, edits) {
            Ok(commit) => self.publish(commit),
            Err(error) => Err(self.restore(error, &backups)),
        }
    }

    /// Retain original filesystem bytes for every edited path.
    fn backups(&self, edits: &[Edit]) -> Result<Vec<Backup>, Error> {
        let mut paths = HashSet::new();
        let mut backups = Vec::new();

        // read every distinct path before performing any write
        for edit in edits {
            match edit {
                Edit::SetText { path, .. }
                | Edit::SetBytes { path, .. }
                | Edit::Remove { path } => self.backup(path, &mut paths, &mut backups)?,
                Edit::Move { from, to } => {
                    if from == to {
                        return Err(Error::InvalidEdit {
                            detail: "move source and destination must differ".to_string(),
                        });
                    }

                    self.backup(from, &mut paths, &mut backups)?;
                    self.backup(to, &mut paths, &mut backups)?;
                }
                Edit::EditText { .. } => {
                    return Err(Error::InvalidEdit {
                        detail: "text patch edits cannot be written directly to disk".to_string(),
                    });
                }
            }
        }

        Ok(backups)
    }

    /// Retain one original filesystem path once.
    fn backup(
        &self,
        path: &Path,
        paths: &mut HashSet<PathBuf>,
        backups: &mut Vec<Backup>,
    ) -> Result<(), Error> {
        // reject disk writes while editor content owns the path
        if self.has_open_file(path) {
            return Err(Error::OpenFileWrite {
                path: path.to_path_buf(),
            });
        }

        // retain each physical path once
        if !paths.insert(path.to_path_buf()) {
            return Ok(());
        }

        // retain the current bytes or absence
        let file_system = self.repository.file_system();
        let content = if file_system.exists(path).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })? {
            Some(file_system.read(path).map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?)
        } else {
            None
        };
        backups.push(Backup {
            path: path.to_path_buf(),
            content,
        });

        Ok(())
    }

    /// Restore original filesystem content after a failed source commit.
    fn restore(&self, operation: Error, backups: &[Backup]) -> Error {
        // restore every path even when one restoration fails
        let mut failure = None;
        for backup in backups.iter().rev() {
            let result = match &backup.content {
                Some(content) => self.repository.file_system().write(&backup.path, content),
                None => self.repository.file_system().remove_file(&backup.path),
            };
            if let Err(source) = result
                && source.kind() != ErrorKind::NotFound
                && failure.is_none()
            {
                failure = Some((backup.path.clone(), source));
            }
        }

        match failure {
            Some((path, source)) => Error::RollbackFailed {
                operation: Box::new(operation),
                path,
                source,
            },
            None => operation,
        }
    }

    /// Write one edit to disk.
    fn write_edit(&self, edit: &Edit) -> Result<(), Error> {
        match edit {
            Edit::SetText { path, text } => {
                self.create_parent(path)?;
                self.repository
                    .file_system()
                    .write_string(path, text)
                    .map_err(|source| Error::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
            }
            Edit::SetBytes { path, bytes } => {
                self.create_parent(path)?;
                self.repository
                    .file_system()
                    .write(path, bytes)
                    .map_err(|source| Error::Io {
                        path: path.to_path_buf(),
                        source,
                    })?;
            }
            Edit::Remove { path } => {
                if let Err(source) = self.repository.file_system().remove_file(path)
                    && source.kind() != ErrorKind::NotFound
                {
                    return Err(Error::Io {
                        path: path.to_path_buf(),
                        source,
                    });
                }
            }
            Edit::EditText { .. } => {
                return Err(Error::InvalidEdit {
                    detail: "text patch edits cannot be written directly to disk".to_string(),
                });
            }
            Edit::Move { from, to } => {
                let bytes =
                    self.repository
                        .file_system()
                        .read(from)
                        .map_err(|source| Error::Io {
                            path: from.to_path_buf(),
                            source,
                        })?;
                self.create_parent(to)?;
                self.repository
                    .file_system()
                    .write(to, &bytes)
                    .map_err(|source| Error::Io {
                        path: to.to_path_buf(),
                        source,
                    })?;
                self.repository
                    .file_system()
                    .remove_file(from)
                    .map_err(|source| Error::Io {
                        path: from.to_path_buf(),
                        source,
                    })?;
            }
        }

        Ok(())
    }

    /// Create the parent directory for one file path.
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
