use std::path::Path;
use std::sync::Arc;

use destack_repository::{Repository, Revision, RevisionPin};
use destack_session::{Session, SessionError};
use destack_source::{File, FileId};

use crate::diagnostic::Error;

use super::LocalWorkspace;

/// Pinned read state for one session at one pinned repository revision.
#[derive(Debug)]
pub(crate) struct SessionPin {
    /// The live session.
    session: Arc<Session>,
    /// The retained repository revision.
    revision: RevisionPin,
}

impl SessionPin {
    /// Create one session pin.
    pub(super) fn new(session: Arc<Session>, revision: RevisionPin) -> Self {
        Self { session, revision }
    }

    /// Return the live session for workspace internals.
    pub(super) fn session(&self) -> &Session {
        self.session.as_ref()
    }

    /// Return the pinned revision id.
    pub(crate) fn revision(&self) -> Revision {
        self.revision.revision()
    }

    /// Return the repository backing this revision.
    pub(crate) fn repository(&self) -> &Repository {
        self.revision.repository()
    }

    /// Return one tracked file id for a path in this revision.
    pub(crate) fn file_id(&self, path: &Path) -> Result<Option<FileId>, Error> {
        let file_id = self.session.file_id(path);
        let file = self.repository().file(self.revision(), file_id)?;

        Ok(file.map(|_| file_id))
    }

    /// Return one tracked file by id from this revision.
    pub(crate) fn file(&self, file_id: FileId) -> Result<Arc<File>, Error> {
        let file = self
            .repository()
            .file(self.revision(), file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        Ok(file)
    }
}

impl LocalWorkspace {
    /// Pin one root session at its current revision.
    pub(crate) fn pin_session(&self, root: &Path) -> Result<SessionPin, Error> {
        let session = self.session(root)?;
        let repository = session.repository();
        let revision = session.revision(session.head())?;
        let revision = repository.pin(revision)?;

        Ok(SessionPin::new(session, revision))
    }
}
