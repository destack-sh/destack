use std::any::Any;
use std::collections::HashMap;
use std::fmt;
#[cfg(unix)]
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;

use super::ResourceId;

/// Resource classification for platform handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    /// File handle resources.
    File,
    /// Directory handle resources.
    Directory,
    /// Socket handle resources.
    Socket,
    /// Listener handle resources.
    Listener,
    /// Timer handle resources.
    Timer,
    /// Process handle resources.
    Process,
    /// Unknown resource kind.
    Unknown,
}

/// Finalizer callback for resource cleanup.
pub trait ResourceFinalizer: Send {
    /// Finalize the resource for the given id.
    fn finalize(self: Box<Self>, resource_id: ResourceId);
}

/// Entry stored in the resource table.
pub struct ResourceEntry {
    /// Resource classification.
    pub kind: ResourceKind,
    /// Optional label for diagnostics.
    pub label: Option<String>,
    /// Opaque payload for resource-specific state.
    pub payload: Option<Box<dyn Any + Send + Sync>>,
    /// Optional finalizer invoked on removal.
    pub finalizer: Option<Box<dyn ResourceFinalizer>>,
}

impl fmt::Debug for ResourceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceEntry")
            .field("kind", &self.kind)
            .field("label", &self.label)
            .field("has_payload", &self.payload.is_some())
            .field("has_finalizer", &self.finalizer.is_some())
            .finish()
    }
}

impl ResourceEntry {
    /// Create a new resource entry for a kind.
    pub fn new(kind: ResourceKind) -> Self {
        Self {
            kind,
            label: None,
            payload: None,
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

    /// Attach a raw file descriptor payload.
    #[cfg(unix)]
    pub fn with_fd(mut self, fd: RawFd) -> Self {
        self.payload = Some(Box::new(fd));
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

    /// Read a raw file descriptor payload when present.
    #[cfg(unix)]
    pub fn fd(&self) -> Option<RawFd> {
        self.payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<RawFd>())
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
        id
    }

    /// Insert a resource entry with an explicit id.
    pub fn insert_with_id(&self, resource_id: ResourceId, entry: ResourceEntry) {
        self.entries.write().insert(resource_id, entry);
        self.next_id.fetch_max(resource_id.0 + 1, Ordering::Relaxed);
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
        self.entries.write().remove(&resource_id)
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
