use std::sync::{Arc, OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::diagnostic::RuntimeResult;
use crate::host::HostEvent;
use crate::host::core::Platform;
use crate::host::core::error::missing_host_queue;
use crate::host::core::observer::{HostEventObserverRegistry, RuntimeIngressObserverRegistry};
use crate::host::core::queue::HostQueue;
use crate::runtime::world::RuntimeId;

/// Cleanup hook run when one runtime host queue registration is removed.
pub(crate) type HostCleanup = fn(runtime_id: u64);

/// Shared process-global host queue registry.
static HOST_RUNTIME_QUEUE_REGISTRY: OnceLock<RwLock<HostQueueRegistry>> = OnceLock::new();

/// Runtime-scoped ingress handle resolved from the process-global registry.
#[derive(Debug, Clone)]
pub(crate) struct HostSessionIngress {
    /// Resolved runtime-owned host queue.
    queue: Arc<HostQueue>,
}

/// Registration guard for one host runtime queue.
#[derive(Debug)]
pub(crate) struct HostRegistrationGuard {
    /// Stable runtime id for this queue.
    runtime_id: RuntimeId,
}

/// Shared registry state for active host runtime queues.
#[derive(Debug, Default)]
pub(crate) struct HostQueueRegistry {
    /// Queue entries keyed by runtime id.
    queues: FxHashMap<RuntimeId, HostQueueRegistryEntry>,
}

/// Shared registry entry metadata for one host runtime queue.
#[derive(Debug)]
struct HostQueueRegistryEntry {
    /// Platform tag for this queue.
    platform: Platform,
    /// Weak reference to one runtime-owned queue.
    queue: Weak<HostQueue>,
    /// Optional platform-specific cleanup hook for this runtime id.
    cleanup: Option<HostCleanup>,
}

impl HostRegistrationGuard {
    /// Return the stable runtime id for this registration.
    pub(crate) fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }
}

impl Drop for HostRegistrationGuard {
    fn drop(&mut self) {
        let mut registry = HostQueueRegistry::shared().write();
        registry.unregister(self.runtime_id);
    }
}

impl HostQueueRegistry {
    /// Return the shared host runtime queue registry lock.
    pub(crate) fn shared() -> &'static RwLock<Self> {
        HOST_RUNTIME_QUEUE_REGISTRY.get_or_init(|| RwLock::new(Self::default()))
    }

    /// Register one host runtime queue with one runtime id.
    pub(crate) fn register(
        &mut self,
        platform: Platform,
        runtime_id: RuntimeId,
        queue: Weak<HostQueue>,
        cleanup: Option<HostCleanup>,
    ) -> HostRegistrationGuard {
        let entry = HostQueueRegistryEntry {
            platform,
            queue,
            cleanup,
        };
        self.queues.insert(runtime_id, entry);

        HostRegistrationGuard { runtime_id }
    }

    /// Resolve one host runtime queue by runtime id and platform tag.
    pub(crate) fn queue_for_runtime(
        &mut self,
        runtime_id: RuntimeId,
        platform: Platform,
    ) -> RuntimeResult<Arc<HostQueue>> {
        let Some(entry) = self.queues.get(&runtime_id) else {
            return Err(missing_host_queue(runtime_id.0, platform));
        };

        if entry.platform != platform {
            return Err(missing_host_queue(runtime_id.0, platform));
        }

        let Some(queue) = entry.queue.upgrade() else {
            self.queues.remove(&runtime_id);
            return Err(missing_host_queue(runtime_id.0, platform));
        };

        Ok(queue)
    }

    /// Resolve one runtime-scoped host session ingress handle.
    pub(crate) fn session_ingress_for_runtime(
        &mut self,
        runtime_id: RuntimeId,
        platform: Platform,
    ) -> RuntimeResult<HostSessionIngress> {
        let queue = self.queue_for_runtime(runtime_id, platform)?;

        Ok(HostSessionIngress { queue })
    }

    /// Remove one registration from the shared host runtime queue registry.
    pub(crate) fn unregister(&mut self, runtime_id: RuntimeId) {
        let cleanup = self
            .queues
            .remove(&runtime_id)
            .and_then(|entry| entry.cleanup);

        RuntimeIngressObserverRegistry::shared()
            .write()
            .unregister_runtime(runtime_id.0);
        HostEventObserverRegistry::shared()
            .write()
            .unregister_runtime(runtime_id.0);

        if let Some(cleanup) = cleanup {
            cleanup(runtime_id.0);
        }
    }
}

impl HostSessionIngress {
    /// Publish one normalized host event into the runtime queue.
    pub(crate) fn publish_event(&self, event: HostEvent) {
        self.queue.enqueue(event);
    }

    /// Wake one blocked host poller for this runtime.
    pub(crate) fn wake(&self) -> RuntimeResult<()> {
        self.queue.poll_wake_handle().wake()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::host::Platform;
    use crate::host::core::HostQueue;
    use crate::host::core::registry::HostQueueRegistry;
    use crate::runtime::world::RuntimeId;

    /// Shared runtime id captured by one cleanup hook invocation in tests.
    static TEST_CLEANUP_RUNTIME_ID: AtomicU64 = AtomicU64::new(0);

    /// Record one cleanup hook invocation for tests.
    fn test_cleanup_hook(runtime_id: u64) {
        TEST_CLEANUP_RUNTIME_ID.store(runtime_id, Ordering::Relaxed);
    }

    #[test]
    fn test_register_host_queue_resolves_by_runtime_id() {
        let queue = Arc::new(HostQueue::new(RuntimeId(1)));
        let mut registry = HostQueueRegistry::shared().write();
        let registration = registry.register(
            Platform::Android,
            RuntimeId(1),
            Arc::downgrade(&queue),
            None,
        );
        let runtime_id = registration.runtime_id;
        drop(registry);

        let resolved_queue = HostQueueRegistry::shared()
            .write()
            .queue_for_runtime(runtime_id, Platform::Android)
            .unwrap();

        assert!(Arc::ptr_eq(&queue, &resolved_queue));
    }

    #[test]
    fn test_drop_registration_unregisters_runtime_id() {
        let queue = Arc::new(HostQueue::new(RuntimeId(2)));
        let registration = HostQueueRegistry::shared().write().register(
            Platform::MacOS,
            RuntimeId(2),
            Arc::downgrade(&queue),
            None,
        );
        let runtime_id = registration.runtime_id;

        drop(registration);

        let resolved_queue = HostQueueRegistry::shared()
            .write()
            .queue_for_runtime(runtime_id, Platform::MacOS);
        assert!(resolved_queue.is_err());
    }

    #[test]
    fn test_host_queue_for_runtime_rejects_platform_mismatch() {
        let queue = Arc::new(HostQueue::new(RuntimeId(3)));
        let registration = HostQueueRegistry::shared().write().register(
            Platform::Windows,
            RuntimeId(3),
            Arc::downgrade(&queue),
            None,
        );
        let runtime_id = registration.runtime_id;

        let resolved_queue = HostQueueRegistry::shared()
            .write()
            .queue_for_runtime(runtime_id, Platform::Android);
        assert!(resolved_queue.is_err());
    }

    #[test]
    fn test_drop_registration_runs_cleanup_hook() {
        TEST_CLEANUP_RUNTIME_ID.store(0, Ordering::Relaxed);

        let queue = Arc::new(HostQueue::new(RuntimeId(4)));
        let registration = HostQueueRegistry::shared().write().register(
            Platform::Android,
            RuntimeId(4),
            Arc::downgrade(&queue),
            Some(test_cleanup_hook),
        );
        let runtime_id = registration.runtime_id;

        drop(registration);

        let cleaned_runtime_id = TEST_CLEANUP_RUNTIME_ID.load(Ordering::Relaxed);
        assert_eq!(cleaned_runtime_id, runtime_id.0);
    }
}
