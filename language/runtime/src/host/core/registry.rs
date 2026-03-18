use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::diagnostic::RuntimeResult;
use crate::host::HostEvent;
use crate::host::core::Platform;
use crate::host::core::error::{missing_host_queue, missing_host_queue_registration};
use crate::host::core::queue::HostQueue;

/// Shared allocator for process-global host runtime routing ids.
static HOST_RUNTIME_ID_NEXT: AtomicU64 = AtomicU64::new(1);

/// Process-global host routing id for one runtime-backed host session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct HostRuntimeId(pub u64);

/// Allocate one fresh process-global host routing id.
pub(crate) fn next_host_runtime_id() -> HostRuntimeId {
    HostRuntimeId(HOST_RUNTIME_ID_NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Cleanup hook run when one runtime host queue registration is removed.
pub(crate) type HostCleanup = fn(host_runtime_id: HostRuntimeId);

/// Shared process-global host runtime registry.
static HOST_RUNTIME_REGISTRY: OnceLock<RwLock<HostRuntimeRegistry>> = OnceLock::new();

/// Observer notified when one runtime ingress path should make progress.
pub(crate) trait RuntimeIngressObserver: std::fmt::Debug + Send + Sync {
    /// Service runtime-owned ingress.
    fn process_runtime_ingress(&self) -> RuntimeResult<()>;
}

/// Observer notified when one host semantic event is enqueued for one runtime.
pub(crate) trait HostEventObserver: std::fmt::Debug + Send + Sync {
    /// Observe one host semantic event.
    fn observe_host_event(&self, event: &HostEvent) -> RuntimeResult<()>;
}

/// Registration guard for one host runtime queue.
#[derive(Debug)]
pub(crate) struct HostRegistrationGuard {
    /// Stable runtime id for this queue.
    host_runtime_id: HostRuntimeId,
}

/// Shared registry state for active host runtimes.
#[derive(Debug, Default)]
pub(crate) struct HostRuntimeRegistry {
    /// Queue entries keyed by runtime id.
    runtime_queues: FxHashMap<HostRuntimeId, HostRuntimeRegistryEntry>,
}

/// Shared registry entry metadata for one host runtime queue.
#[derive(Debug)]
struct HostRuntimeRegistryEntry {
    /// Platform tag for this queue.
    platform: Platform,
    /// Weak reference to one runtime-owned queue.
    queue: Weak<HostQueue>,
    /// Optional platform-specific cleanup hook for this runtime id.
    cleanup: Option<HostCleanup>,
}

impl HostRegistrationGuard {
    /// Return the stable runtime id for this registration.
    pub(crate) fn host_runtime_id(&self) -> HostRuntimeId {
        self.host_runtime_id
    }
}

impl Drop for HostRegistrationGuard {
    fn drop(&mut self) {
        HostRuntimeRegistry::unregister_runtime(self.host_runtime_id);
    }
}

impl HostRuntimeRegistry {
    /// Return the shared host runtime registry lock.
    fn shared() -> &'static RwLock<Self> {
        HOST_RUNTIME_REGISTRY.get_or_init(|| RwLock::new(Self::default()))
    }

    /// Register one host runtime queue with one runtime id.
    pub(crate) fn register_queue(
        platform: Platform,
        host_runtime_id: HostRuntimeId,
        queue: Weak<HostQueue>,
        cleanup: Option<HostCleanup>,
    ) -> HostRegistrationGuard {
        let mut registry = Self::shared().write();

        // duplicate runtime ids are a registry contract violation
        if let Some(existing) = registry.runtime_queues.get(&host_runtime_id) {
            if existing.queue.upgrade().is_some() {
                panic!(
                    "duplicate host runtime queue registration for runtime id {}",
                    host_runtime_id.0
                );
            }

            registry.runtime_queues.remove(&host_runtime_id);
        }

        let entry = HostRuntimeRegistryEntry {
            platform,
            queue,
            cleanup,
        };

        registry.runtime_queues.insert(host_runtime_id, entry);

        HostRegistrationGuard { host_runtime_id }
    }

    /// Resolve one host runtime queue by runtime id and platform tag.
    pub(crate) fn queue_for_runtime(
        host_runtime_id: HostRuntimeId,
        platform: Platform,
    ) -> RuntimeResult<Arc<HostQueue>> {
        let mut registry = Self::shared().write();
        registry.queue_for_runtime_inner(host_runtime_id, platform)
    }

    /// Register one runtime ingress observer.
    pub(crate) fn register_runtime_ingress_observer(
        host_runtime_id: HostRuntimeId,
        observer: &Arc<dyn RuntimeIngressObserver>,
    ) -> RuntimeResult<()> {
        let queue = {
            let mut registry = Self::shared().write();
            registry.queue_for_runtime_id_inner(host_runtime_id)?
        };

        queue.register_runtime_ingress_observer(observer);

        Ok(())
    }

    /// Dispatch one host event to observers for one runtime id.
    #[cfg(test)]
    pub(crate) fn dispatch_host_event(
        host_runtime_id: HostRuntimeId,
        event: &HostEvent,
    ) -> RuntimeResult<()> {
        let queue = {
            let mut registry = Self::shared().write();
            registry.queue_for_runtime_id_inner(host_runtime_id)?
        };

        queue.dispatch_host_event(event)?;

        Ok(())
    }

    /// Service ingress for one runtime id.
    pub(crate) fn process_runtime_ingress(host_runtime_id: HostRuntimeId) -> RuntimeResult<()> {
        let queue = {
            let mut registry = Self::shared().write();
            registry.queue_for_runtime_id_inner(host_runtime_id)?
        };

        queue.process_runtime_ingress()?;

        Ok(())
    }

    /// Service ingress for every registered runtime.
    #[cfg(feature = "execution")]
    pub(crate) fn process_all_runtime_ingress() -> RuntimeResult<()> {
        let queues = {
            let mut registry = Self::shared().write();
            registry.collect_all_runtime_queues()
        };

        for queue in queues {
            queue.process_runtime_ingress()?;
        }

        Ok(())
    }

    /// Remove one registration from the shared host runtime registry.
    pub(crate) fn unregister_runtime(host_runtime_id: HostRuntimeId) {
        let cleanup = {
            let mut registry = Self::shared().write();
            registry
                .runtime_queues
                .remove(&host_runtime_id)
                .and_then(|entry| entry.cleanup)
        };

        if let Some(cleanup) = cleanup {
            cleanup(host_runtime_id);
        }
    }

    /// Resolve one queue entry from the current registry state.
    fn queue_for_runtime_inner(
        &mut self,
        host_runtime_id: HostRuntimeId,
        platform: Platform,
    ) -> RuntimeResult<Arc<HostQueue>> {
        let Some(entry) = self.runtime_queues.get(&host_runtime_id) else {
            return Err(missing_host_queue(host_runtime_id.0, platform));
        };

        if entry.platform != platform {
            return Err(missing_host_queue(host_runtime_id.0, platform));
        }

        let Some(queue) = entry.queue.upgrade() else {
            self.runtime_queues.remove(&host_runtime_id);
            return Err(missing_host_queue(host_runtime_id.0, platform));
        };

        Ok(queue)
    }

    /// Resolve one queue entry from the current registry state without platform filtering.
    fn queue_for_runtime_id_inner(
        &mut self,
        host_runtime_id: HostRuntimeId,
    ) -> RuntimeResult<Arc<HostQueue>> {
        let Some(entry) = self.runtime_queues.get(&host_runtime_id) else {
            return Err(missing_host_queue_registration(host_runtime_id.0));
        };
        let platform = entry.platform;

        let Some(queue) = entry.queue.upgrade() else {
            self.runtime_queues.remove(&host_runtime_id);
            return Err(missing_host_queue(host_runtime_id.0, platform));
        };

        Ok(queue)
    }

    /// Collect live ingress observers for every runtime id.
    #[cfg(feature = "execution")]
    fn collect_all_runtime_queues(&mut self) -> Vec<Arc<HostQueue>> {
        let mut queues = Vec::new();

        self.runtime_queues.retain(|_, entry| {
            let Some(queue) = entry.queue.upgrade() else {
                return false;
            };

            queues.push(queue);
            true
        });

        queues
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::diagnostic::RuntimeResult;
    use crate::host::core::queue::HostQueue;
    use crate::host::core::registry::{
        HostEventObserver, HostRuntimeRegistry, RuntimeIngressObserver, next_host_runtime_id,
    };
    use crate::host::{HostEvent, HostLifecycleEvent, HostLifecycleState, Platform};
    /// Shared mutex that serializes registry tests.
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    /// Shared runtime id captured by one cleanup hook invocation in tests.
    static TEST_CLEANUP_RUNTIME_ID: AtomicU64 = AtomicU64::new(0);

    /// One test observer that counts ingress callbacks.
    #[derive(Debug)]
    struct TestIngressObserver {
        /// Callback counter for this observer.
        callback_count: Arc<AtomicU64>,
    }

    /// One test observer that counts host event callbacks.
    #[derive(Debug)]
    struct TestHostEventObserver {
        /// Callback counter for this observer.
        callback_count: Arc<AtomicU64>,
    }

    /// Lock the shared registry test mutex.
    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    /// Record one cleanup hook invocation for tests.
    fn test_cleanup_hook(host_runtime_id: crate::host::core::HostRuntimeId) {
        TEST_CLEANUP_RUNTIME_ID.store(host_runtime_id.0, Ordering::Relaxed);
    }

    impl RuntimeIngressObserver for TestIngressObserver {
        /// Count one ingress callback.
        fn process_runtime_ingress(&self) -> RuntimeResult<()> {
            self.callback_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    impl HostEventObserver for TestHostEventObserver {
        /// Count one host event callback.
        fn observe_host_event(&self, _event: &HostEvent) -> RuntimeResult<()> {
            self.callback_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn test_register_queue_resolves_by_runtime_id() {
        let _guard = test_lock();
        let runtime_id = next_host_runtime_id();
        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostRuntimeRegistry::register_queue(
            Platform::Android,
            runtime_id,
            Arc::downgrade(&queue),
            None,
        );

        let resolved_queue = HostRuntimeRegistry::queue_for_runtime(
            registration.host_runtime_id(),
            Platform::Android,
        )
        .unwrap();

        assert!(Arc::ptr_eq(&queue, &resolved_queue));

        drop(registration);
    }

    #[test]
    fn test_register_queue_rejects_duplicate_runtime_id() {
        let _guard = test_lock();
        let runtime_id = next_host_runtime_id();
        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostRuntimeRegistry::register_queue(
            Platform::Android,
            runtime_id,
            Arc::downgrade(&queue),
            None,
        );
        let duplicate_queue = Arc::new(HostQueue::new(runtime_id));

        // duplicate ids must fail loudly instead of replacing live registrations
        let duplicate_registration = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            HostRuntimeRegistry::register_queue(
                Platform::Android,
                runtime_id,
                Arc::downgrade(&duplicate_queue),
                None,
            )
        }));

        assert!(duplicate_registration.is_err());

        drop(registration);
    }

    #[test]
    fn test_drop_registration_unregisters_runtime_id() {
        let _guard = test_lock();
        let runtime_id = next_host_runtime_id();
        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostRuntimeRegistry::register_queue(
            Platform::MacOS,
            runtime_id,
            Arc::downgrade(&queue),
            None,
        );

        drop(registration);

        let resolved_queue = HostRuntimeRegistry::queue_for_runtime(runtime_id, Platform::MacOS);
        assert!(resolved_queue.is_err());
    }

    #[test]
    fn test_queue_for_runtime_rejects_platform_mismatch() {
        let _guard = test_lock();
        let runtime_id = next_host_runtime_id();
        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostRuntimeRegistry::register_queue(
            Platform::Windows,
            runtime_id,
            Arc::downgrade(&queue),
            None,
        );

        let resolved_queue = HostRuntimeRegistry::queue_for_runtime(runtime_id, Platform::Android);
        assert!(resolved_queue.is_err());

        drop(registration);
    }

    #[test]
    fn test_drop_registration_runs_cleanup_hook() {
        let _guard = test_lock();
        TEST_CLEANUP_RUNTIME_ID.store(0, Ordering::Relaxed);
        let runtime_id = next_host_runtime_id();
        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostRuntimeRegistry::register_queue(
            Platform::Android,
            runtime_id,
            Arc::downgrade(&queue),
            Some(test_cleanup_hook),
        );

        drop(registration);

        let cleaned_runtime_id = TEST_CLEANUP_RUNTIME_ID.load(Ordering::Relaxed);
        assert_eq!(cleaned_runtime_id, runtime_id.0);
    }

    #[test]
    fn test_process_runtime_ingress_notifies_registered_runtime() {
        let _guard = test_lock();
        let runtime_id = next_host_runtime_id();
        let callback_count = Arc::new(AtomicU64::new(0));
        let observer: Arc<dyn RuntimeIngressObserver> = Arc::new(TestIngressObserver {
            callback_count: Arc::clone(&callback_count),
        });

        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostRuntimeRegistry::register_queue(
            Platform::Android,
            runtime_id,
            Arc::downgrade(&queue),
            None,
        );

        HostRuntimeRegistry::register_runtime_ingress_observer(runtime_id, &observer).unwrap();
        HostRuntimeRegistry::process_runtime_ingress(runtime_id).unwrap();

        let callback_count = callback_count.load(Ordering::Relaxed);
        assert_eq!(callback_count, 1);

        drop(registration);
    }

    #[test]
    fn test_dispatch_host_event_notifies_registered_runtime() {
        let _guard = test_lock();
        let runtime_id = next_host_runtime_id();
        let callback_count = Arc::new(AtomicU64::new(0));
        let observer: Arc<dyn HostEventObserver> = Arc::new(TestHostEventObserver {
            callback_count: Arc::clone(&callback_count),
        });

        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostRuntimeRegistry::register_queue(
            Platform::Android,
            runtime_id,
            Arc::downgrade(&queue),
            None,
        );

        queue.register_observer_and_snapshot(&observer);
        HostRuntimeRegistry::dispatch_host_event(
            runtime_id,
            &HostEvent::Lifecycle(HostLifecycleEvent {
                state: HostLifecycleState::Running,
            }),
        )
        .unwrap();

        let callback_count = callback_count.load(Ordering::Relaxed);
        assert_eq!(callback_count, 1);

        drop(registration);
    }

    #[cfg(feature = "execution")]
    #[test]
    fn test_process_all_runtime_ingress_notifies_all_registered_runtimes() {
        let _guard = test_lock();
        let first_runtime_id = next_host_runtime_id();
        let second_runtime_id = next_host_runtime_id();
        let first_callback_count = Arc::new(AtomicU64::new(0));
        let second_callback_count = Arc::new(AtomicU64::new(0));

        let first_observer: Arc<dyn RuntimeIngressObserver> = Arc::new(TestIngressObserver {
            callback_count: Arc::clone(&first_callback_count),
        });
        let second_observer: Arc<dyn RuntimeIngressObserver> = Arc::new(TestIngressObserver {
            callback_count: Arc::clone(&second_callback_count),
        });

        let first_queue = Arc::new(HostQueue::new(first_runtime_id));
        let second_queue = Arc::new(HostQueue::new(second_runtime_id));
        let first_registration = HostRuntimeRegistry::register_queue(
            Platform::Android,
            first_runtime_id,
            Arc::downgrade(&first_queue),
            None,
        );
        let second_registration = HostRuntimeRegistry::register_queue(
            Platform::Android,
            second_runtime_id,
            Arc::downgrade(&second_queue),
            None,
        );

        HostRuntimeRegistry::register_runtime_ingress_observer(first_runtime_id, &first_observer)
            .unwrap();
        HostRuntimeRegistry::register_runtime_ingress_observer(second_runtime_id, &second_observer)
            .unwrap();
        HostRuntimeRegistry::process_all_runtime_ingress().unwrap();

        let first_callback_count = first_callback_count.load(Ordering::Relaxed);
        let second_callback_count = second_callback_count.load(Ordering::Relaxed);
        assert_eq!(first_callback_count, 1);
        assert_eq!(second_callback_count, 1);

        drop(first_registration);
        drop(second_registration);
    }
}
