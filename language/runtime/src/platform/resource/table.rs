use std::any::Any;
use std::collections::HashMap;
use std::fmt;
#[cfg(unix)]
use std::os::unix::io::RawFd;
#[cfg(windows)]
use std::os::windows::io::{RawHandle, RawSocket};
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{ResourceId, ResourceSnapshotAdapter, ResourceSnapshotPolicy};
use crate::runtime::{RuntimeHookState, with_current_binding_call_context};

/// Resource classification for platform handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceKind {
    /// File handle resources.
    File,
    /// Directory handle resources.
    Directory,
    /// Pipe endpoint resources.
    Pipe,
    /// Socket handle resources.
    Socket,
    /// Listener handle resources.
    Listener,
    /// Timer handle resources.
    Timer,
    /// Timer fd-style resources.
    TimerFd,
    /// File watch resources.
    Watch,
    /// Process handle resources.
    Process,
    /// Poll handle resources.
    Poll,
    /// Completion queue resources.
    Completion,
    /// User event token resources.
    Event,
    /// io_uring ring resources.
    Uring,
    /// Process fd-style resources.
    ProcessFd,
    /// Shared memory resources.
    SharedMemory,
    /// Semaphore resources.
    Semaphore,
    /// Signal subscription resources.
    Signal,
    /// Signal fd-style queue resources.
    SignalFd,
    /// Thread-domain resources.
    Thread,
    /// Mutex resources.
    Mutex,
    /// Read-write lock resources.
    RwLock,
    /// Condition variable resources.
    CondVar,
    /// Thread semaphore resources.
    ThreadSemaphore,
    /// Barrier resources.
    Barrier,
    /// Thread local key resources.
    ThreadLocal,
    /// Dynamic library resources.
    Library,
    /// Symbol resources.
    Symbol,
    /// Crypto store resources.
    CryptoStore,
    /// Crypto key resources.
    CryptoKey,
    /// Crypto certificate resources.
    CryptoCertificate,
    /// TLS context resources.
    TlsContext,
    /// TLS session resources.
    TlsSession,
    /// Device endpoint resources.
    Device,
    /// Pty resources.
    Pty,
    /// Tty resources.
    Tty,
    /// Sandbox resources.
    Sandbox,
    /// Inspector resources.
    Inspector,
    /// Profile resources.
    Profile,
    /// Trace resources.
    Trace,
    /// IPC transferred-handle resources.
    Transferred,
    /// Message queue resources.
    MessageQueue,
    /// Audio device resources.
    AudioDevice,
    /// Audio stream resources.
    AudioStream,
    /// Display resources.
    Display,
    /// Window resources.
    Window,
    /// Input device handle resources.
    Input,
    /// GPU adapter resources.
    GpuAdapter,
    /// GPU device resources.
    GpuDevice,
    /// GPU queue resources.
    GpuQueue,
    /// GPU command list resources.
    GpuCommandList,
    /// GPU memory resources.
    GpuMemory,
    /// GPU buffer resources.
    GpuBuffer,
    /// GPU texture resources.
    GpuTexture,
    /// GPU sampler resources.
    GpuSampler,
    /// GPU shader resources.
    GpuShader,
    /// GPU pipeline resources.
    GpuPipeline,
    /// Unknown resource kind.
    Unknown,
}

/// Finalizer callback for resource cleanup.
pub trait ResourceFinalizer: Send + Sync {
    /// Finalize the resource for the given id.
    fn finalize(self: Box<Self>, resource_id: ResourceId);
}

/// Entry stored in the resource table.
pub struct ResourceEntry {
    /// Resource classification.
    pub kind: ResourceKind,
    /// Optional label for diagnostics.
    pub label: Option<String>,
    /// Optional raw handle payload.
    #[cfg(windows)]
    pub raw_handle: Option<RawHandle>,
    /// Opaque payload for resource-specific state.
    pub payload: Option<Box<dyn Any + Send + Sync>>,
    /// Snapshot policy for this resource.
    pub snapshot_policy: ResourceSnapshotPolicy,
    /// Optional snapshot adapter for this resource.
    pub snapshot_adapter: Option<Box<dyn ResourceSnapshotAdapter>>,
    /// Optional finalizer invoked on removal.
    pub finalizer: Option<Box<dyn ResourceFinalizer>>,
}

impl fmt::Debug for ResourceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceEntry")
            .field("kind", &self.kind)
            .field("label", &self.label)
            .field("has_raw_handle", &{
                #[cfg(windows)]
                {
                    self.raw_handle.is_some()
                }
                #[cfg(not(windows))]
                {
                    false
                }
            })
            .field("has_payload", &self.payload.is_some())
            .field("snapshot_policy", &self.snapshot_policy)
            .field("has_snapshot_adapter", &self.snapshot_adapter.is_some())
            .field("has_finalizer", &self.finalizer.is_some())
            .finish()
    }
}

/// Send + Sync wrapper for raw handles on Windows.
#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
struct HandlePayload(
    /// Raw handle payload.
    RawHandle,
);

#[cfg(windows)]
unsafe impl Send for HandlePayload {}
#[cfg(windows)]
unsafe impl Sync for HandlePayload {}

impl ResourceEntry {
    /// Create a new resource entry for a kind.
    pub fn new(kind: ResourceKind) -> Self {
        Self {
            kind,
            label: None,
            #[cfg(windows)]
            raw_handle: None,
            payload: None,
            snapshot_policy: ResourceSnapshotPolicy::Uncheckpointable,
            snapshot_adapter: None,
            finalizer: None,
        }
    }

    /// Attach a diagnostic label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Attach a payload object.
    pub fn with_payload(mut self, payload: impl Any + Send + Sync) -> Self {
        self.payload = Some(Box::new(payload));
        self
    }

    /// Attach a snapshot policy.
    pub fn with_snapshot_policy(mut self, policy: ResourceSnapshotPolicy) -> Self {
        self.snapshot_policy = policy;
        self
    }

    /// Attach a snapshot adapter.
    pub fn with_snapshot_adapter(
        mut self,
        adapter: impl ResourceSnapshotAdapter + 'static,
    ) -> Self {
        self.snapshot_adapter = Some(Box::new(adapter));
        self
    }

    /// Attach a raw file descriptor payload.
    #[cfg(unix)]
    pub fn with_fd(mut self, fd: RawFd) -> Self {
        self.payload = Some(Box::new(fd));
        self
    }

    /// Attach a raw handle payload.
    #[cfg(windows)]
    pub fn with_handle(mut self, handle: RawHandle) -> Self {
        self.raw_handle = Some(handle);
        self
    }

    /// Attach a raw socket descriptor payload.
    #[cfg(unix)]
    pub fn with_socket(self, fd: RawFd) -> Self {
        self.with_fd(fd)
    }

    /// Attach a raw listener descriptor payload.
    #[cfg(unix)]
    pub fn with_listener(self, fd: RawFd) -> Self {
        self.with_fd(fd)
    }

    /// Attach a raw socket payload.
    #[cfg(windows)]
    pub fn with_socket(mut self, socket: RawSocket) -> Self {
        self.payload = Some(Box::new(socket));
        self
    }

    /// Attach a raw listener payload.
    #[cfg(windows)]
    pub fn with_listener(self, socket: RawSocket) -> Self {
        self.with_socket(socket)
    }

    /// Read a raw file descriptor payload when present.
    #[cfg(unix)]
    pub fn fd(&self) -> Option<RawFd> {
        self.payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<RawFd>())
            .copied()
    }

    /// Read a raw handle payload when present.
    #[cfg(windows)]
    pub fn handle(&self) -> Option<RawHandle> {
        self.raw_handle.or_else(|| {
            self.payload
                .as_ref()
                .and_then(|payload| payload.downcast_ref::<HandlePayload>())
                .map(|payload| payload.0)
        })
    }

    /// Read a raw socket payload when present.
    #[cfg(windows)]
    pub fn socket(&self) -> Option<RawSocket> {
        self.payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<RawSocket>())
            .copied()
    }

    /// Attach a resource finalizer.
    pub fn with_finalizer(mut self, finalizer: impl ResourceFinalizer + 'static) -> Self {
        self.finalizer = Some(Box::new(finalizer));
        self
    }

    /// Invoke the finalizer when present.
    pub fn finalize(self, resource_id: ResourceId) {
        if let Some(finalizer) = self.finalizer {
            finalizer.finalize(resource_id);
        }
    }
}

/// External resource table and finalizer registry.
#[derive(Debug)]
pub struct ResourceTable {
    /// Next resource identifier to allocate.
    next_id: AtomicU64,
    /// Stored resource entries.
    entries: RwLock<HashMap<ResourceId, ResourceEntry>>,
}

impl ResourceTable {
    /// Allocate and insert a resource entry.
    pub fn insert(&self, entry: ResourceEntry) -> ResourceId {
        let id = ResourceId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.entries.write().insert(id, entry);
        let _ = with_current_binding_call_context(|context| {
            context.hooks().on_resource_attach(RuntimeHookState {
                engine: Some(context.engine()),
                resource_id: Some(id),
                ..RuntimeHookState::empty()
            })
        });
        id
    }

    /// Insert a resource entry with an explicit id.
    pub fn insert_with_id(&self, resource_id: ResourceId, entry: ResourceEntry) {
        self.entries.write().insert(resource_id, entry);
        self.next_id.fetch_max(resource_id.0 + 1, Ordering::Relaxed);
        let _ = with_current_binding_call_context(|context| {
            context.hooks().on_resource_attach(RuntimeHookState {
                engine: Some(context.engine()),
                resource_id: Some(resource_id),
                ..RuntimeHookState::empty()
            })
        });
    }

    /// Return true if the table contains the resource id.
    pub fn contains(&self, resource_id: ResourceId) -> bool {
        self.entries.read().contains_key(&resource_id)
    }

    /// Run a closure with a read-only entry reference.
    pub fn with_entry<R>(
        &self,
        resource_id: ResourceId,
        f: impl FnOnce(&ResourceEntry) -> R,
    ) -> Option<R> {
        let entries = self.entries.read();
        let entry = entries.get(&resource_id)?;
        Some(f(entry))
    }

    /// Run a closure with a mutable entry reference.
    pub fn with_entry_mut<R>(
        &self,
        resource_id: ResourceId,
        f: impl FnOnce(&mut ResourceEntry) -> R,
    ) -> Option<R> {
        let mut entries = self.entries.write();
        let entry = entries.get_mut(&resource_id)?;
        Some(f(entry))
    }

    /// Remove a resource entry from the table.
    pub fn remove(&self, resource_id: ResourceId) -> Option<ResourceEntry> {
        let removed = self.entries.write().remove(&resource_id);
        if removed.is_some() {
            let _ = with_current_binding_call_context(|context| {
                context.hooks().on_resource_detach(RuntimeHookState {
                    engine: Some(context.engine()),
                    resource_id: Some(resource_id),
                    ..RuntimeHookState::empty()
                })
            });
        }

        removed
    }

    /// Remove a resource entry and run its finalizer.
    pub fn remove_and_finalize(&self, resource_id: ResourceId) -> bool {
        let Some(entry) = self.remove(resource_id) else {
            return false;
        };
        entry.finalize(resource_id);
        true
    }
}

impl Default for ResourceTable {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            entries: RwLock::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{ResourceEntry, ResourceFinalizer, ResourceKind, ResourceTable};

    /// Ensures entries can be inserted, removed, and finalized.
    #[test]
    fn test_insert_remove_and_finalize() {
        // create a new resource table
        let table = ResourceTable::default();

        // create a finalizer to track removals
        let hits = Arc::new(AtomicUsize::new(0));
        let finalizer = TestFinalizer { hits: hits.clone() };

        // insert an entry with a finalizer
        let entry = ResourceEntry::new(ResourceKind::Timer).with_finalizer(finalizer);
        let resource_id = table.insert(entry);
        assert!(table.contains(resource_id));

        // remove and finalize the entry
        let removed = table.remove_and_finalize(resource_id);
        assert!(removed);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(!table.contains(resource_id));

        // removing again should return false
        let removed_again = table.remove_and_finalize(resource_id);
        assert!(!removed_again);
    }

    struct TestFinalizer {
        hits: Arc<AtomicUsize>,
    }

    impl ResourceFinalizer for TestFinalizer {
        fn finalize(self: Box<Self>, _resource_id: super::ResourceId) {
            self.hits.fetch_add(1, Ordering::SeqCst);
        }
    }
}
