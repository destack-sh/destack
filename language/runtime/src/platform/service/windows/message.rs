use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Weak};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};
use crate::platform::service::global_service;

use super::super::executor::HostLoopExecutor;

/// Shared callback queue for one windows loop service.
#[derive(Default)]
pub(crate) struct WindowsLoopQueue {
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

/// Service-dispatch windows thread message id.
pub(crate) const WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID: u32 =
    windows_sys::Win32::UI::WindowsAndMessaging::WM_APP + 0x2541;

/// Bind the current windows message loop thread on first use.
pub(crate) fn try_bind_windows_message_loop(service: &HostLoopExecutor) -> bool {
    if service.is_current_bound_thread() {
        return true;
    }

    if service.is_bound.load(std::sync::atomic::Ordering::Acquire) {
        return false;
    }

    let current_thread_id = std::thread::current().id();
    let current_windows_thread_id =
        unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
    let mut message =
        unsafe { std::mem::zeroed::<windows_sys::Win32::UI::WindowsAndMessaging::MSG>() };

    // create the thread message queue before other threads post callbacks to it
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW(
            &mut message,
            0,
            0,
            0,
            windows_sys::Win32::UI::WindowsAndMessaging::PM_NOREMOVE,
        );
    }

    // bind the first observed windows message loop thread
    if service.thread_id.set(current_thread_id).is_err() {
        return service.is_current_bound_thread();
    }

    let _windows_thread_id_set_result = service.windows_thread_id.set(current_windows_thread_id);
    register_windows_loop_queue(service);
    service
        .is_bound
        .store(true, std::sync::atomic::Ordering::Release);

    true
}

/// Register one windows loop callback queue.
pub(crate) fn register_windows_loop_queue(service: &HostLoopExecutor) {
    let Some(thread_id) = service.windows_thread_id.get().copied() else {
        return;
    };

    let registry = windows_loop_registry();
    let mut queues_by_thread = registry.queues_by_thread.lock();

    // prune dead queues while updating this thread entry
    queues_by_thread.retain(|_, queues| {
        queues.retain(|weak| weak.upgrade().is_some());
        !queues.is_empty()
    });

    let queues = queues_by_thread.entry(thread_id).or_default();
    let queue_pointer = Arc::as_ptr(&service.windows_queue) as *const ();
    let is_registered = queues.iter().any(|weak| {
        let Some(existing) = weak.upgrade() else {
            return false;
        };

        Arc::as_ptr(&existing) as *const () == queue_pointer
    });

    if !is_registered {
        queues.push(Arc::downgrade(&service.windows_queue));
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
pub(crate) fn windows_loop_queue() -> Arc<WindowsLoopQueue> {
    Arc::new(WindowsLoopQueue::default())
}

/// Execute one callback on the bound windows message loop.
pub(crate) fn call_process_windows_message_loop<R>(
    operation: &'static str,
    service: &HostLoopExecutor,
    callback: impl FnOnce() -> RuntimeResult<R> + Send + 'static,
) -> RuntimeResult<R>
where
    R: Send + 'static,
{
    // run directly when the current thread already owns the host loop
    if service.try_bind_windows_message_loop() || service.is_current_bound_thread() {
        return callback();
    }

    let Some(thread_id) = service.windows_thread_id.get().copied() else {
        return Err(core_platform::io_operation_error(
            operation,
            None,
            format!(
                "loop service {} has no bound windows message loop thread",
                service.name
            ),
        ));
    };

    let (result_tx, result_rx) = sync_channel::<RuntimeResult<R>>(1);

    // queue one callback for the bound windows message loop
    {
        let mut callbacks = service.windows_queue.callbacks.lock();
        callbacks.push_back(Box::new(move || {
            if result_tx.send(callback()).is_err() {
                return;
            }
        }));
    }

    // wake the bound message loop thread to service the callback
    let dispatched = unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
            thread_id,
            WINDOWS_HOST_LOOP_SERVICE_MESSAGE_ID,
            0,
            0,
        )
    };
    if dispatched == 0 {
        return Err(core_platform::io_operation_error(
            operation,
            None,
            format!(
                "failed to post loop callback to windows thread {thread_id}: {}",
                core_platform::last_error_code()
            ),
        ));
    }

    // wait for the windows loop callback result
    result_rx.recv().map_err(|error| {
        core_platform::io_operation_error(
            operation,
            None,
            format!(
                "failed to receive windows loop callback result for {}: {error}",
                service.name
            ),
        )
    })?
}

/// Return the shared windows loop callback registry.
fn windows_loop_registry() -> Arc<WindowsLoopRegistry> {
    global_service(|| {
        Ok(WindowsLoopRegistry {
            queues_by_thread: Mutex::new(BTreeMap::new()),
        })
    })
    .expect("windows loop registry initialization should succeed")
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
