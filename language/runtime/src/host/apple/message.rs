use std::sync::{Arc, OnceLock, Weak};

use parking_lot::RwLock;
use rustc_hash::FxHashMap;

/// CoreFoundation string reference type.
type CFStringRef = *const libc::c_void;

/// CoreFoundation run-loop handled-source status code.
const KCF_RUN_LOOP_RUN_HANDLED_SOURCE: i32 = 4;
/// CoreFoundation run-loop timeout status code.
const KCF_RUN_LOOP_RUN_TIMED_OUT: i32 = 3;
/// CoreFoundation run-loop finished status code.
const KCF_RUN_LOOP_RUN_FINISHED: i32 = 1;
/// CoreFoundation run-loop stopped status code.
const KCF_RUN_LOOP_RUN_STOPPED: i32 = 2;
/// Slice duration for bounded Apple run-loop servicing.
const APPLE_THREAD_MESSAGE_WAIT_SLICE_SECONDS: f64 = 0.001;

/// Shared runtime observer registry for Apple thread-message servicing.
static APPLE_THREAD_MESSAGE_OBSERVERS: OnceLock<RwLock<FxHashMap<u64, AppleThreadMessageEntry>>> =
    OnceLock::new();

/// Shared observer entry for one Apple runtime id.
type AppleThreadMessageEntry = Weak<dyn AppleThreadMessageObserver>;

/// Observer notified after one Apple thread-message pump dispatches host work.
pub(crate) trait AppleThreadMessageObserver: std::fmt::Debug + Send + Sync {
    /// Reconcile runtime-owned state after one message-pump step handled work.
    fn did_pump_thread_messages(&self);
}

// link corefoundation run-loop symbols used by host adapter message pumping
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    /// Run one CoreFoundation run-loop mode for one bounded interval.
    fn CFRunLoopRunInMode(
        mode: CFStringRef,
        seconds: f64,
        return_after_source_handled: bool,
    ) -> i32;
    /// Run one CoreFoundation run loop until explicit stop.
    fn CFRunLoopRun();
    /// Default run-loop mode used by platform thread sources.
    static kCFRunLoopDefaultMode: CFStringRef;
}

/// Drain pending platform thread messages without blocking.
pub(crate) fn pump_pending_thread_messages(ignore_quit_message: bool) -> bool {
    // ignore quit-message policy on run-loop platforms with no quit packets
    let _ = ignore_quit_message;
    let mut dispatched_any = false;

    loop {
        // dispatch one immediately ready source when available
        let status = unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.0, true) };
        if status == KCF_RUN_LOOP_RUN_HANDLED_SOURCE {
            dispatched_any = true;
            continue;
        }

        // otherwise no immediate source remained, stop the pump loop
        if status == KCF_RUN_LOOP_RUN_TIMED_OUT
            || status == KCF_RUN_LOOP_RUN_FINISHED
            || status == KCF_RUN_LOOP_RUN_STOPPED
        {
            break;
        }

        break;
    }

    dispatched_any
}

/// Drain pending platform thread messages for one runtime without blocking.
pub(crate) fn pump_pending_thread_messages_for_runtime(
    runtime_id: Option<u64>,
    ignore_quit_message: bool,
) -> bool {
    let dispatched_any = pump_pending_thread_messages(ignore_quit_message);

    // notify the runtime observer only when one source was actually handled
    if dispatched_any {
        if let Some(runtime_id) = runtime_id {
            notify_thread_message_observer(runtime_id);
        }
    }

    dispatched_any
}

/// Service Apple thread messages until one caller-provided stop condition becomes true.
pub(crate) fn service_registered_runtimes_until(
    ignore_quit_message: bool,
    mut should_stop: impl FnMut() -> bool,
) {
    // ignore quit-message policy on run-loop platforms with no quit packets
    let _ = ignore_quit_message;

    // keep the current run loop alive until the stop condition is satisfied
    while !should_stop() {
        let status = unsafe {
            CFRunLoopRunInMode(
                kCFRunLoopDefaultMode,
                APPLE_THREAD_MESSAGE_WAIT_SLICE_SECONDS,
                true,
            )
        };

        // notify runtime observers after one handled source
        if status == KCF_RUN_LOOP_RUN_HANDLED_SOURCE {
            notify_thread_message_observers();
            continue;
        }

        // continue after benign wait statuses
        if status == KCF_RUN_LOOP_RUN_TIMED_OUT
            || status == KCF_RUN_LOOP_RUN_FINISHED
            || status == KCF_RUN_LOOP_RUN_STOPPED
        {
            continue;
        }
    }
}

/// Run one blocking platform thread message loop.
pub(crate) fn run_blocking_thread_message_loop() {
    // run until the current run loop is explicitly stopped
    unsafe {
        CFRunLoopRun();
    }
}

/// Register one Apple thread-message observer for one runtime id.
pub(crate) fn register_thread_message_observer(
    runtime_id: u64,
    observer: &Arc<dyn AppleThreadMessageObserver>,
) {
    let mut observers = thread_message_observers().write();

    // keep only live observer entries while replacing this runtime binding
    observers.retain(|_, weak| weak.upgrade().is_some());
    observers.insert(runtime_id, Arc::downgrade(observer));
}

/// Remove one Apple thread-message observer registration.
pub(crate) fn unregister_thread_message_observer(runtime_id: u64) {
    let mut observers = thread_message_observers().write();
    observers.remove(&runtime_id);
}

/// Remove Apple thread-message servicing state for one runtime id.
pub(crate) fn cleanup_runtime(runtime_id: u64) {
    unregister_thread_message_observer(runtime_id);
}

/// Return the shared Apple thread-message observer registry.
fn thread_message_observers() -> &'static RwLock<FxHashMap<u64, AppleThreadMessageEntry>> {
    APPLE_THREAD_MESSAGE_OBSERVERS.get_or_init(|| RwLock::new(FxHashMap::default()))
}

/// Notify one Apple thread-message observer after the host serviced callbacks.
fn notify_thread_message_observer(runtime_id: u64) {
    let observer = {
        let mut observers = thread_message_observers().write();
        let Some(observer) = observers.get(&runtime_id).and_then(Weak::upgrade) else {
            observers.remove(&runtime_id);
            return;
        };

        observer
    };

    observer.did_pump_thread_messages();
}

/// Notify every registered Apple thread-message observer after the host serviced callbacks.
fn notify_thread_message_observers() {
    let observers = {
        let mut registered_observers = thread_message_observers().write();
        let mut live_observers = Vec::with_capacity(registered_observers.len());

        // retain only live observers before dispatching callbacks
        registered_observers.retain(|_, weak| {
            let Some(observer) = weak.upgrade() else {
                return false;
            };

            live_observers.push(observer);
            true
        });

        live_observers
    };

    // run callbacks outside the registry lock
    for observer in observers {
        observer.did_pump_thread_messages();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        AppleThreadMessageObserver, cleanup_runtime, notify_thread_message_observer,
        register_thread_message_observer,
    };

    /// Sentinel runtime id reserved for observer registration tests.
    const TEST_RUNTIME_ID_ONE: u64 = u64::MAX - 1;
    /// Sentinel runtime id reserved for observer cleanup tests.
    const TEST_RUNTIME_ID_TWO: u64 = u64::MAX;

    /// One test observer that counts host pump notifications.
    #[derive(Debug)]
    struct TestThreadMessageObserver {
        /// Per-test callback counter.
        callback_count: Arc<AtomicU64>,
    }

    impl AppleThreadMessageObserver for TestThreadMessageObserver {
        /// Count one observed host pump callback.
        fn did_pump_thread_messages(&self) {
            self.callback_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn test_register_thread_message_observer_notifies_registered_runtime() {
        let callback_count = Arc::new(AtomicU64::new(0));

        let observer: Arc<dyn AppleThreadMessageObserver> = Arc::new(TestThreadMessageObserver {
            callback_count: Arc::clone(&callback_count),
        });
        register_thread_message_observer(TEST_RUNTIME_ID_ONE, &observer);
        notify_thread_message_observer(TEST_RUNTIME_ID_ONE);

        let callback_count = callback_count.load(Ordering::Relaxed);
        assert_eq!(callback_count, 1);

        cleanup_runtime(TEST_RUNTIME_ID_ONE);
    }

    #[test]
    fn test_cleanup_runtime_unregisters_thread_message_observer() {
        let callback_count = Arc::new(AtomicU64::new(0));

        let observer: Arc<dyn AppleThreadMessageObserver> = Arc::new(TestThreadMessageObserver {
            callback_count: Arc::clone(&callback_count),
        });
        register_thread_message_observer(TEST_RUNTIME_ID_TWO, &observer);
        cleanup_runtime(TEST_RUNTIME_ID_TWO);
        notify_thread_message_observer(TEST_RUNTIME_ID_TWO);

        let callback_count = callback_count.load(Ordering::Relaxed);
        assert_eq!(callback_count, 0);
    }
}
