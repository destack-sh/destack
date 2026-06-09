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
use destack_source::{FileSystem, PhysicalFileSystem};

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
    /// Open one session from one source input.
    pub fn open(source: bridge::Source) -> Result<Self> {
        let repository = match source {
            bridge::Source::FileSystem { path } => Self::open_repository_from_file_system(
                PathBuf::from(path),
                Arc::new(PhysicalFileSystem::new()),
            )?,
            bridge::Source::Memory { root, edits } => {
                let edits = edits
                    .into_iter()
                    .map(session::FileEdit::try_from)
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

        Self::from_repository(repository)
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

    /// Apply one file update through the default session ref.
    pub fn update(&self, update: bridge::FileUpdate) -> Result<bridge::FileUpdateResult> {
        let update = session::FileUpdate::try_from(update).map_err(Error::new)?;
        let result = self
            .session
            .update(self.session.head(), update)
            .map_err(Error::new)?;

        Ok(bridge::FileUpdateResult::from_session_update(
            &self.session,
            result,
        ))
    }

    /// Reload tracked files from this session backing source.
    pub fn reload(&self) -> Result<Vec<bridge::FileChange>> {
        let updates = self
            .session
            .reload_from_fs(self.session.head())
            .map_err(Error::new)?;
        let updates = updates
            .into_iter()
            .map(|update| bridge::FileChange::from_session_update(&self.session, update))
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

    /// Return one raw artifact record for one immutable revision.
    pub fn artifact_record(
        &self,
        revision: bridge::Revision,
        key: bridge::ArtifactKey,
    ) -> Result<bridge::ArtifactRecord> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let record = repository
            .artifact_store()
            .record(&version, repository.string_pool())
            .map_err(Error::new)?
            .ok_or_else(|| Error::new(format!("artifact record is missing for {version:?}")))?;

        Ok(bridge::ArtifactRecord::from_artifact(record))
    }

    /// Return the parsed DIR artifact for one loaded module.
    pub fn parse(
        &self,
        revision: bridge::Revision,
        module: bridge::Module,
    ) -> Result<bridge::DirParsed> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let module_id = module.id.clone();
        let module = module.id.into_source().map_err(Error::new)?;
        let key = bridge::ArtifactKey::DirParsed {
            module: module_id.clone(),
        };
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let parsed = repository
            .artifact_store()
            .dir_parsed(&version)
            .ok_or_else(|| Error::new(format!("parsed DIR artifact is missing for {version:?}")))?;

        Ok(bridge::DirParsed::from_artifact(
            version,
            module.into(),
            parsed.as_ref(),
        ))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    pub fn resolve(
        &self,
        revision: bridge::Revision,
        module: bridge::Module,
        profile: bridge::ProfileId,
    ) -> Result<bridge::DirResolved> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let module_id = module.id.clone();
        let profile_id = profile.clone();
        let module = module.id.into_source().map_err(Error::new)?;
        let profile = profile.into_source().map_err(Error::new)?;
        let key = bridge::ArtifactKey::DirResolved {
            module: module_id,
            profile: profile_id,
        };
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let resolved = repository
            .artifact_store()
            .dir_resolved(&version)
            .ok_or_else(|| {
                Error::new(format!("resolved DIR artifact is missing for {version:?}"))
            })?;

        Ok(bridge::DirResolved::from_artifact(
            version,
            module.into(),
            profile.into(),
            resolved.as_ref(),
        ))
    }

    /// Return the checked DIR facade artifact for one loaded module profile.
    pub fn check(
        &self,
        revision: bridge::Revision,
        module: bridge::Module,
        profile: bridge::ProfileId,
    ) -> Result<bridge::DirChecked> {
        let revision = revision.into_repository().map_err(Error::new)?;
        let module_id = module.id.clone();
        let profile_id = profile.clone();
        let module = module.id.into_source().map_err(Error::new)?;
        let profile = profile.into_source().map_err(Error::new)?;
        let key = bridge::ArtifactKey::DirChecked {
            module: module_id,
            profile: profile_id,
        };
        let key = key.into_artifact().map_err(Error::new)?;
        let version = self.session.require(revision, key).map_err(Error::new)?;
        let repository = self.session.repository();
        let store = repository.artifact_store();
        let checked = store.dir_checked(&version).ok_or_else(|| {
            Error::new(format!("checked DIR artifact is missing for {version:?}"))
        })?;

        Ok(bridge::DirChecked::from_artifact(
            version,
            module.into(),
            profile.into(),
            checked.as_ref(),
        ))
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
    fn from_repository(repository: repository::Repository) -> Result<Self> {
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

    /// Open one repository from one filesystem source.
    fn open_repository_from_file_system(
        path: PathBuf,
        file_system: Arc<dyn FileSystem>,
    ) -> Result<repository::Repository> {
        session::open_repository_from_fs(
            path,
            file_system,
            repository::Environment::default(),
            repository::Settings::default(),
            repository::DestackLayoutOverride::default(),
        )
        .map_err(Error::new)
    }
}
