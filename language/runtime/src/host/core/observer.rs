#[cfg(any(test, target_os = "linux", target_os = "macos", windows))]
use std::sync::Arc;
use std::sync::{OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::diagnostic::RuntimeResult;
use crate::host::core::event::HostEvent;

/// Shared ingress observer registry keyed by host runtime id.
static RUNTIME_INGRESS_OBSERVERS: OnceLock<RwLock<RuntimeIngressObserverRegistry>> =
    OnceLock::new();
/// Shared host event observer registry keyed by host runtime id.
static HOST_EVENT_OBSERVERS: OnceLock<RwLock<HostEventObserverRegistry>> = OnceLock::new();

/// Shared observer entry for one runtime.
type RuntimeIngressEntry = Weak<dyn RuntimeIngressObserver>;
/// Shared observer entry for one runtime host event observer.
type HostEventEntry = Weak<dyn HostEventObserver>;

/// Shared runtime ingress observer state.
#[derive(Debug, Default)]
pub(crate) struct RuntimeIngressObserverRegistry {
    /// Runtime observer entries keyed by runtime id.
    observers_by_runtime: FxHashMap<u64, Vec<RuntimeIngressEntry>>,
}

/// Shared host event observer state.
#[derive(Debug, Default)]
pub(crate) struct HostEventObserverRegistry {
    /// Host event observer entries keyed by runtime id.
    observers_by_runtime: FxHashMap<u64, Vec<HostEventEntry>>,
}

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

impl RuntimeIngressObserverRegistry {
    /// Return the shared runtime ingress observer registry lock.
    pub(crate) fn shared() -> &'static RwLock<Self> {
        RUNTIME_INGRESS_OBSERVERS.get_or_init(|| RwLock::new(Self::default()))
    }

    /// Register one runtime ingress observer.
    #[cfg(any(test, target_os = "linux", target_os = "macos", windows))]
    pub(crate) fn register(&mut self, runtime_id: u64, observer: &Arc<dyn RuntimeIngressObserver>) {
        // retain only live observers while updating this runtime entry
        self.observers_by_runtime.retain(|_, observers| {
            observers.retain(|weak| weak.upgrade().is_some());
            !observers.is_empty()
        });

        let observers = self.observers_by_runtime.entry(runtime_id).or_default();
        let observer_pointer = Arc::as_ptr(observer) as *const ();

        // avoid duplicate observer registration for the same runtime
        let is_registered = observers.iter().any(|weak| {
            let Some(existing) = weak.upgrade() else {
                return false;
            };

            Arc::as_ptr(&existing) as *const () == observer_pointer
        });

        if !is_registered {
            observers.push(Arc::downgrade(observer));
        }
    }

    /// Remove every ingress observer registered for one runtime id.
    pub(crate) fn unregister_runtime(&mut self, runtime_id: u64) {
        self.observers_by_runtime.remove(&runtime_id);
    }

    /// Service ingress for one runtime id.
    pub(crate) fn process_runtime(&mut self, runtime_id: u64) -> RuntimeResult<()> {
        let observers = {
            let Some(observers) = self.observers_by_runtime.get_mut(&runtime_id) else {
                return Ok(());
            };

            // retain only live observers before dispatch
            observers.retain(|weak| weak.upgrade().is_some());
            if observers.is_empty() {
                self.observers_by_runtime.remove(&runtime_id);
                return Ok(());
            }

            observers
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };

        // run callbacks after the mutable borrow above ends
        for observer in observers {
            observer.process_runtime_ingress()?;
        }

        Ok(())
    }

    /// Service ingress for every registered runtime.
    #[cfg(feature = "execution")]
    pub(crate) fn process_all(&mut self) -> RuntimeResult<()> {
        let observers = {
            let mut live_observers = Vec::new();

            // retain only live observers while collecting every runtime observer
            self.observers_by_runtime.retain(|_, runtime_observers| {
                runtime_observers.retain(|weak| weak.upgrade().is_some());

                for observer in runtime_observers.iter().filter_map(Weak::upgrade) {
                    live_observers.push(observer);
                }

                !runtime_observers.is_empty()
            });

            live_observers
        };

        // run callbacks after the mutable borrow above ends
        for observer in observers {
            observer.process_runtime_ingress()?;
        }

        Ok(())
    }
}

impl HostEventObserverRegistry {
    /// Return the shared host event observer registry lock.
    pub(crate) fn shared() -> &'static RwLock<Self> {
        HOST_EVENT_OBSERVERS.get_or_init(|| RwLock::new(Self::default()))
    }

    /// Register one host event observer.
    pub(crate) fn register(&mut self, runtime_id: u64, observer: &Arc<dyn HostEventObserver>) {
        // retain only live observers while updating this runtime entry
        self.observers_by_runtime.retain(|_, observers| {
            observers.retain(|weak| weak.upgrade().is_some());
            !observers.is_empty()
        });

        let observers = self.observers_by_runtime.entry(runtime_id).or_default();
        let observer_pointer = Arc::as_ptr(observer) as *const ();

        // avoid duplicate observer registration for the same runtime
        let is_registered = observers.iter().any(|weak| {
            let Some(existing) = weak.upgrade() else {
                return false;
            };

            Arc::as_ptr(&existing) as *const () == observer_pointer
        });

        if !is_registered {
            observers.push(Arc::downgrade(observer));
        }
    }

    /// Dispatch one host event to observers for one runtime id.
    pub(crate) fn dispatch(&mut self, runtime_id: u64, event: &HostEvent) -> RuntimeResult<()> {
        let observers = {
            let Some(observers) = self.observers_by_runtime.get_mut(&runtime_id) else {
                return Ok(());
            };

            // retain only live observers before dispatch
            observers.retain(|weak| weak.upgrade().is_some());
            if observers.is_empty() {
                self.observers_by_runtime.remove(&runtime_id);
                return Ok(());
            }

            observers
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };

        // run callbacks after the mutable borrow above ends
        for observer in observers {
            observer.observe_host_event(event)?;
        }

        Ok(())
    }

    /// Remove every host event observer registered for one runtime id.
    pub(crate) fn unregister_runtime(&mut self, runtime_id: u64) {
        self.observers_by_runtime.remove(&runtime_id);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::diagnostic::RuntimeResult;

    use crate::host::core::observer::{
        HostEventObserver, HostEventObserverRegistry, RuntimeIngressObserver,
        RuntimeIngressObserverRegistry,
    };
    use crate::host::{HostEvent, HostLifecycleEvent, HostLifecycleState};

    /// Shared runtime id allocator for ingress observer tests.
    static TEST_RUNTIME_ID_NEXT: AtomicU64 = AtomicU64::new(u64::MAX - 1024);
    /// Shared mutex that serializes ingress registry tests.
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

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

    /// Allocate one unique runtime id for this test process.
    fn next_test_runtime_id() -> u64 {
        TEST_RUNTIME_ID_NEXT.fetch_add(1, Ordering::Relaxed)
    }

    /// Lock the shared ingress test mutex.
    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
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
    fn test_process_runtime_observer_notifies_registered_runtime() {
        let _guard = test_lock();
        let runtime_id = next_test_runtime_id();
        let callback_count = Arc::new(AtomicU64::new(0));
        let observer: Arc<dyn RuntimeIngressObserver> = Arc::new(TestIngressObserver {
            callback_count: Arc::clone(&callback_count),
        });

        RuntimeIngressObserverRegistry::shared()
            .write()
            .register(runtime_id, &observer);
        RuntimeIngressObserverRegistry::shared()
            .write()
            .process_runtime(runtime_id)
            .unwrap();

        let callback_count = callback_count.load(Ordering::Relaxed);
        assert_eq!(callback_count, 1);

        RuntimeIngressObserverRegistry::shared()
            .write()
            .unregister_runtime(runtime_id);
    }

    #[test]
    fn test_dispatch_host_event_notifies_registered_runtime() {
        let _guard = test_lock();
        let runtime_id = next_test_runtime_id();
        let callback_count = Arc::new(AtomicU64::new(0));
        let observer: Arc<dyn HostEventObserver> = Arc::new(TestHostEventObserver {
            callback_count: Arc::clone(&callback_count),
        });

        HostEventObserverRegistry::shared()
            .write()
            .register(runtime_id, &observer);
        HostEventObserverRegistry::shared()
            .write()
            .dispatch(
                runtime_id,
                &HostEvent::Lifecycle(HostLifecycleEvent {
                    state: HostLifecycleState::Running,
                }),
            )
            .unwrap();

        let callback_count = callback_count.load(Ordering::Relaxed);
        assert_eq!(callback_count, 1);

        HostEventObserverRegistry::shared()
            .write()
            .unregister_runtime(runtime_id);
    }

    #[cfg(feature = "execution")]
    #[test]
    fn test_process_all_notifies_all_registered_runtimes() {
        let _guard = test_lock();
        let first_runtime_id = next_test_runtime_id();
        let second_runtime_id = next_test_runtime_id();
        let first_callback_count = Arc::new(AtomicU64::new(0));
        let second_callback_count = Arc::new(AtomicU64::new(0));

        let first_observer: Arc<dyn RuntimeIngressObserver> = Arc::new(TestIngressObserver {
            callback_count: Arc::clone(&first_callback_count),
        });
        let second_observer: Arc<dyn RuntimeIngressObserver> = Arc::new(TestIngressObserver {
            callback_count: Arc::clone(&second_callback_count),
        });

        RuntimeIngressObserverRegistry::shared()
            .write()
            .register(first_runtime_id, &first_observer);
        RuntimeIngressObserverRegistry::shared()
            .write()
            .register(second_runtime_id, &second_observer);
        RuntimeIngressObserverRegistry::shared()
            .write()
            .process_all()
            .unwrap();

        let first_callback_count = first_callback_count.load(Ordering::Relaxed);
        let second_callback_count = second_callback_count.load(Ordering::Relaxed);
        assert_eq!(first_callback_count, 1);
        assert_eq!(second_callback_count, 1);

        RuntimeIngressObserverRegistry::shared()
            .write()
            .unregister_runtime(first_runtime_id);
        RuntimeIngressObserverRegistry::shared()
            .write()
            .unregister_runtime(second_runtime_id);
    }
}
