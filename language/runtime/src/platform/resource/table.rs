use std::any::Any;
use std::collections::HashMap;
use std::fmt;
#[cfg(unix)]
use std::os::unix::io::RawFd;
#[cfg(windows)]
use std::os::windows::io::{RawHandle, RawSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_core::{Capture, CaptureMode};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::error;

use super::{
    ResourceAffinity, ResourceBacking, ResourceCapture, ResourceHandle, ResourceId,
    ResourceImageEntry, ResourceKind, ResourcePortability, ResourceProvider, ResourceRebinders,
    ResourceRoute,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::Hooks;
use crate::runtime::bindings::{BindingAffinity, BindingEngine};
use crate::runtime::world::WorldScope;

/// Durable resource-table state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceTableSnapshot {
    /// The next resource identifier to allocate.
    pub next_id: u64,
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
    /// Optional typed route metadata.
    pub route: Option<ResourceRoute>,
    /// Backing model for this resource.
    pub backing: ResourceBacking,
    /// Capture model for this resource.
    pub capture: ResourceCapture,
    /// Portability model for this resource.
    pub portability: ResourcePortability,
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
            .field("route", &self.route)
            .field("backing", &self.backing)
            .field("capture", &self.capture)
            .field("portability", &self.portability)
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
            route: None,
            backing: ResourceBacking::Host,
            capture: ResourceCapture::None,
            portability: ResourcePortability::Local,
            affinity: None,
            #[cfg(windows)]
            raw_handle: None,
            payload: None,
            provider: None,
            finalizer: None,
        }
    }

    /// Create a new resource entry for one typed handle.
    pub fn for_handle<H: ResourceHandle>() -> Self {
        Self::new(H::KIND)
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

    /// Attach one typed route tag.
    pub fn with_route(mut self, route: ResourceRoute) -> Self {
        self.route = Some(route);
        self
    }

    /// Attach one explicit resource-affinity requirement.
    pub fn with_affinity(mut self, affinity: ResourceAffinity) -> Self {
        self.affinity = Some(affinity);
        self
    }

    /// Attach one explicit resource backing model.
    pub fn with_backing(mut self, backing: ResourceBacking) -> Self {
        self.backing = backing;
        self
    }

    /// Attach one explicit resource capture model.
    pub fn with_capture(mut self, capture: ResourceCapture) -> Self {
        self.capture = capture;
        self
    }

    /// Attach one explicit resource portability model.
    pub fn with_portability(mut self, portability: ResourcePortability) -> Self {
        self.portability = portability;
        self
    }

    /// Attach one resource-affinity requirement derived from binding metadata.
    pub fn with_binding_affinity(mut self, affinity: BindingAffinity) -> Self {
        self.affinity = ResourceAffinity::from_binding_affinity(affinity);
        self
    }

    /// Attach a payload object.
    pub fn with_payload(mut self, payload: impl Any + Send + Sync) -> Self {
        self.payload = Some(Box::new(payload));
        self
    }

    /// Attach one resource provider.
    pub fn with_provider(mut self, provider: Arc<dyn ResourceProvider>) -> Self {
        self.provider = Some(provider);
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

    /// Return one typed payload reference when present.
    pub fn payload_ref<T: 'static>(&self) -> Option<&T> {
        self.payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<T>())
    }

    /// Return one typed mutable payload reference when present.
    pub fn payload_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.payload
            .as_mut()
            .and_then(|payload| payload.downcast_mut::<T>())
    }

    /// Return one cloned typed payload when present.
    pub fn payload_cloned<T: Clone + 'static>(&self) -> Option<T> {
        self.payload_ref::<T>().cloned()
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
    /// Next resource identifier to allocate.
    next_id: AtomicU64,
    /// Stored resource entries.
    entries: RwLock<HashMap<ResourceId, ResourceEntry>>,
    /// Registered resource providers keyed by resource kind.
    providers: RwLock<HashMap<ResourceKind, Arc<dyn ResourceProvider>>>,
    /// Runtime hooks sink for non-binding resource mutations.
    hooks: RwLock<Option<Arc<Hooks>>>,
}

impl fmt::Debug for ResourceTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry_count = self.entries.read().len();
        let provider_count = self.providers.read().len();

        f.debug_struct("ResourceTable")
            .field("next_id", &self.next_id.load(Ordering::Relaxed))
            .field("entry_count", &entry_count)
            .field("provider_count", &provider_count)
            .field("has_hooks", &self.hooks.read().is_some())
            .finish()
    }
}

impl ResourceTable {
    /// Register one resource provider for a resource kind.
    pub fn register_provider(
        &self,
        resource_kind: ResourceKind,
        provider: Arc<dyn ResourceProvider>,
    ) {
        let _ = self.providers.write().insert(resource_kind, provider);
    }

    /// Configure one runtime hooks sink for resource lifecycle hooks.
    pub fn set_hooks(&self, hooks: Arc<Hooks>) {
        *self.hooks.write() = Some(hooks);
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
    pub(crate) fn try_fork(&self, hooks: Arc<Hooks>) -> Option<Self> {
        // direct live fork only supports empty live resource tables
        if !self.entries.read().is_empty() {
            return None;
        }

        // clone shared providers and allocator state onto one fresh table
        let forked = Self {
            next_id: AtomicU64::new(self.next_id.load(Ordering::Relaxed)),
            entries: RwLock::new(HashMap::new()),
            providers: RwLock::new(self.providers.read().clone()),
            hooks: RwLock::new(Some(hooks)),
        };

        Some(forked)
    }

    /// Allocate and insert a resource entry.
    pub(crate) fn insert(
        &self,
        world: &WorldScope,
        entry: ResourceEntry,
        engine: Option<BindingEngine>,
    ) -> ResourceId {
        // allocate one new id and persist the entry first
        let id = ResourceId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let resource_kind = entry.kind;
        let resource_label = entry.label.clone();
        let resource_backing = entry.backing;
        let resource_capture = entry.capture;
        let resource_portability = entry.portability;
        self.entries.write().insert(id, entry);

        // notify runtime hooks about the new resource
        if let Some(hooks) = self.hooks.read().as_ref().cloned()
            && let Err(error) = hooks.on_resource_attach(
                world,
                id,
                resource_kind,
                resource_label.as_deref(),
                resource_backing,
                resource_capture,
                resource_portability,
                engine,
            )
        {
            error!(?error, "resource attach hook failed");
        }

        id
    }

    /// Allocate and insert one resource entry outside world hooks.
    pub(crate) fn insert_untracked(&self, entry: ResourceEntry) -> ResourceId {
        // resource table
        let id = ResourceId(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.entries.write().insert(id, entry);

        id
    }

    /// Insert a resource entry with an explicit id.
    pub(crate) fn insert_with_id(
        &self,
        world: &WorldScope,
        resource_id: ResourceId,
        entry: ResourceEntry,
        engine: Option<BindingEngine>,
    ) {
        // persist the entry and advance the allocator when needed
        let resource_kind = entry.kind;
        let resource_label = entry.label.clone();
        let resource_backing = entry.backing;
        let resource_capture = entry.capture;
        let resource_portability = entry.portability;
        self.entries.write().insert(resource_id, entry);
        self.next_id.fetch_max(resource_id.0 + 1, Ordering::Relaxed);

        // notify runtime hooks about the restored resource
        if let Some(hooks) = self.hooks.read().as_ref().cloned()
            && let Err(error) = hooks.on_resource_attach(
                world,
                resource_id,
                resource_kind,
                resource_label.as_deref(),
                resource_backing,
                resource_capture,
                resource_portability,
                engine,
            )
        {
            error!(?error, "resource attach hook failed");
        }
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
    pub(crate) fn remove(
        &self,
        world: &WorldScope,
        resource_id: ResourceId,
        engine: Option<BindingEngine>,
    ) -> Option<ResourceEntry> {
        // remove the entry before notifying hooks
        let removed = self.entries.write().remove(&resource_id);
        if let Some(entry) = removed.as_ref() {
            // notify runtime hooks about the removed resource
            if let Some(hooks) = self.hooks.read().as_ref().cloned()
                && let Err(error) = hooks.on_resource_detach(
                    world,
                    resource_id,
                    entry.kind,
                    entry.label.as_deref(),
                    engine,
                )
            {
                error!(?error, "resource detach hook failed");
            }
        }

        removed
    }

    /// Remove a resource entry and run its finalizer.
    pub(crate) fn remove_and_finalize(
        &self,
        world: &WorldScope,
        resource_id: ResourceId,
        engine: Option<BindingEngine>,
    ) -> bool {
        let Some(entry) = self.remove(world, resource_id, engine) else {
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

    // capture one attached resource entry
    fn capture_entry(
        &self,
        _mode: CaptureMode,
        resource_id: ResourceId,
        entry: &ResourceEntry,
    ) -> RuntimeResult<ResourceImageEntry> {
        let snapshot = match entry.capture {
            ResourceCapture::None => {
                return Err(RuntimeError::Internal {
                    message: format!(
                        "resource {} of kind {:?} does not support capture",
                        resource_id.0, entry.kind
                    ),
                }
                .boxed());
            }
            ResourceCapture::State | ResourceCapture::Recipe => {
                let provider = entry
                    .provider
                    .clone()
                    .or_else(|| self.providers.read().get(&entry.kind).cloned());
                let Some(provider) = provider else {
                    return Err(RuntimeError::Internal {
                        message: format!(
                            "resource {} of kind {:?} is missing one resource provider",
                            resource_id.0, entry.kind
                        ),
                    }
                    .boxed());
                };

                Some(provider.snapshot(resource_id).map_err(|error| {
                    RuntimeError::Internal {
                        message: format!(
                            "resource {} of kind {:?} failed to capture: {error}",
                            resource_id.0, entry.kind
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
            route: entry.route,
            backing: entry.backing,
            capture: entry.capture,
            portability: entry.portability,
            affinity: entry.affinity,
            snapshot,
        })
    }

    /// Capture one durable resource-table snapshot.
    pub(crate) fn snapshot(&self, mode: CaptureMode) -> RuntimeResult<ResourceTableSnapshot> {
        let entries = self.entries.read();
        let entries = entries
            .iter()
            .map(|(resource_id, entry)| self.capture_entry(mode, *resource_id, entry))
            .collect::<RuntimeResult<Vec<_>>>()?;

        Ok(ResourceTableSnapshot {
            next_id: self.next_id.load(Ordering::Relaxed),
            entries,
        })
    }

    /// Restore one durable resource-table snapshot.
    pub(crate) fn restore_snapshot(
        &self,
        snapshot: &ResourceTableSnapshot,
        rebind_context: Option<&ResourceRebinders>,
    ) -> RuntimeResult<()> {
        self.restore_barrier()?;
        self.next_id.store(snapshot.next_id, Ordering::Relaxed);

        let mut entries = self.entries.write();
        for image_entry in &snapshot.entries {
            let mut entry = match image_entry.capture {
                ResourceCapture::None => ResourceEntry::new(image_entry.kind),
                ResourceCapture::State | ResourceCapture::Recipe => {
                    let snapshot = image_entry.snapshot.as_ref().ok_or_else(|| {
                        RuntimeError::Internal {
                            message: format!(
                                "resource {} of kind {:?} is missing one captured payload",
                                image_entry.resource_id.0, image_entry.kind
                            ),
                        }
                        .boxed()
                    })?;

                    if image_entry.portability == ResourcePortability::External {
                        let rebinder = rebind_context
                            .and_then(|context| context.rebinder(image_entry.kind))
                            .ok_or_else(|| {
                                RuntimeError::Internal {
                                    message: format!(
                                        "resource {} of kind {:?} requires one external rebinding hook",
                                        image_entry.resource_id.0, image_entry.kind
                                    ),
                                }
                                .boxed()
                            })?;

                        rebinder.rebind(snapshot).map_err(|error| {
                            RuntimeError::Internal {
                                message: format!(
                                    "resource {} of kind {:?} failed to rebind: {error}",
                                    image_entry.resource_id.0, image_entry.kind
                                ),
                            }
                            .boxed()
                        })?
                    } else {
                        let provider = self
                            .providers
                            .read()
                            .get(&image_entry.kind)
                            .cloned()
                            .ok_or_else(|| {
                                RuntimeError::Internal {
                                    message: format!(
                                        "resource {} of kind {:?} is missing one restore provider",
                                        image_entry.resource_id.0, image_entry.kind
                                    ),
                                }
                                .boxed()
                            })?;

                        provider.restore(snapshot).map_err(|error| {
                            RuntimeError::Internal {
                                message: format!(
                                    "resource {} of kind {:?} failed to restore: {error}",
                                    image_entry.resource_id.0, image_entry.kind
                                ),
                            }
                            .boxed()
                        })?
                    }
                }
            };

            entry.kind = image_entry.kind;
            entry.label = image_entry.label.clone();
            entry.route = image_entry.route;
            entry.backing = image_entry.backing;
            entry.capture = image_entry.capture;
            entry.portability = image_entry.portability;
            entry.affinity = image_entry.affinity;
            entries.insert(image_entry.resource_id, entry);
        }

        Ok(())
    }
}

impl Capture for ResourceTable {
    type Image = ResourceTableSnapshot;
    type Error = Box<RuntimeError>;
    type CaptureContext<'a> = ();
    type RestoreContext<'a> = Option<&'a ResourceRebinders>;

    /// Capture one resource-table image.
    fn capture_image(
        &mut self,
        mode: CaptureMode,
        _context: Self::CaptureContext<'_>,
    ) -> Result<Self::Image, Self::Error> {
        self.snapshot(mode)
    }

    /// Restore one resource-table image.
    fn restore_image(
        &mut self,
        image: &Self::Image,
        context: Self::RestoreContext<'_>,
    ) -> Result<(), Self::Error> {
        self.restore_snapshot(image, context)
    }
}

impl Default for ResourceTable {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            entries: RwLock::new(HashMap::new()),
            providers: RwLock::new(HashMap::new()),
            hooks: RwLock::new(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use destack_core::CaptureMode;
    use destack_workspace::RuntimeOptions;

    use super::ResourceId;
    use crate::platform::diagnostic::PlatformError;
    use crate::platform::resource::{
        ResourceCapture, ResourcePortability, ResourceRebinder, ResourceRebinders, ResourceSnapshot,
    };

    use super::{ResourceEntry, ResourceFinalizer, ResourceKind, ResourceProvider, ResourceTable};
    use crate::runtime::world::World;

    /// Ensures entries can be inserted, removed, and finalized.
    #[test]
    fn test_insert_remove_and_finalize() {
        // create a new resource table
        let table = ResourceTable::default();
        let mut world = World::from_options(&RuntimeOptions::default())
            .expect("resource-table test world should build");
        let world_scope = world.world_scope();

        // create a finalizer to track removals
        let hits = Arc::new(AtomicUsize::new(0));
        let finalizer = TestFinalizer { hits: hits.clone() };

        // insert an entry with a finalizer
        let entry = ResourceEntry::new(ResourceKind::Timer).with_finalizer(finalizer);
        let resource_id = table.insert(&world_scope, entry, None);
        assert!(table.contains(resource_id));

        // remove and finalize the entry
        let removed = table.remove_and_finalize(&world_scope, resource_id, None);
        assert!(removed);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert!(!table.contains(resource_id));

        // removing again should return false
        let removed_again = table.remove_and_finalize(&world_scope, resource_id, None);
        assert!(!removed_again);
    }

    /// Capturing and restoring one resource table should roundtrip provider-backed entries.
    #[test]
    fn test_snapshot_roundtrip_restores_provider_backed_resources() {
        let mut world = World::from_options(&RuntimeOptions::default())
            .expect("resource-table test world should build");
        let world_scope = world.world_scope();
        let table = ResourceTable::default();
        let provider = Arc::new(TestResourceProvider);
        table.register_provider(ResourceKind::Timer, provider);

        let entry = ResourceEntry::new(ResourceKind::Timer)
            .with_capture(ResourceCapture::State)
            .with_portability(ResourcePortability::Portable);
        let resource_id = table.insert(&world_scope, entry, None);

        let snapshot = table
            .snapshot(CaptureMode::Fork)
            .expect("capture resource table");
        let restored = ResourceTable::default();
        restored.register_provider(ResourceKind::Timer, Arc::new(TestResourceProvider));
        restored
            .restore_snapshot(&snapshot, None)
            .expect("restore resource table");

        assert!(restored.contains(resource_id));
        let kind = restored.with_entry(resource_id, |entry| entry.kind);
        assert_eq!(kind, Some(ResourceKind::Timer));
    }

    /// External resources should require explicit rebinding on restore.
    #[test]
    fn test_snapshot_restore_requires_external_rebinding() {
        let mut world = World::from_options(&RuntimeOptions::default())
            .expect("resource-table test world should build");
        let world_scope = world.world_scope();
        let table = ResourceTable::default();
        let provider = Arc::new(TestResourceProvider);
        table.register_provider(ResourceKind::Timer, provider);

        let entry = ResourceEntry::new(ResourceKind::Timer)
            .with_capture(ResourceCapture::Recipe)
            .with_portability(ResourcePortability::External);
        let _ = table.insert(&world_scope, entry, None);

        let snapshot = table
            .snapshot(CaptureMode::Hibernate)
            .expect("external resources should capture with one recipe");
        let restored = ResourceTable::default();
        let error = restored
            .restore_snapshot(&snapshot, None)
            .expect_err("external resources should require rebinding");
        let message = error.to_string();

        assert!(
            message.contains("requires one external rebinding hook"),
            "unexpected resource restore error: {message}"
        );

        let mut rebind_context = ResourceRebinders::default();
        rebind_context.register(ResourceKind::Timer, Arc::new(TestResourceRebinder));

        restored
            .restore_snapshot(&snapshot, Some(&rebind_context))
            .expect("external resources should restore with rebinding");
        assert_eq!(
            restored.with_entry(ResourceId(1), |entry| entry.kind),
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
        fn snapshot(
            &self,
            resource_id: ResourceId,
        ) -> Result<ResourceSnapshot, Box<PlatformError>> {
            Ok(ResourceSnapshot::Snapshot {
                resource_id,
                kind: ResourceKind::Timer,
                payload: vec![1, 2, 3],
            })
        }

        fn restore(
            &self,
            snapshot: &ResourceSnapshot,
        ) -> Result<ResourceEntry, Box<PlatformError>> {
            let _ = snapshot;

            Ok(ResourceEntry::new(ResourceKind::Timer)
                .with_capture(ResourceCapture::State)
                .with_portability(ResourcePortability::Portable))
        }
    }

    struct TestResourceRebinder;

    impl ResourceRebinder for TestResourceRebinder {
        fn rebind(&self, snapshot: &ResourceSnapshot) -> Result<ResourceEntry, Box<PlatformError>> {
            let _ = snapshot;

            Ok(ResourceEntry::new(ResourceKind::Timer)
                .with_capture(ResourceCapture::Recipe)
                .with_portability(ResourcePortability::External))
        }
    }
}
