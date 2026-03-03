use std::any::Any;
use std::fmt;
#[cfg(unix)]
use std::os::unix::io::RawFd;
#[cfg(windows)]
use std::os::windows::io::{RawHandle, RawSocket};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{ResourceId, ResourceSnapshotAdapter, ResourceSnapshotPolicy};
use crate::runtime::bindings::BindingEngine;
use crate::runtime::{HookState, Hooks};

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
    /// Crypto digest resources.
    CryptoDigest,
    /// Crypto mac resources.
    CryptoMac,
    /// Crypto cipher resources.
    CryptoCipher,
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
    /// Audio event subscription resources.
    AudioEvent,
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

impl ResourceKind {
    /// Return one stable runtime label for this resource kind.
    pub const fn label(self) -> &'static str {
        match self {
            ResourceKind::File => "file",
            ResourceKind::Directory => "directory",
            ResourceKind::Pipe => "pipe",
            ResourceKind::Socket => "socket",
            ResourceKind::Listener => "listener",
            ResourceKind::Timer => "timer",
            ResourceKind::TimerFd => "timer_fd",
            ResourceKind::Watch => "watch",
            ResourceKind::Process => "process",
            ResourceKind::Poll => "poll",
            ResourceKind::Completion => "completion",
            ResourceKind::Event => "event",
            ResourceKind::Uring => "uring",
            ResourceKind::ProcessFd => "process_fd",
            ResourceKind::SharedMemory => "shared_memory",
            ResourceKind::Semaphore => "semaphore",
            ResourceKind::Signal => "signal",
            ResourceKind::SignalFd => "signal_fd",
            ResourceKind::Thread => "thread",
            ResourceKind::Mutex => "mutex",
            ResourceKind::RwLock => "rw_lock",
            ResourceKind::CondVar => "cond_var",
            ResourceKind::ThreadSemaphore => "thread_semaphore",
            ResourceKind::Barrier => "barrier",
            ResourceKind::ThreadLocal => "thread_local",
            ResourceKind::Library => "library",
            ResourceKind::Symbol => "symbol",
            ResourceKind::CryptoStore => "crypto_store",
            ResourceKind::CryptoKey => "crypto_key",
            ResourceKind::CryptoCertificate => "crypto_certificate",
            ResourceKind::CryptoDigest => "crypto_digest",
            ResourceKind::CryptoMac => "crypto_mac",
            ResourceKind::CryptoCipher => "crypto_cipher",
            ResourceKind::TlsContext => "tls_context",
            ResourceKind::TlsSession => "tls_session",
            ResourceKind::Device => "device",
            ResourceKind::Pty => "pty",
            ResourceKind::Tty => "tty",
            ResourceKind::Sandbox => "sandbox",
            ResourceKind::Inspector => "inspector",
            ResourceKind::Profile => "profile",
            ResourceKind::Trace => "trace",
            ResourceKind::Transferred => "transferred",
            ResourceKind::MessageQueue => "message_queue",
            ResourceKind::AudioDevice => "audio_device",
            ResourceKind::AudioStream => "audio_stream",
            ResourceKind::AudioEvent => "audio_event",
            ResourceKind::Display => "display",
            ResourceKind::Window => "window",
            ResourceKind::Input => "input",
            ResourceKind::GpuAdapter => "gpu_adapter",
            ResourceKind::GpuDevice => "gpu_device",
            ResourceKind::GpuQueue => "gpu_queue",
            ResourceKind::GpuCommandList => "gpu_command_list",
            ResourceKind::GpuMemory => "gpu_memory",
            ResourceKind::GpuBuffer => "gpu_buffer",
            ResourceKind::GpuTexture => "gpu_texture",
            ResourceKind::GpuSampler => "gpu_sampler",
            ResourceKind::GpuShader => "gpu_shader",
            ResourceKind::GpuPipeline => "gpu_pipeline",
            ResourceKind::Unknown => "unknown",
        }
    }
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

    /// Create one labeled resource entry.
    pub fn labeled(kind: ResourceKind, label: impl Into<String>) -> Self {
        Self::new(kind).with_label(label)
    }

    /// Create one labeled resource entry with one typed payload.
    pub fn labeled_payload(
        kind: ResourceKind,
        label: impl Into<String>,
        payload: impl Any + Send + Sync,
    ) -> Self {
        Self::new(kind).with_label(label).with_payload(payload)
    }

    /// Create one labeled resource entry with one typed payload and finalizer.
    pub fn labeled_payload_finalizer(
        kind: ResourceKind,
        label: impl Into<String>,
        payload: impl Any + Send + Sync,
        finalizer: impl ResourceFinalizer + 'static,
    ) -> Self {
        Self::new(kind)
            .with_label(label)
            .with_payload(payload)
            .with_finalizer(finalizer)
    }

    /// Create one labeled resource entry with one finalizer.
    pub fn labeled_finalizer(
        kind: ResourceKind,
        label: impl Into<String>,
        finalizer: impl ResourceFinalizer + 'static,
    ) -> Self {
        Self::new(kind).with_label(label).with_finalizer(finalizer)
    }

    /// Create one labeled unix descriptor entry with one finalizer.
    #[cfg(unix)]
    pub fn labeled_fd_finalizer(
        kind: ResourceKind,
        label: impl Into<String>,
        descriptor: RawFd,
        finalizer: impl ResourceFinalizer + 'static,
    ) -> Self {
        Self::new(kind)
            .with_label(label)
            .with_fd(descriptor)
            .with_finalizer(finalizer)
    }

    /// Create one labeled windows handle entry with one finalizer.
    #[cfg(windows)]
    pub fn labeled_handle_finalizer(
        kind: ResourceKind,
        label: impl Into<String>,
        handle: RawHandle,
        finalizer: impl ResourceFinalizer + 'static,
    ) -> Self {
        Self::new(kind)
            .with_label(label)
            .with_handle(handle)
            .with_finalizer(finalizer)
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
        self.payload_ref::<RawFd>().copied()
    }

    /// Read a raw handle payload when present.
    #[cfg(windows)]
    pub fn handle(&self) -> Option<RawHandle> {
        self.raw_handle
            .or_else(|| self.payload_ref::<HandlePayload>().map(|payload| payload.0))
    }

    /// Read a raw socket payload when present.
    #[cfg(windows)]
    pub fn socket(&self) -> Option<RawSocket> {
        self.payload_ref::<RawSocket>().copied()
    }

    /// Read one typed payload reference when present.
    pub fn payload_ref<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<T>())
    }

    /// Read one mutable typed payload reference when present.
    pub fn payload_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<T>())
    }

    /// Read one cloned typed payload value when present.
    pub fn payload_cloned<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.payload_ref::<T>().cloned()
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
    /// Mutable slot table for all resources.
    inner: RwLock<ResourceTableInner>,
    /// Runtime hooks sink for non-binding resource mutations.
    hooks: RwLock<Option<Arc<Hooks>>>,
}

/// Mutable slot table payload.
#[derive(Debug)]
struct ResourceTableInner {
    /// Allocated slots indexed by resource slot id minus one.
    slots: Vec<ResourceSlot>,
    /// Reusable slot ids with no active entry.
    free_slots: Vec<u32>,
    /// Next slot id for first-use allocations.
    next_slot: u32,
}

/// One slot payload in the resource table.
#[derive(Debug)]
struct ResourceSlot {
    /// Current generation stamp for this slot.
    generation: u32,
    /// Active entry payload when one is present.
    entry: Option<ResourceEntry>,
}

impl ResourceTable {
    /// Configure one runtime hooks sink for resource lifecycle hooks.
    pub fn set_hooks(&self, hooks: Arc<Hooks>) {
        *self.hooks.write() = Some(hooks);
    }

    /// Number of bits used for one resource slot id.
    const SLOT_BITS: u64 = 32;
    /// Mask for one packed resource slot id.
    const SLOT_MASK: u64 = (1u64 << Self::SLOT_BITS) - 1;

    /// Decode one resource id into slot and generation selectors.
    fn decode_resource_id(resource_id: ResourceId) -> Option<(u32, u32)> {
        let slot = (resource_id.0 & Self::SLOT_MASK) as u32;
        let generation = (resource_id.0 >> Self::SLOT_BITS) as u32;
        if slot == 0 {
            return None;
        }

        Some((slot, generation))
    }

    /// Encode one slot and generation selector into one resource id.
    fn encode_resource_id(slot: u32, generation: u32) -> ResourceId {
        let value = (u64::from(generation) << Self::SLOT_BITS) | u64::from(slot);
        ResourceId(value)
    }

    /// Return one mutable slot reference for one decoded slot id.
    fn slot_mut(inner: &mut ResourceTableInner, slot: u32) -> Option<&mut ResourceSlot> {
        let index = slot.checked_sub(1)? as usize;
        inner.slots.get_mut(index)
    }

    /// Return one read-only slot reference for one decoded slot id.
    fn slot(inner: &ResourceTableInner, slot: u32) -> Option<&ResourceSlot> {
        let index = slot.checked_sub(1)? as usize;
        inner.slots.get(index)
    }

    /// Return one next generation value after one removal.
    fn next_generation(generation: u32) -> u32 {
        generation.wrapping_add(1)
    }

    /// Allocate and insert a resource entry.
    pub fn insert(&self, entry: ResourceEntry, engine: Option<BindingEngine>) -> ResourceId {
        let mut inner = self.inner.write();

        let (resource_id, slot_index) = if let Some(slot) = inner.free_slots.pop() {
            let id = Self::slot(&inner, slot)
                .map(|value| Self::encode_resource_id(slot, value.generation))
                .unwrap_or_else(|| panic!("resource table free slot {slot} is missing"));
            let index = slot
                .checked_sub(1)
                .unwrap_or_else(|| panic!("resource table free slot {slot} is invalid"))
                as usize;
            (id, index)
        } else {
            let slot = inner.next_slot;
            inner.next_slot = inner
                .next_slot
                .checked_add(1)
                .unwrap_or_else(|| panic!("resource table exhausted all slot ids"));
            inner.slots.push(ResourceSlot {
                generation: 0,
                entry: None,
            });
            let id = Self::encode_resource_id(slot, 0);
            let index = slot
                .checked_sub(1)
                .unwrap_or_else(|| panic!("resource table allocated invalid slot id {slot}"))
                as usize;
            (id, index)
        };

        let slot = inner
            .slots
            .get_mut(slot_index)
            .unwrap_or_else(|| panic!("resource table missing slot {slot_index}"));
        slot.entry = Some(entry);
        drop(inner);

        self.emit_resource_attach(resource_id, engine);

        resource_id
    }

    /// Insert a resource entry with an explicit id.
    pub fn insert_with_id(
        &self,
        resource_id: ResourceId,
        entry: ResourceEntry,
        engine: Option<BindingEngine>,
    ) {
        let (slot, generation) = Self::decode_resource_id(resource_id)
            .unwrap_or_else(|| panic!("resource table insert_with_id received invalid id 0"));

        let mut inner = self.inner.write();
        let required_len = slot as usize;
        if inner.slots.len() < required_len {
            inner.slots.resize_with(required_len, || ResourceSlot {
                generation: 0,
                entry: None,
            });
        }

        let slot_entry = Self::slot_mut(&mut inner, slot)
            .unwrap_or_else(|| panic!("resource table missing slot {slot}"));
        slot_entry.generation = generation;
        slot_entry.entry = Some(entry);
        inner.free_slots.retain(|value| *value != slot);
        inner.next_slot = inner.next_slot.max(slot.saturating_add(1));
        drop(inner);

        self.emit_resource_attach(resource_id, engine);
    }

    /// Return true if the table contains the resource id.
    pub fn contains(&self, resource_id: ResourceId) -> bool {
        let Some((slot, generation)) = Self::decode_resource_id(resource_id) else {
            return false;
        };

        let inner = self.inner.read();
        let Some(slot_entry) = Self::slot(&inner, slot) else {
            return false;
        };

        slot_entry.generation == generation && slot_entry.entry.is_some()
    }

    /// Run a closure with a read-only entry reference.
    pub fn with_entry<R>(
        &self,
        resource_id: ResourceId,
        f: impl FnOnce(&ResourceEntry) -> R,
    ) -> Option<R> {
        let (slot, generation) = Self::decode_resource_id(resource_id)?;
        let inner = self.inner.read();
        let slot_entry = Self::slot(&inner, slot)?;
        if slot_entry.generation != generation {
            return None;
        }
        let entry = slot_entry.entry.as_ref()?;

        Some(f(entry))
    }

    /// Run a closure with a mutable entry reference.
    pub fn with_entry_mut<R>(
        &self,
        resource_id: ResourceId,
        f: impl FnOnce(&mut ResourceEntry) -> R,
    ) -> Option<R> {
        let (slot, generation) = Self::decode_resource_id(resource_id)?;
        let mut inner = self.inner.write();
        let slot_entry = Self::slot_mut(&mut inner, slot)?;
        if slot_entry.generation != generation {
            return None;
        }
        let entry = slot_entry.entry.as_mut()?;

        Some(f(entry))
    }

    /// Remove a resource entry from the table.
    pub fn remove(
        &self,
        resource_id: ResourceId,
        engine: Option<BindingEngine>,
    ) -> Option<ResourceEntry> {
        let (slot, generation) = Self::decode_resource_id(resource_id)?;
        let mut inner = self.inner.write();
        let slot_entry = Self::slot_mut(&mut inner, slot)?;
        if slot_entry.generation != generation {
            return None;
        }

        let removed = slot_entry.entry.take();
        if removed.is_some() {
            slot_entry.generation = Self::next_generation(slot_entry.generation);
            inner.free_slots.push(slot);
        }
        drop(inner);

        if removed.is_some() {
            self.emit_resource_detach(resource_id, engine);
        }

        removed
    }

    /// Emit one resource-attach hook through the shared runtime hook sink.
    fn emit_resource_attach(&self, resource_id: ResourceId, engine: Option<BindingEngine>) {
        if let Some(hooks) = self.hooks.read().as_ref().cloned() {
            hooks.on_resource_attach(HookState {
                engine,
                resource_id: Some(resource_id),
                ..HookState::empty()
            });
        }
    }

    /// Emit one resource-detach hook through the shared runtime hook sink.
    fn emit_resource_detach(&self, resource_id: ResourceId, engine: Option<BindingEngine>) {
        if let Some(hooks) = self.hooks.read().as_ref().cloned() {
            hooks.on_resource_detach(HookState {
                engine,
                resource_id: Some(resource_id),
                ..HookState::empty()
            });
        }
    }

    /// Remove a resource entry and run its finalizer.
    pub fn remove_and_finalize(
        &self,
        resource_id: ResourceId,
        engine: Option<BindingEngine>,
    ) -> bool {
        let Some(entry) = self.remove(resource_id, engine) else {
            return false;
        };
        entry.finalize(resource_id);
        true
    }
}

impl Default for ResourceTable {
    fn default() -> Self {
        Self {
            inner: RwLock::new(ResourceTableInner {
                slots: Vec::new(),
                free_slots: Vec::new(),
                next_slot: 1,
            }),
            hooks: RwLock::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{ResourceEntry, ResourceFinalizer, ResourceId, ResourceKind, ResourceTable};

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
        let resource_id = table.insert(entry, None);
        assert!(table.contains(resource_id));

        // remove and finalize the entry
        let removed = table.remove_and_finalize(resource_id, None);
        assert!(removed);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(!table.contains(resource_id));

        // removing again should return false
        let removed_again = table.remove_and_finalize(resource_id, None);
        assert!(!removed_again);
    }

    /// Ensures stale handles are rejected after slot reuse.
    #[test]
    fn test_rejects_stale_handle_after_reuse() {
        // create a new resource table
        let table = ResourceTable::default();

        // allocate one handle and then remove it
        let first = table.insert(ResourceEntry::new(ResourceKind::Timer), None);
        assert!(table.remove(first, None).is_some());
        assert!(!table.contains(first));

        // allocate one new handle and ensure the stale handle is rejected
        let second = table.insert(ResourceEntry::new(ResourceKind::Timer), None);
        assert_ne!(first, second);
        assert!(!table.contains(first));
        assert!(table.contains(second));
    }

    /// Ensures explicit id insertion respects generation checks.
    #[test]
    fn test_insert_with_id_honors_generation() {
        // create a new resource table
        let table = ResourceTable::default();
        let slot = ResourceId(7);
        let generation = ResourceId((3u64 << 32) | 7u64);

        // insert one legacy id and then one generated id in the same slot
        table.insert_with_id(slot, ResourceEntry::new(ResourceKind::Timer), None);
        assert!(table.contains(slot));
        table.remove(slot, None);
        table.insert_with_id(generation, ResourceEntry::new(ResourceKind::Timer), None);

        // stale and current ids should resolve as expected
        assert!(!table.contains(slot));
        assert!(table.contains(generation));
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
