use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_bridge_language as bridge;
use destack_compiler as compiler;
use destack_linter as linter;
use destack_query as query;
use destack_repository as repository;
use destack_session as session;
use destack_source as source;
use napi::Result;
use napi_derive::napi;
use source::FileSystem;

use crate::{
    FileUpdate, Module, Revision, SessionFile, SourceSnapshot, SourceUpdate, SourceUpdateResult,
};

/// Live language session exposed to Node API clients.
#[derive(Debug)]
#[napi]
pub struct Session {
    /// Live language session.
    session: session::Session,
}

#[napi]
impl Session {
    /// Open one session from an explicit source snapshot.
    #[napi(factory)]
    pub fn open_source(root: String, source: SourceSnapshot) -> Result<Self> {
        let source = session::SourceSnapshot::try_from(source.into_bridge()?).map_err(to_error)?;
        let repository = session::open_repository_from_source(
            root.into(),
            source,
            repository::Environment::default(),
            repository::Settings::default(),
            repository::DestackLayoutOverride::default(),
        )
        .map_err(to_error)?;

        Self::open(repository)
    }

    /// Open one session from a native filesystem path.
    #[napi(factory)]
    pub fn open_path(path: String) -> Result<Self> {
        let file_system = Arc::new(source::PhysicalFileSystem::new());
        let repository = session::open_repository_from_fs(
            PathBuf::from(path),
            file_system,
            repository::Environment::default(),
            repository::Settings::default(),
            repository::DestackLayoutOverride::default(),
        )
        .map_err(to_error)?;

        Self::open(repository)
    }

    /// Return the current session revision.
    #[napi]
    pub fn revision(&self) -> Result<Revision> {
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(to_error)?;
        let revision = bridge::Revision::from_repository(revision);

        Ok(Revision::from_bridge(revision))
    }

    /// Return editable repository file paths at the current revision.
    #[napi]
    pub fn files(&self) -> Result<Vec<SessionFile>> {
        let repository = self.session.repository();
        let revision = self
            .session
            .revision(self.session.head())
            .map_err(to_error)?;
        let mut paths = repository
            .editable_file_logical_paths(revision)
            .map_err(to_error)?
            .into_iter()
            .map(|(_, path)| repository.string_pool().get(path).to_string())
            .collect::<Vec<_>>();
        paths.sort();
        let files = paths
            .into_iter()
            .map(|path| {
                let file = bridge::SessionFile::new(path);

                SessionFile::from_bridge(file)
            })
            .collect();

        Ok(files)
    }

    /// Apply one source update through the default session ref.
    #[napi]
    pub fn update(&self, update: SourceUpdate) -> Result<SourceUpdateResult> {
        let update = session::SourceUpdate::try_from(update.into_bridge()?).map_err(to_error)?;
        let result = self
            .session
            .update(self.session.head(), update)
            .map_err(to_error)?;
        let result = bridge::SourceUpdateResult::from_session_update(&self.session, result);

        Ok(SourceUpdateResult::from_bridge(result))
    }

    /// Reload tracked files from this session filesystem.
    #[napi]
    pub fn reload(&self) -> Result<Vec<FileUpdate>> {
        let updates = self
            .session
            .reload_from_fs(self.session.head())
            .map_err(to_error)?;
        let updates = updates
            .into_iter()
            .map(|update| {
                FileUpdate::from_bridge(bridge::FileUpdate::from_session_update(
                    &self.session,
                    update,
                ))
            })
            .collect();

        Ok(updates)
    }

    /// Load one module path into the default session ref.
    #[napi]
    pub fn load_module(&self, path: String) -> Result<Module> {
        let module = self
            .session
            .load_module_from_fs(self.session.head(), Path::new(&path))
            .map_err(to_error)?;
        let module = bridge::Module::new(format!("{module:?}"));

        Ok(Module::from_bridge(module))
    }
}

impl Session {
    /// Open one NAPI session from one prepared repository.
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
        .map_err(to_error)?;

        Ok(Self { session })
    }
}

/// Convert one bridge error into one NAPI error.
fn to_error(error: impl ToString) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
