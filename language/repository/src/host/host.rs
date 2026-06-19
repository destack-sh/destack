use std::sync::Arc;

use destack_artifact::BlobStore;
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
use destack_artifact::DiskBlobStore;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use destack_artifact::MemoryBlobStore;
use destack_source::FileSystem;

use super::{Clock, Environment};

/// Host capabilities available to the repository toolchain.
#[derive(Debug, Clone)]
pub struct Host {
    /// Captured invocation environment.
    environment: Environment,
    /// File system backing repository discovery and loads.
    files: Arc<dyn FileSystem>,
    /// Shared byte store for persisted blobs.
    blob_store: Arc<dyn BlobStore>,
    /// Clock available to repository tooling.
    clock: Clock,
    /// Execution available to repository tooling.
    execution: Execution,
}

impl Host {
    /// Create one host from explicit capabilities.
    pub fn new(
        environment: Environment,
        files: Arc<dyn FileSystem>,
        blob_store: Arc<dyn BlobStore>,
    ) -> Self {
        Self {
            environment,
            files,
            blob_store,
            clock: Clock::default(),
            execution: Execution::default(),
        }
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

    /// Return the file system backing repository discovery and loads.
    pub fn files(&self) -> &Arc<dyn FileSystem> {
        &self.files
    }

    /// Return the shared byte store.
    pub fn blob_store(&self) -> &Arc<dyn BlobStore> {
        &self.blob_store
    }

    /// Replace the shared byte store.
    pub(crate) fn set_blob_store(&mut self, blob_store: Arc<dyn BlobStore>) {
        self.blob_store = blob_store;
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

/// Return the default blob store for this host platform.
pub fn default_blob_store() -> Arc<dyn BlobStore> {
    default_platform_blob_store()
}

/// Execution available to repository tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Execution {
    /// Run work on real host threads.
    Threaded,
    /// Run work inline on the calling thread.
    Inline,
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
    Execution::Inline
}

/// Return a persistent disk store on native hosts.
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
fn default_platform_blob_store() -> Arc<dyn BlobStore> {
    Arc::new(DiskBlobStore::new())
}

/// Return a process-local store on browser WebAssembly.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn default_platform_blob_store() -> Arc<dyn BlobStore> {
    Arc::new(MemoryBlobStore::new())
}
