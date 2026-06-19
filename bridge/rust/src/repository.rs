use std::path::PathBuf;
use std::sync::Arc;

use destack_bridge_language as bridge;
use destack_repository as repository;
use destack_session as session;
use destack_source::{FileSystem, PhysicalFileSystem};

use crate::{Error, Result, Session, Workspace};

/// Durable language repository exposed to bridge clients.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Shared repository state.
    pub(crate) repository: Arc<repository::Repository>,
}

impl Repository {
    /// Open one repository from one source input.
    pub fn open(source: bridge::Source) -> Result<Self> {
        let repository = match source {
            bridge::Source::FileSystem { path } => Session::open_repository_from_file_system(
                PathBuf::from(path),
                Arc::new(PhysicalFileSystem::new()),
            )?,
            bridge::Source::Memory { root, edits } => {
                let edits = edits
                    .into_iter()
                    .map(session::Edit::try_from)
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Error::new)?;

                session::open_repository_from_memory(
                    PathBuf::from(root),
                    edits,
                    repository::Environment::default(),
                    repository::Settings::default(),
                    repository::DestackLayoutOverride::default(),
                )
                .map_err(Error::new)?
            }
        };

        Ok(Self {
            repository: Arc::new(repository),
        })
    }

    /// Return this repository root path.
    pub fn root(&self) -> String {
        self.repository.path().to_string_lossy().to_string()
    }

    /// Open one root session over this repository.
    pub fn session(&self) -> Result<Session> {
        Session::from_repository(Arc::clone(&self.repository))
    }

    /// Open one root workspace over this repository.
    pub fn workspace(&self) -> Result<Workspace> {
        Workspace::from_repository(self.clone())
    }
}
