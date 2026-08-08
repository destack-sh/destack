use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use destack_repository as repository;
use destack_repository::{FileSystemSource, Revision, RevisionPin};
use destack_serde::Reflect;
use destack_source::{ContentId, Edit, FileId, Uri};
use serde::{Deserialize, Serialize};

use crate::diagnostic::Error;
use crate::file::{FileImage, FileUpdate, UpdateKind};
use crate::workspace::{LocalWorkspace, WorkspaceRoot};

/// Workspace projection of one committed edit batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Commit {
    /// Previous repository revision.
    pub before: Revision,
    /// Updated repository revision.
    pub after: Revision,
    /// Workspace updates produced by the commit.
    pub updates: Vec<FileUpdate>,
}

/// One requested source mutation batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SourceUpdate {
    /// Optional expected base revision.
    pub base: Option<Revision>,
    /// Source file edits.
    pub edits: Vec<Edit>,
}

/// One source commit retained until publication.
#[derive(Debug)]
pub(super) struct PendingCommit {
    /// The pinned base revision.
    before: RevisionPin,
    /// The pinned edited revision.
    after: RevisionPin,
    /// The changed file ids in edit order.
    file_ids: Vec<FileId>,
}

impl PendingCommit {
    /// Build one source commit against an immutable revision.
    fn new(
        repository: &Arc<repository::Repository>,
        revision: Revision,
        edits: Vec<repository::Edit>,
    ) -> Result<Self, Error> {
        let file_ids = Self::file_ids(&edits);
        let before = repository.pin(revision)?;
        let after = repository.commit_edits(revision, edits)?;
        let after = repository.pin(after)?;

        Ok(Self {
            before,
            after,
            file_ids,
        })
    }

    /// Return file ids affected by repository edits.
    fn file_ids(edits: &[repository::Edit]) -> Vec<FileId> {
        let mut file_ids = Vec::new();
        let mut seen = HashSet::new();

        // retain each changed file in edit order
        for edit in edits {
            for file_id in edit.changed_file_ids() {
                if seen.insert(file_id) {
                    file_ids.push(file_id);
                }
            }
        }

        file_ids
    }

    /// Return the prepared revision produced by this commit.
    pub(super) fn after(&self) -> Revision {
        self.after.revision()
    }

    /// Advance one root through this commit.
    pub(super) fn advance(
        self,
        root: &WorkspaceRoot,
        workspace: &LocalWorkspace,
    ) -> Result<Commit, Error> {
        let before = self.before.revision();
        let after = self.after.revision();
        let updates = workspace.file_updates(before, after, self.file_ids)?;

        // publish only after the complete observable commit is ready
        let was_published = workspace
            .repository
            .advance_ref(&root.head, before, after)?;

        // reject publication after another root mutation
        if !was_published {
            let current = root.revision(&workspace.repository)?;

            return Err(Error::StaleRevision {
                expected: before,
                current,
            });
        }

        Ok(Commit {
            before,
            after,
            updates,
        })
    }
}

impl Commit {
    /// Publish this commit to one root's semantic watches.
    pub(super) fn publish(self, root: &WorkspaceRoot) -> Self {
        if self.before != self.after {
            root.watch.lock().publish(&self);
        }

        self
    }
}

impl WorkspaceRoot {
    /// Prepare one source edit batch against this root's exact revision.
    pub(super) fn prepare(
        &self,
        revision: Revision,
        edits: Vec<Edit>,
        workspace: &LocalWorkspace,
    ) -> Result<PendingCommit, Error> {
        let current = self.revision(&workspace.repository)?;
        if current != revision {
            return Err(Error::StaleRevision {
                expected: revision,
                current,
            });
        }

        // lower every source edit against the same immutable base
        let edits = edits
            .into_iter()
            .map(|edit| self.lower(&workspace.repository, revision, edit))
            .collect::<Result<Vec<_>, _>>()?;

        PendingCommit::new(&workspace.repository, revision, edits)
    }
}

impl LocalWorkspace {
    /// Commit one source edit batch through an opened root.
    pub(super) fn commit(
        &self,
        root: &WorkspaceRoot,
        revision: Revision,
        edits: Vec<Edit>,
    ) -> Result<Commit, Error> {
        let pending = root.prepare(revision, edits, self)?;
        let commit = pending.advance(root, self)?;

        Ok(commit.publish(root))
    }

    /// Reload one root from its host filesystem.
    pub fn reload(&self, root: &Path) -> Result<Option<Commit>, Error> {
        let root = self.root(root)?;
        let _write = root.write()?;

        self.reload_locked(root.as_ref())
    }

    /// Reload one opened root while its source state is exclusively owned.
    pub(crate) fn reload_locked(&self, root: &WorkspaceRoot) -> Result<Option<Commit>, Error> {
        let before = root.revision(&self.repository)?;
        let source = FileSystemSource::new(self.repository.as_ref(), &root.path, before);
        let edits = source.edits()?;
        let pending = PendingCommit::new(&self.repository, before, edits)?;
        let commit = pending.advance(root, self)?;

        // restore watch delivery after the filesystem was reconciled
        root.watch.lock().recover();
        if commit.before == commit.after {
            return Ok(None);
        }

        Ok(Some(commit.publish(root)))
    }

    /// Project changed repository files into workspace updates.
    fn file_updates(
        &self,
        before: Revision,
        after: Revision,
        file_ids: Vec<FileId>,
    ) -> Result<Vec<FileUpdate>, Error> {
        let mut updates = Vec::with_capacity(file_ids.len());

        // project every changed file exactly once
        for file_id in file_ids {
            updates.push(self.file_update(before, after, file_id)?);
        }

        Ok(updates)
    }

    /// Project one changed repository file into a workspace update.
    fn file_update(
        &self,
        before: Revision,
        after: Revision,
        file_id: FileId,
    ) -> Result<FileUpdate, Error> {
        let file = self.repository.file(after, file_id)?;

        // project a live file from the updated revision
        if let Some(file) = file {
            let path = file.path.as_deref().ok_or_else(|| Error::Internal {
                detail: format!("updated repository file has no path: {file_id:?}"),
            })?;
            let uri = Uri::from_file_path(path);
            let content_id = file.content_id();
            let mut update = FileUpdate {
                module_id: self.repository.module_id_for_file(after, file_id)?,
                file_id,
                diagnostic_uri: uri,
                diagnostic_version: None,
                file: Some(FileImage::from(file.as_ref())),
                is_removed: false,
                kind: UpdateKind::for_path(path),
            };
            self.attach_open_file(path, content_id, &mut update);

            Ok(update)
        }
        // project a removed file from the previous revision
        else {
            let file = self
                .repository
                .file(before, file_id)?
                .ok_or_else(|| Error::Internal {
                    detail: format!("changed repository file is missing: {file_id:?}"),
                })?;
            let path = file.path.as_deref().ok_or_else(|| Error::Internal {
                detail: format!("removed repository file has no path: {file_id:?}"),
            })?;

            Ok(FileUpdate {
                module_id: self.repository.module_id_for_file(before, file_id)?,
                file_id,
                diagnostic_uri: Uri::from_file_path(path),
                diagnostic_version: None,
                file: None,
                is_removed: true,
                kind: UpdateKind::for_path(path),
            })
        }
    }

    /// Attach editor identity when its open content matches one committed file.
    fn attach_open_file(&self, path: &Path, content_id: ContentId, update: &mut FileUpdate) {
        let Some(file) = self.open_state(path) else {
            return;
        };
        if file.content_id != content_id {
            return;
        }

        update.diagnostic_uri = file.uri;
        update.diagnostic_version = Some(file.version);
    }

    /// Return the content identity for one path in a revision.
    pub(super) fn content_id(
        &self,
        root: &WorkspaceRoot,
        revision: Revision,
        path: &Path,
    ) -> Result<ContentId, Error> {
        let file_id = root.file_id(&self.repository, path)?;
        let content_id = self
            .repository
            .file_content_id(revision, file_id)?
            .ok_or_else(|| Error::Internal {
                detail: format!("file has no content in revision: {}", path.display()),
            })?;

        Ok(content_id)
    }
}
