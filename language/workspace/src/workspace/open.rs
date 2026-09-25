use std::path::PathBuf;
use std::sync::Arc;

use tspp_repository::{DestackLayoutOverride, Environment, Repository, Settings};
use tspp_session::Executor;
use tspp_source::Edit;

#[cfg(not(target_arch = "wasm32"))]
use tspp_artifact::ArtifactCache;
#[cfg(not(target_arch = "wasm32"))]
use tspp_repository::{DestackLayout, Host, RepositoryError};
#[cfg(not(target_arch = "wasm32"))]
use tspp_source::PhysicalFileSystem;

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
        let build_id = Self::BUILD_ID;
        let artifact_cache =
            DestackLayout::resolve_cache(cwd, &home, &environment, &settings, None);
        let artifact_cache =
            ArtifactCache::open(build_id, artifact_cache, settings.cache.maximum_bytes)
                .map(Arc::new)
                .map_err(RepositoryError::from)?;
        let host = Host::new(build_id, environment, file_system)
            .with_artifact_cache(artifact_cache, executor.worker_count());
        let (repository, physical) =
            Repository::open(path, host, settings, DestackLayoutOverride::default())?;

        // restore cached artifacts valid at this revision
        repository.restore_artifacts(physical, executor.worker_count())?;

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
            Self::BUILD_ID,
            edits,
            Environment::default(),
            Settings::default(),
            DestackLayoutOverride::default(),
        )?;

        Self::new(Arc::new(repository), physical, executor)
    }
}
