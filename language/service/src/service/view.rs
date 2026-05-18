use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;

use destack_session::{Session, SessionError};
use destack_source::{File, FileId};
use destack_workspace::{Repository, Revision, RevisionPin};

use super::{LanguageService, LanguageServiceError};

/// Read view over one session at one pinned repository revision.
#[derive(Debug)]
pub struct SessionRevisionView {
    /// The live session.
    session: Arc<Session>,
    /// The retained repository revision.
    revision: RevisionPin,
}

impl SessionRevisionView {
    /// Create one session revision view.
    pub(super) fn new(session: Arc<Session>, revision: RevisionPin) -> Self {
        Self { session, revision }
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
    pub fn file_id(&self, path: &Path) -> Result<Option<FileId>, LanguageServiceError> {
        let file_id = self.repository().file_id(path);
        let file = self.repository().file(self.revision(), file_id)?;

        Ok(file.map(|_| file_id))
    }

    /// Return one tracked file by id from this revision.
    pub fn file(&self, file_id: FileId) -> Result<Arc<File>, LanguageServiceError> {
        let file = self
            .repository()
            .file(self.revision(), file_id)?
            .ok_or(SessionError::FileNotTracked { file_id })?;

        Ok(file)
    }

    /// Return one file view for a path in this revision.
    pub fn file_view(self, path: &Path) -> Result<FileView, LanguageServiceError> {
        let file_id = self
            .file_id(path)?
            .ok_or_else(|| LanguageServiceError::FileMissing {
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

impl Deref for SessionRevisionView {
    type Target = Session;

    /// Return the live session behind this pinned revision.
    fn deref(&self) -> &Self::Target {
        self.session.as_ref()
    }
}

/// Read view over one file inside a pinned session revision.
#[derive(Debug)]
pub struct FileView {
    /// The pinned session revision.
    pub session: SessionRevisionView,
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

impl LanguageService {
    /// Return a read view over the current revision for one root session.
    pub fn session_revision_view(
        &self,
        root: &Path,
    ) -> Result<SessionRevisionView, LanguageServiceError> {
        let session = self.session(root)?;
        let repository = session.repository();
        let revision = session.revision(session.head())?;
        let revision = repository.pin(revision)?;

        Ok(SessionRevisionView::new(session, revision))
    }

    /// Return a read view over one file at the current revision.
    pub fn file_view(&self, path: &Path) -> Result<FileView, LanguageServiceError> {
        let root = self.root_at(path)?;
        let session = self.session_revision_view(&root)?;

        session.file_view(path)
    }
}
