use std::error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_bridge_language as bridge;
use destack_compiler as compiler;
use destack_linter as linter;
use destack_query as query;
use destack_repository as repository;
use destack_session as session;
use destack_source as source;
use source::FileSystem;

/// Result returned by the Rust bridge facade.
pub type Result<T> = std::result::Result<T, Error>;

/// Error returned by the Rust bridge facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Error message.
    message: String,
}

/// Live language session exposed to Rust clients.
#[derive(Debug)]
pub struct Session {
    /// Live language session.
    session: session::Session,
}

impl Error {
    /// Create one bridge error from a displayable error.
    fn new(error: impl ToString) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

impl Display for Error {
    /// Format this bridge error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl error::Error for Error {}

impl Session {
    /// Open one session from an explicit source snapshot.
    pub fn open_source(root: impl Into<PathBuf>, source: bridge::SourceSnapshot) -> Result<Self> {
        let source = session::SourceSnapshot::try_from(source).map_err(Error::new)?;
        let repository = session::open_repository_from_source(
            root.into(),
            source,
            repository::Environment::default(),
            repository::Settings::default(),
            repository::DestackLayoutOverride::default(),
        )
        .map_err(Error::new)?;

        Self::open(repository)
    }

    /// Open one session from a native filesystem path.
    pub fn open_path(path: impl Into<PathBuf>) -> Result<Self> {
        let file_system = Arc::new(source::PhysicalFileSystem::new());
        let repository = session::open_repository_from_fs(
            path.into(),
            file_system,
            repository::Environment::default(),
            repository::Settings::default(),
            repository::DestackLayoutOverride::default(),
        )
        .map_err(Error::new)?;

        Self::open(repository)
    }

    /// Return the current session revision.
    pub fn revision(&self) -> Result<bridge::Revision> {
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(Error::new)?;

        Ok(bridge::Revision::from_repository(revision))
    }

    /// Return editable repository file paths at the current revision.
    pub fn files(&self) -> Result<Vec<bridge::SessionFile>> {
        let repository = self.session.repository();
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(Error::new)?;
        let mut paths = repository
            .editable_file_logical_paths(revision)
            .map_err(Error::new)?
            .into_iter()
            .map(|(_, path)| repository.string_pool().get(path).to_string())
            .collect::<Vec<_>>();

        paths.sort();

        let files = paths.into_iter().map(bridge::SessionFile::new).collect();

        Ok(files)
    }

    /// Apply one source update through the default session ref.
    pub fn update(&self, update: bridge::SourceUpdate) -> Result<bridge::SourceUpdateResult> {
        let update = session::SourceUpdate::try_from(update).map_err(Error::new)?;
        let result = self
            .session
            .update(self.session.head(), update)
            .map_err(Error::new)?;

        Ok(bridge::SourceUpdateResult::from_session_update(
            &self.session,
            result,
        ))
    }

    /// Reload tracked files from this session filesystem.
    pub fn reload(&self) -> Result<Vec<bridge::FileUpdate>> {
        let updates = self
            .session
            .reload_from_fs(self.session.head())
            .map_err(Error::new)?;
        let updates = updates
            .into_iter()
            .map(|update| bridge::FileUpdate::from_session_update(&self.session, update))
            .collect();

        Ok(updates)
    }

    /// Load one module path into the default session ref.
    pub fn load_module(&self, path: impl AsRef<Path>) -> Result<bridge::Module> {
        let module = self
            .session
            .load_module_from_fs(self.session.head(), path.as_ref())
            .map_err(Error::new)?;

        Ok(bridge::Module::new(module.into()))
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(
        &self,
        revision: bridge::Revision,
        keys: Vec<bridge::ArtifactKey>,
    ) -> Result<()> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let keys = keys
            .into_iter()
            .map(bridge::ArtifactKey::into_artifact)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::new)?;

        self.session.provide(revision, &keys).map_err(Error::new)
    }

    /// Require one root artifact for one immutable revision.
    pub fn require(
        &self,
        revision: bridge::Revision,
        key: bridge::ArtifactKey,
    ) -> Result<bridge::ArtifactVersion> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;

        Ok(bridge::ArtifactVersion::from_artifact(version))
    }

    /// Return diagnostics for one immutable revision.
    pub fn diagnostics(
        &self,
        revision: bridge::Revision,
        key: Option<bridge::ArtifactKey>,
    ) -> Result<Vec<bridge::Diagnostic>> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key
            .map(bridge::ArtifactKey::into_artifact)
            .transpose()
            .map_err(Error::new)?;
        let diagnostics = self
            .session
            .repository()
            .diagnostics(revision, key)
            .map_err(Error::new)?;
        let diagnostics = diagnostics
            .to_vec()
            .into_iter()
            .map(bridge::Diagnostic::from_source)
            .collect();

        Ok(diagnostics)
    }

    /// Return sidecars for one artifact key in one immutable revision.
    pub fn sidecars(
        &self,
        revision: bridge::Revision,
        key: bridge::ArtifactKey,
    ) -> Result<Vec<bridge::ArtifactSidecar>> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key.into_artifact().map_err(Error::new)?;
        let sidecars = self
            .session
            .repository()
            .artifact_sidecars(revision, key)
            .map_err(Error::new)?;
        let sidecars = sidecars
            .iter()
            .cloned()
            .map(bridge::ArtifactSidecar::from_artifact)
            .collect();

        Ok(sidecars)
    }

    /// Open one Rust session from one prepared repository.
    fn open(repository: repository::Repository) -> Result<Self> {
        let repository = Arc::new(repository);
        let root = repository.path().to_path_buf();
        let head = repository::Ref::for_root(&root);
        let compiler = Arc::new(compiler::Compiler::new(Arc::clone(&repository)));
        let linter = Arc::new(linter::Linter::new(Arc::clone(&repository)));
        let query = Arc::new(query::Query::new(Arc::clone(&repository)));
        let worker_count = session::Session::default_worker_count();
        let session = session::Session::new(
            root.clone(),
            root,
            repository,
            head,
            compiler,
            linter,
            query,
            worker_count,
            None,
        )
        .map_err(Error::new)?;

        Ok(Self { session })
    }
}
