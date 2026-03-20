use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::Mutex;

/// Shared callback queue for one windows loop service.
#[derive(Default)]
pub(in super::super) struct WindowsLoopQueue {
    /// Pending callbacks for one bound windows thread.
    callbacks: Mutex<VecDeque<WindowsLoopCallback>>,
}

/// One queued windows loop callback.
type WindowsLoopCallback = Box<dyn FnOnce() + Send + 'static>;

/// Shared registry of windows loop callback queues.
struct WindowsLoopRegistry {
    /// Live loop queues keyed by windows thread id.
    queues_by_thread: Mutex<BTreeMap<u32, Vec<Weak<WindowsLoopQueue>>>>,
}

/// Global registry of windows loop callback queues.
static WINDOWS_LOOP_REGISTRY: OnceLock<WindowsLoopRegistry> = OnceLock::new();

/// Service-dispatch windows thread message id.
pub(crate) const WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID: u32 =
    windows_sys::Win32::UI::WindowsAndMessaging::WM_APP + 0x2541;

/// Register one windows loop callback queue.
pub(in super::super) fn register_windows_loop_queue(thread_id: u32, queue: &Arc<WindowsLoopQueue>) {
    let registry = windows_loop_registry();
    let mut queues_by_thread = registry.queues_by_thread.lock();

    // prune dead queues while updating this thread entry
    queues_by_thread.retain(|_, queues| {
        queues.retain(|weak| weak.upgrade().is_some());
        !queues.is_empty()
    });

    let queues = queues_by_thread.entry(thread_id).or_default();
    let queue_pointer = Arc::as_ptr(queue) as *const ();
    let is_registered = queues.iter().any(|weak| {
        let Some(existing) = weak.upgrade() else {
            return false;
        };

        Arc::as_ptr(&existing) as *const () == queue_pointer
    });

    if !is_registered {
        queues.push(Arc::downgrade(queue));
    }
}

/// Drain queued windows loop callbacks for the current thread.
pub(crate) fn process_windows_loop_callbacks() -> bool {
    let current_windows_thread_id =
        unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
    let callbacks = take_windows_loop_callbacks(current_windows_thread_id);

    // run every queued callback for this host-loop thread
    let mut dispatched_any = false;
    for callback in callbacks {
        callback();
        dispatched_any = true;
    }

    dispatched_any
}

/// Return one shared windows loop queue.
pub(in super::super) fn windows_loop_queue() -> Arc<WindowsLoopQueue> {
    Arc::new(WindowsLoopQueue::default())
}

impl WindowsLoopQueue {
    /// Enqueue one callback for later service on the bound windows loop.
    pub(in super::super) fn enqueue(&self, callback: WindowsLoopCallback) {
        let mut callbacks = self.callbacks.lock();

        callbacks.push_back(callback);
    }
}

/// Return the shared windows loop callback registry.
fn windows_loop_registry() -> &'static WindowsLoopRegistry {
    WINDOWS_LOOP_REGISTRY.get_or_init(|| WindowsLoopRegistry {
        queues_by_thread: Mutex::new(BTreeMap::new()),
    })
}

/// Drain every queued windows loop callback for one thread id.
fn take_windows_loop_callbacks(thread_id: u32) -> Vec<WindowsLoopCallback> {
    let registry = windows_loop_registry();
    let queues = {
        let mut queues_by_thread = registry.queues_by_thread.lock();
        let Some(queues) = queues_by_thread.get_mut(&thread_id) else {
            return Vec::new();
        };

        queues.retain(|weak| weak.upgrade().is_some());
        if queues.is_empty() {
            queues_by_thread.remove(&thread_id);
            return Vec::new();
        }

        queues.iter().filter_map(Weak::upgrade).collect::<Vec<_>>()
    };

    // drain every live callback queue outside the registry lock
    let mut callbacks = Vec::new();
    for queue in queues {
        let mut pending = queue.callbacks.lock();

        while let Some(callback) = pending.pop_front() {
            callbacks.push(callback);
        }
    }

    callbacks
}
