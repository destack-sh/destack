use std::sync::{Arc, OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::diagnostic::RuntimeResult;

/// Shared ingress observer registry keyed by host runtime id.
static RUNTIME_INGRESS_OBSERVERS: OnceLock<RwLock<RuntimeIngressRegistryState>> = OnceLock::new();

/// Shared observer entry for one runtime.
type RuntimeIngressEntry = Weak<dyn RuntimeIngressObserver>;

/// Shared runtime ingress observer state.
#[derive(Debug, Default)]
struct RuntimeIngressRegistryState {
    /// Runtime observer entries keyed by runtime id.
    observers_by_runtime: FxHashMap<u64, Vec<RuntimeIngressEntry>>,
}

/// Observer notified when one runtime ingress path should make progress.
pub(crate) trait RuntimeIngressObserver: std::fmt::Debug + Send + Sync {
    /// Service runtime-owned ingress.
    fn process_runtime_ingress(&self) -> RuntimeResult<()>;
}

/// Register one runtime ingress observer.
pub(crate) fn register_runtime_ingress_observer(
    runtime_id: u64,
    observer: &Arc<dyn RuntimeIngressObserver>,
) {
    let mut registry = runtime_ingress_observers().write();

    // retain only live observers while updating this runtime entry
    registry.observers_by_runtime.retain(|_, observers| {
        observers.retain(|weak| weak.upgrade().is_some());
        !observers.is_empty()
    });

    let observers = registry.observers_by_runtime.entry(runtime_id).or_default();
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
pub(crate) fn cleanup_runtime_ingress_observers(runtime_id: u64) {
    let mut registry = runtime_ingress_observers().write();
    registry.observers_by_runtime.remove(&runtime_id);
}

/// Service ingress for one runtime id.
pub(crate) fn process_runtime_ingress_observer(runtime_id: u64) -> RuntimeResult<()> {
    let observers = {
        let mut registry = runtime_ingress_observers().write();
        let Some(observers) = registry.observers_by_runtime.get_mut(&runtime_id) else {
            return Ok(());
        };

        // retain only live observers before dispatch
        observers.retain(|weak| weak.upgrade().is_some());
        if observers.is_empty() {
            registry.observers_by_runtime.remove(&runtime_id);
            return Ok(());
        }

        observers
            .iter()
            .filter_map(Weak::upgrade)
            .collect::<Vec<_>>()
    };

    // run callbacks outside the registry lock
    for observer in observers {
        observer.process_runtime_ingress()?;
    }

    Ok(())
}

/// Service ingress for every registered runtime.
pub(crate) fn process_runtime_ingress_observers() -> RuntimeResult<()> {
    let observers = {
        let mut registry = runtime_ingress_observers().write();
        let mut live_observers = Vec::new();

        // retain only live observers while collecting every runtime observer
        registry
            .observers_by_runtime
            .retain(|_, runtime_observers| {
                runtime_observers.retain(|weak| weak.upgrade().is_some());

                for observer in runtime_observers.iter().filter_map(Weak::upgrade) {
                    live_observers.push(observer);
                }

                !runtime_observers.is_empty()
            });

        live_observers
    };

    // run callbacks outside the registry lock
    for observer in observers {
        observer.process_runtime_ingress()?;
    }

    Ok(())
}

/// Return the shared runtime ingress observer registry.
fn runtime_ingress_observers() -> &'static RwLock<RuntimeIngressRegistryState> {
    RUNTIME_INGRESS_OBSERVERS.get_or_init(|| RwLock::new(RuntimeIngressRegistryState::default()))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::diagnostic::RuntimeResult;

    use super::{
        RuntimeIngressObserver, cleanup_runtime_ingress_observers,
        process_runtime_ingress_observer, process_runtime_ingress_observers,
        register_runtime_ingress_observer,
    };

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

    #[test]
    fn test_process_runtime_ingress_observer_notifies_registered_runtime() {
        let _guard = test_lock();
        let runtime_id = next_test_runtime_id();
        let callback_count = Arc::new(AtomicU64::new(0));
        let observer: Arc<dyn RuntimeIngressObserver> = Arc::new(TestIngressObserver {
            callback_count: Arc::clone(&callback_count),
        });

        register_runtime_ingress_observer(runtime_id, &observer);
        process_runtime_ingress_observer(runtime_id).unwrap();

        let callback_count = callback_count.load(Ordering::Relaxed);
        assert_eq!(callback_count, 1);

        cleanup_runtime_ingress_observers(runtime_id);
    }

    #[test]
    fn test_process_runtime_ingress_observers_notifies_all_registered_runtimes() {
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

        register_runtime_ingress_observer(first_runtime_id, &first_observer);
        register_runtime_ingress_observer(second_runtime_id, &second_observer);
        process_runtime_ingress_observers().unwrap();

        let first_callback_count = first_callback_count.load(Ordering::Relaxed);
        let second_callback_count = second_callback_count.load(Ordering::Relaxed);
        assert_eq!(first_callback_count, 1);
        assert_eq!(second_callback_count, 1);

        cleanup_runtime_ingress_observers(first_runtime_id);
        cleanup_runtime_ingress_observers(second_runtime_id);
    }
}
