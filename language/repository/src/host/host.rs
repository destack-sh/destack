use std::sync::Arc;

use tspp_artifact::{ArtifactCache, ArtifactCacheError, ArtifactTable, BuildId};
use tspp_core::{BlobStore, StringPool};
use tspp_source::FileSystem;

use super::{ArtifactCacheWriter, Clock, Environment};
use crate::EmbeddedBuiltinPackage;

/// Host capabilities available to repository operations.
#[derive(Debug, Clone)]
pub struct Host {
    /// The toolchain build producing derived artifacts.
    build_id: BuildId,
    /// Captured invocation environment.
    environment: Environment,
    /// File system backing repository discovery and loads.
    files: Arc<dyn FileSystem>,
    /// Embedded Builtin Package shipped with the current build.
    builtin: Arc<EmbeddedBuiltinPackage>,
    /// Retained immutable bytes shared by hosted repositories.
    blobs: Arc<BlobStore>,
    /// Derived artifact results shared by hosted repositories.
    artifact_table: Arc<ArtifactTable>,
    /// Asynchronous persistent artifact writer shared by hosted repositories.
    artifact_cache_writer: Option<Arc<ArtifactCacheWriter>>,
    /// Interned strings shared by hosted repositories.
    strings: Arc<StringPool>,
    /// Clock available to repository tooling.
    clock: Clock,
    /// Execution available to repository tooling.
    execution: Execution,
}

impl Host {
    /// Create one host from explicit capabilities.
    pub fn new(build_id: BuildId, environment: Environment, files: Arc<dyn FileSystem>) -> Self {
        let blobs = Arc::new(BlobStore::new());
        let builtin = Arc::new(EmbeddedBuiltinPackage::new(&blobs));

        Self {
            build_id,
            environment,
            files,
            builtin,
            blobs,
            artifact_table: Arc::new(ArtifactTable::default()),
            artifact_cache_writer: None,
            strings: Arc::new(StringPool::new()),
            clock: Clock::default(),
            execution: Execution::default(),
        }
    }

    /// Return this host with one shared BlobStore.
    pub fn with_blob_store(mut self, blob_store: Arc<BlobStore>) -> Self {
        self.builtin = Arc::new(EmbeddedBuiltinPackage::new(&blob_store));
        self.blobs = blob_store;

        self
    }

    /// Return this host with one persistent artifact cache.
    pub fn with_artifact_cache(
        mut self,
        artifact_cache: Arc<ArtifactCache>,
        worker_count: usize,
    ) -> Self {
        self.artifact_cache_writer = Some(Arc::new(ArtifactCacheWriter::new(
            artifact_cache,
            worker_count,
        )));

        self
    }

    /// Return the toolchain build producing derived artifacts.
    pub fn build_id(&self) -> BuildId {
        self.build_id
    }

    /// Return this host with one clock capability.
    pub fn with_clock(mut self, clock: Clock) -> Self {
        self.clock = clock;

        self
    }

    /// Return this host with one execution capability.
    pub fn with_execution(mut self, execution: Execution) -> Self {
        self.execution = execution;

        self
    }

    /// Return the captured invocation environment.
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Return this host with one invocation environment.
    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;

        self
    }

    /// Return the file system backing repository discovery and loads.
    pub fn files(&self) -> &Arc<dyn FileSystem> {
        &self.files
    }

    /// Return the embedded Builtin Package.
    pub(crate) fn embedded_builtin(&self) -> &Arc<EmbeddedBuiltinPackage> {
        &self.builtin
    }

    /// Return the shared Blob store.
    pub(crate) fn blob_store(&self) -> &Arc<BlobStore> {
        &self.blobs
    }

    /// Return the shared artifact table.
    pub(crate) fn artifact_table(&self) -> &Arc<ArtifactTable> {
        &self.artifact_table
    }

    /// Return persistent artifact storage when configured.
    pub(crate) fn artifact_cache(&self) -> Option<&ArtifactCache> {
        self.artifact_cache_writer
            .as_ref()
            .map(|writer| writer.cache())
    }

    /// Return the shared persistent artifact writer when configured.
    pub(crate) fn artifact_cache_writer(&self) -> Option<&Arc<ArtifactCacheWriter>> {
        self.artifact_cache_writer.as_ref()
    }

    /// Wait for every persistent artifact write scheduled through this host.
    pub fn flush_artifact_cache(&self) -> Result<(), ArtifactCacheError> {
        self.artifact_cache_writer
            .as_ref()
            .map_or(Ok(()), |writer| writer.flush())
    }

    /// Return the shared interned strings.
    pub(crate) fn string_pool(&self) -> &Arc<StringPool> {
        &self.strings
    }

    /// Return the clock available to repository tooling.
    pub fn clock(&self) -> Clock {
        self.clock
    }

    /// Return the execution available to repository tooling.
    pub fn execution(&self) -> Execution {
        self.execution
    }
}

/// Execution available to repository tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Execution {
    /// Run work on real host threads.
    Threaded,
    /// Run bounded work when explicitly polled by the host.
    Cooperative,
}

impl Default for Execution {
    /// Return the default execution capability for this host platform.
    fn default() -> Self {
        default_execution()
    }
}

/// Return threaded execution on hosts with thread support.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn default_execution() -> Execution {
    Execution::Threaded
}

/// Return inline execution on bare WebAssembly.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn default_execution() -> Execution {
    Execution::Cooperative
}
