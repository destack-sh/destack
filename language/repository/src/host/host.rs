use std::sync::Arc;

use destack_artifact::BuildId;
use destack_source::FileSystem;

use super::{Clock, Environment};
use crate::BlobStore;

/// Host capabilities available to repository operations.
#[derive(Debug, Clone)]
pub struct Host {
    /// The Destack build producing derived artifacts.
    build_id: BuildId,
    /// Captured invocation environment.
    environment: Environment,
    /// File system backing repository discovery and loads.
    files: Arc<dyn FileSystem>,
    /// Explicit shared BlobStore, when supplied by the host.
    blob_store: Option<Arc<dyn BlobStore>>,
    /// Clock available to repository tooling.
    clock: Clock,
    /// Execution available to repository tooling.
    execution: Execution,
}

impl Host {
    /// Create one host from explicit capabilities.
    pub fn new(build_id: BuildId, environment: Environment, files: Arc<dyn FileSystem>) -> Self {
        Self {
            build_id,
            environment,
            files,
            blob_store: None,
            clock: Clock::default(),
            execution: Execution::default(),
        }
    }

    /// Return this host with one shared BlobStore.
    pub fn with_blob_store(mut self, blob_store: Arc<dyn BlobStore>) -> Self {
        self.blob_store = Some(blob_store);

        self
    }

    /// Return the Destack build producing derived artifacts.
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

    /// Return the file system backing repository discovery and loads.
    pub fn files(&self) -> &Arc<dyn FileSystem> {
        &self.files
    }

    /// Return the explicitly supplied shared BlobStore.
    pub(crate) fn blob_store(&self) -> Option<&Arc<dyn BlobStore>> {
        self.blob_store.as_ref()
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
