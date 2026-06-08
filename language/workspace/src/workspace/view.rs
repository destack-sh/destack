use std::path::Path;
use std::sync::Arc;

use destack_repository::{Repository, Revision, RevisionPin};
use destack_session::{Session, SessionError};
use destack_source::{File, FileId};

use crate::diagnostic::Error;

use super::Workspace;

/// Read view over one session at one pinned repository revision.
#[derive(Debug)]
pub struct Snapshot {
    /// The live session.
    session: Arc<Session>,
    /// The retained repository revision.
    revision: RevisionPin,
}

impl Snapshot {
    /// Create one snapshot.
    pub(super) fn new(session: Arc<Session>, revision: RevisionPin) -> Self {
        Self { session, revision }
    }

    /// Return the live session for workspace internals.
    pub(super) fn session(&self) -> &Session {
        self.session.as_ref()
    }

    /// Return the pinned revision id.
    pub fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Return the repository backing this revision.
    pub fn repository(&self) -> &Repository {
        self.revision.repository()
    }

    /// Return one tracked file id for a path in this revision.
    pub fn file_id(&self, path: &Path) -> Result<Option<FileId>, Error> {
        let file_id = self.session.file_id(path);
        let file = self.repository().file(self.revision(), file_id)?;

        Ok(file.map(|_| file_id))
    }

    /// Return one tracked file by id from this revision.
    pub fn file(&self, file_id: FileId) -> Result<Arc<File>, Error> {
        let file = self
            .repository()
            .file(self.revision(), file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        Ok(file)
    }

    /// Return one file view for a path in this revision.
    pub fn file_view(self, path: &Path) -> Result<FileView, Error> {
        let file_id = self.file_id(path)?.ok_or_else(|| Error::FileMissing {
            path: path.to_path_buf(),
        })?;
        let file = self.file(file_id)?;

        Ok(FileView {
            session: self,
            file_id,
            file,
        })
    }
}

/// Read view over one file inside a pinned session revision.
#[derive(Debug)]
pub struct FileView {
    /// The pinned session revision.
    pub session: Snapshot,
    /// The source file id.
    pub file_id: FileId,
    /// The source file image.
    pub file: Arc<File>,
}

impl FileView {
    /// Return the pinned revision id.
    pub fn revision(&self) -> Revision {
        self.session.revision()
    }

    /// Return the repository backing this file view.
    pub fn repository(&self) -> &Repository {
        self.session.repository()
    }
}

impl Workspace {
    /// Return a read view over the current revision for one root session.
    pub fn snapshot(&self, root: &Path) -> Result<Snapshot, Error> {
        let session = self.session(root)?;
        let repository = session.repository();
        let revision = session.revision(session.head())?;
        let revision = repository.pin(revision)?;

        Ok(Snapshot::new(session, revision))
    }

    /// Return a read view over one file at the current revision.
    pub fn file_view(&self, path: &Path) -> Result<FileView, Error> {
        let root = self.root_at(path)?;
        let session = self.snapshot(&root)?;

        session.file_view(path)
    }
}
