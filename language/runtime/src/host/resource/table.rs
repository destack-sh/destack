use std::any::Any;
use std::collections::HashMap;
use std::fmt;
#[cfg(windows)]
use std::os::windows::io::RawHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tspp_core::{Capture, CaptureMode};

use super::{
    ResourceAffinity, ResourceId, ResourceImageEntry, ResourceKind, ResourceProvider,
    ResourceRebinders, ResourceRestore,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::worker::WorkerId;

/// Captured resource-table state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceImage {
    /// The next worker-local resource sequence to allocate.
    pub next_sequence: u64,
    /// Captured resource entries keyed by table id.
    pub entries: Vec<ResourceImageEntry>,
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
    /// Restore model for this resource.
    pub restore: ResourceRestore,
    /// Optional execution-affinity requirement for this resource.
    pub affinity: Option<ResourceAffinity>,
    /// Optional raw handle payload.
    #[cfg(windows)]
    pub raw_handle: Option<RawHandle>,
    /// Opaque payload for resource-specific state.
    pub payload: Option<Box<dyn Any + Send + Sync>>,
    /// Optional provider for this resource.
    pub provider: Option<Arc<dyn ResourceProvider>>,
    /// Optional finalizer invoked on removal.
    pub finalizer: Option<Box<dyn ResourceFinalizer>>,
}

impl fmt::Debug for ResourceEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceEntry")
            .field("kind", &self.kind)
            .field("label", &self.label)
            .field("restore", &self.restore)
            .field("affinity", &self.affinity)
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
            .field("has_provider", &self.provider.is_some())
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
            restore: ResourceRestore::None,
            affinity: None,
            #[cfg(windows)]
            raw_handle: None,
            payload: None,
            provider: None,
            finalizer: None,
        }
    }

    /// Create one labeled resource entry.
    pub fn labeled(kind: ResourceKind, label: impl Into<String>) -> Self {
        Self::new(kind).with_label(label)
    }

    /// Attach a diagnostic label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Attach one explicit restore model.
    pub fn with_restore(mut self, restore: ResourceRestore) -> Self {
        self.restore = restore;
        self
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
pub struct ResourceTable {
    /// Worker that owns this resource table.
    worker_id: WorkerId,
    /// Next worker-local resource sequence to allocate.
    next_sequence: AtomicU64,
    /// Stored resource entries.
    entries: RwLock<HashMap<ResourceId, ResourceEntry>>,
    /// Registered resource providers keyed by resource kind.
    providers: RwLock<HashMap<ResourceKind, Arc<dyn ResourceProvider>>>,
}

impl fmt::Debug for ResourceTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry_count = self.entries.read().len();
        let provider_count = self.providers.read().len();

        f.debug_struct("ResourceTable")
            .field("worker_id", &self.worker_id)
            .field("next_sequence", &self.next_sequence.load(Ordering::Relaxed))
            .field("entry_count", &entry_count)
            .field("provider_count", &provider_count)
            .finish()
    }
}

impl ResourceTable {
    /// Create one resource table owned by one worker.
    pub fn new(worker_id: WorkerId) -> Self {
        Self {
            worker_id,
            next_sequence: AtomicU64::new(1),
            entries: RwLock::new(HashMap::new()),
            providers: RwLock::new(HashMap::new()),
        }
    }

    /// Allocate one new resource identifier.
    fn allocate_id(&self) -> RuntimeResult<ResourceId> {
        let local_id = self
            .next_sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| {
                RuntimeError::Internal {
                    message: "resource identifier space exhausted".to_string(),
                }
                .boxed()
            })?;

        Ok(ResourceId::new(self.worker_id, local_id))
    }

    /// Register one resource provider for a resource kind.
    pub fn register_provider(
        &self,
        resource_kind: ResourceKind,
        provider: Arc<dyn ResourceProvider>,
    ) -> RuntimeResult<()> {
        let mut providers = self.providers.write();
        if providers.contains_key(&resource_kind) {
            return Err(RuntimeError::Internal {
                message: format!("resource provider for {resource_kind:?} is already registered"),
            }
            .boxed());
        }
        providers.insert(resource_kind, provider);

        Ok(())
    }

    /// Return the number of stored resources.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Return whether the table holds no resources.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Fork one quiescent resource table for one child worker.
    pub(crate) fn try_fork(&self) -> Option<Self> {
        // direct live fork only supports empty live resource tables
        if !self.entries.read().is_empty() {
            return None;
        }

        // clone shared providers and allocator state onto one fresh table
        let forked = Self {
            worker_id: self.worker_id,
            next_sequence: AtomicU64::new(self.next_sequence.load(Ordering::Relaxed)),
            entries: RwLock::new(HashMap::new()),
            providers: RwLock::new(self.providers.read().clone()),
        };

        Some(forked)
    }

    /// Allocate and insert a resource entry.
    pub fn insert(&self, entry: ResourceEntry) -> RuntimeResult<ResourceId> {
        let id = self.allocate_id()?;
        self.entries.write().insert(id, entry);

        Ok(id)
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

    // restore barrier
    fn restore_barrier(&self) -> RuntimeResult<()> {
        if self.entries.read().is_empty() {
            return Ok(());
        }

        Err(RuntimeError::Internal {
            message: "resource table restore requires one empty live table".to_string(),
        }
        .boxed())
    }

    /// Require one structurally valid resource image for this worker.
    fn validate_image(&self, image: &ResourceImage) -> RuntimeResult<()> {
        // require the first allocatable sequence
        if image.next_sequence == 0 {
            return Err(RuntimeError::Internal {
                message: "resource image has an invalid next sequence".to_string(),
            }
            .boxed());
        }

        let mut resource_ids = HashMap::with_capacity(image.entries.len());
        for entry in &image.entries {
            // require worker ownership
            if entry.resource_id.worker_id != self.worker_id {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "resource {} belongs to worker {}, expected worker {}",
                        entry.resource_id.local_id, entry.resource_id.worker_id.0, self.worker_id.0,
                    ),
                }
                .boxed());
            }

            // require one previously allocated sequence
            if entry.resource_id.local_id == 0 || entry.resource_id.local_id >= image.next_sequence
            {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "resource {} exceeds the captured resource sequence {}",
                        entry.resource_id.local_id, image.next_sequence,
                    ),
                }
                .boxed());
            }

            // require one unique table identity
            if resource_ids.insert(entry.resource_id, ()).is_some() {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "resource {} appears more than once in the captured resource table",
                        entry.resource_id.local_id,
                    ),
                }
                .boxed());
            }
        }

        Ok(())
    }

    // capture one attached resource entry
    fn capture_entry(
        &self,
        resource_id: ResourceId,
        entry: &ResourceEntry,
    ) -> RuntimeResult<ResourceImageEntry> {
        let snapshot = match entry.restore {
            ResourceRestore::None => {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "resource {} of kind {:?} does not support capture",
                        resource_id.local_id, entry.kind
                    ),
                }
                .boxed());
            }
            ResourceRestore::State | ResourceRestore::Rebind => {
                let provider = entry
                    .provider
                    .clone()
                    .or_else(|| self.providers.read().get(&entry.kind).cloned());
                let Some(provider) = provider else {
                    return Err(RuntimeError::Internal {
                        message: format!(
                            "resource {} of kind {:?} is missing one resource provider",
                            resource_id.local_id, entry.kind
                        ),
                    }
                    .boxed());
                };

                Some(provider.snapshot(resource_id).map_err(|error| {
                    RuntimeError::Internal {
                        message: format!(
                            "resource {} of kind {:?} failed to capture: {error}",
                            resource_id.local_id, entry.kind
                        ),
                    }
                    .boxed()
                })?)
            }
        };

        Ok(ResourceImageEntry {
            resource_id,
            kind: entry.kind,
            label: entry.label.clone(),
            restore: entry.restore,
            affinity: entry.affinity,
            snapshot,
        })
    }

    /// Restore one captured resource entry.
    fn restore_entry(
        &self,
        image: &ResourceImageEntry,
        rebinders: Option<&ResourceRebinders>,
    ) -> RuntimeResult<ResourceEntry> {
        // reconstruct the resource according to its capture model
        let mut entry = match image.restore {
            ResourceRestore::None => ResourceEntry::new(image.kind),
            ResourceRestore::State | ResourceRestore::Rebind => {
                let snapshot = image.snapshot.as_ref().ok_or_else(|| {
                    RuntimeError::Internal {
                        message: format!(
                            "resource {} of kind {:?} is missing one captured payload",
                            image.resource_id.local_id, image.kind
                        ),
                    }
                    .boxed()
                })?;

                if image.restore == ResourceRestore::Rebind {
                    let rebinder = rebinders
                        .and_then(|context| context.rebinder(image.kind))
                        .ok_or_else(|| {
                            RuntimeError::Internal {
                                message: format!(
                                    "resource {} of kind {:?} requires one external rebinding hook",
                                    image.resource_id.local_id, image.kind
                                ),
                            }
                            .boxed()
                        })?;

                    rebinder.rebind(snapshot).map_err(|error| {
                        RuntimeError::Internal {
                            message: format!(
                                "resource {} of kind {:?} failed to rebind: {error}",
                                image.resource_id.local_id, image.kind
                            ),
                        }
                        .boxed()
                    })?
                } else {
                    let provider =
                        self.providers
                            .read()
                            .get(&image.kind)
                            .cloned()
                            .ok_or_else(|| {
                                RuntimeError::Internal {
                                    message: format!(
                                        "resource {} of kind {:?} is missing one restore provider",
                                        image.resource_id.local_id, image.kind
                                    ),
                                }
                                .boxed()
                            })?;

                    provider.restore(snapshot).map_err(|error| {
                        RuntimeError::Internal {
                            message: format!(
                                "resource {} of kind {:?} failed to restore: {error}",
                                image.resource_id.local_id, image.kind
                            ),
                        }
                        .boxed()
                    })?
                }
            }
        };

        // restore runtime-owned resource metadata
        entry.kind = image.kind;
        entry.label = image.label.clone();
        entry.restore = image.restore;
        entry.affinity = image.affinity;

        Ok(entry)
    }

    /// Finalize every resource reconstructed before one failed restore.
    fn finalize_entries(entries: HashMap<ResourceId, ResourceEntry>) {
        for (resource_id, entry) in entries {
            entry.finalize(resource_id);
        }
    }

    /// Capture one resource-table image.
    pub(crate) fn image(&self, _mode: CaptureMode) -> RuntimeResult<ResourceImage> {
        let entries = self.entries.read();
        let entries = entries
            .iter()
            .map(|(resource_id, entry)| self.capture_entry(*resource_id, entry))
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(ResourceImage {
            next_sequence: self.next_sequence.load(Ordering::Relaxed),
            entries,
        })
    }

    /// Restore one resource-table image.
    pub(crate) fn restore(
        &self,
        image: &ResourceImage,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        self.restore_barrier()?;
        self.validate_image(image)?;

        // restore every entry before publishing replacement state
        let mut restored = HashMap::with_capacity(image.entries.len());
        for image_entry in &image.entries {
            let entry = match self.restore_entry(image_entry, rebind_context) {
                Ok(entry) => entry,
                Err(error) => {
                    Self::finalize_entries(restored);

                    return Err(error);
                }
            };
            restored.insert(image_entry.resource_id, entry);
        }

        // publish the complete restored table
        *self.entries.write() = restored;
        self.next_sequence
            .store(image.next_sequence, Ordering::Relaxed);

        Ok(())
    }
}

impl Capture for ResourceTable {
    type Image = ResourceImage;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = Option<&'a ResourceRebinders>;

    /// Capture one resource-table image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.image(mode)
    }

    /// Restore one resource-table image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore(image, context)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tspp_core::CaptureMode;

    use super::ResourceId;
    use crate::diagnostic::HostError;
    use crate::host::resource::{
        ResourceRebinder, ResourceRebinders, ResourceRestore, ResourceSnapshot,
    };

    use super::{ResourceEntry, ResourceFinalizer, ResourceKind, ResourceProvider, ResourceTable};
    use crate::worker::WorkerId;

    const TEST_WORKER_ID: WorkerId = WorkerId(1);

    fn test_resource_id(local_id: u64) -> ResourceId {
        ResourceId::new(TEST_WORKER_ID, local_id)
    }

    fn test_resource_table() -> ResourceTable {
        ResourceTable::new(TEST_WORKER_ID)
    }

    /// Ensures entries can be inserted, removed, and finalized.
    #[test]
    fn test_insert_remove_and_finalize() {
        // create a new resource table
        let table = test_resource_table();
        // create a finalizer to track removals
        let hits = Arc::new(AtomicUsize::new(0));
        let finalizer = TestFinalizer { hits: hits.clone() };

        // insert an entry with a finalizer
        let entry = ResourceEntry::new(ResourceKind::Timer).with_finalizer(finalizer);
        let resource_id = table.insert(entry).expect("insert resource");
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

    /// Resource images should restore provider-backed entries.
    #[test]
    fn test_restore_resource_image_with_provider() {
        let table = test_resource_table();
        let provider = Arc::new(TestResourceProvider);
        table
            .register_provider(ResourceKind::Timer, provider)
            .expect("register resource provider");

        let entry = ResourceEntry::new(ResourceKind::Timer).with_restore(ResourceRestore::State);
        let resource_id = table.insert(entry).expect("insert resource");

        let image = table
            .image(CaptureMode::Fork)
            .expect("capture resource table");
        let restored = test_resource_table();
        restored
            .register_provider(ResourceKind::Timer, Arc::new(TestResourceProvider))
            .expect("register resource provider");
        restored
            .restore(&image, None)
            .expect("restore resource table");

        assert!(restored.contains(resource_id));
        let kind = restored.with_entry(resource_id, |entry| entry.kind);
        assert_eq!(kind, Some(ResourceKind::Timer));
    }

    /// External resource images should require explicit rebinding on restore.
    #[test]
    fn test_restore_resource_image_requires_rebinding() {
        let table = test_resource_table();
        let provider = Arc::new(TestResourceProvider);
        table
            .register_provider(ResourceKind::Timer, provider)
            .expect("register resource provider");

        let entry = ResourceEntry::new(ResourceKind::Timer).with_restore(ResourceRestore::Rebind);
        table.insert(entry).expect("insert resource");

        let image = table
            .image(CaptureMode::Hibernate)
            .expect("external resources should capture with one recipe");
        let restored = test_resource_table();
        let error = restored
            .restore(&image, None)
            .expect_err("external resources should require rebinding");
        let message = error.to_string();

        assert!(
            message.contains("requires one external rebinding hook"),
            "unexpected resource restore error: {message}"
        );

        let mut rebind_context = ResourceRebinders::default();
        rebind_context.register(ResourceKind::Timer, Arc::new(TestResourceRebinder));

        restored
            .restore(&image, Some(&rebind_context))
            .expect("external resources should restore with rebinding");
        assert_eq!(
            restored.with_entry(test_resource_id(1), |entry| entry.kind),
            Some(ResourceKind::Timer)
        );
    }

    struct TestFinalizer {
        hits: Arc<AtomicUsize>,
    }

    impl ResourceFinalizer for TestFinalizer {
        fn finalize(self: Box<Self>, _resource_id: ResourceId) {
            self.hits.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct TestResourceProvider;

    impl ResourceProvider for TestResourceProvider {
        fn snapshot(&self, resource_id: ResourceId) -> Result<ResourceSnapshot, Box<HostError>> {
            Ok(ResourceSnapshot::Snapshot {
                resource_id,
                kind: ResourceKind::Timer,
                payload: vec![1, 2, 3],
            })
        }

        fn restore(&self, snapshot: &ResourceSnapshot) -> Result<ResourceEntry, Box<HostError>> {
            let _ = snapshot;

            Ok(ResourceEntry::new(ResourceKind::Timer).with_restore(ResourceRestore::State))
        }
    }

    struct TestResourceRebinder;

    impl ResourceRebinder for TestResourceRebinder {
        fn rebind(&self, snapshot: &ResourceSnapshot) -> Result<ResourceEntry, Box<HostError>> {
            let _ = snapshot;

            Ok(ResourceEntry::new(ResourceKind::Timer).with_restore(ResourceRestore::Rebind))
        }
    }
}
