use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{ArtifactCache, BuildId};
use destack_repository::{
    DestackLayout, DestackLayoutOverride, Environment, Host, Repository, RepositoryError, Settings,
};
use destack_session::Executor;
use destack_source::Edit;

#[cfg(not(target_arch = "wasm32"))]
use destack_source::PhysicalFileSystem;

use super::Workspace;
use crate::Error;

impl Workspace {
    /// Open a local workspace from one filesystem path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(path: impl Into<PathBuf>, executor: Arc<Executor>) -> Result<Self, Error> {
        let path = path.into();
        let path = std::fs::canonicalize(&path).map_err(|source| Error::Io { path, source })?;
        let file_system = Arc::new(PhysicalFileSystem);
        let environment = Environment::capture_process();
        let cwd = environment.cwd.as_deref().unwrap_or(&path);
        let settings = Settings::default();
        let home = DestackLayout::resolve_home(cwd, &environment, None);
        let build_id = BuildId::current().map_err(|error| RepositoryError::InvalidArtifact {
            message: format!("failed to identify Destack build: {error}"),
        })?;
        let artifact_cache =
            DestackLayout::resolve_cache(cwd, &home, &environment, &settings, None);
        let artifact_cache = ArtifactCache::open(build_id, file_system.clone(), artifact_cache)
            .map(Arc::new)
            .map_err(RepositoryError::from)?;
        let host = Host::new(build_id, environment, file_system)
            .with_artifact_cache(artifact_cache, executor.worker_count());
        let (repository, physical) =
            Repository::open(path, host, settings, DestackLayoutOverride::default())?;

        Self::new(Arc::new(repository), physical, executor)
    }

    /// Open a local workspace from in-memory source edits.
    pub fn memory(
        root: impl Into<PathBuf>,
        edits: Vec<Edit>,
        executor: Arc<Executor>,
    ) -> Result<Self, Error> {
        let root = root.into();
        let (repository, physical) = Repository::memory(
            root,
            edits,
            Environment::default(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;

        Self::new(Arc::new(repository), physical, executor)
    }
}
