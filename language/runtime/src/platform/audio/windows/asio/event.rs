use super::constants::ASIO_REGISTRY_PATH;
use super::core::is_backend_supported as backend_supported;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_NOTIFY, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RegCloseKey, RegNotifyChangeKeyValue,
    RegOpenKeyExW,
};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent, WaitForMultipleObjects};

/// One opened ASIO registry watcher row.
struct AsioRegistryWatcher {
    /// Opened ASIO registry key handle.
    key: HKEY,
    /// Event handle notified on key changes.
    event: HANDLE,
}

/// One shared ASIO monitor state.
#[derive(Debug)]
struct AsioDeviceMonitor {
    /// Stop signal for the monitor thread.
    stop: Arc<AtomicBool>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
    /// Active reference count for subscriptions using this monitor.
    reference_count: usize,
}

/// Runtime-owned ASIO monitor state.
#[derive(Debug, Default)]
struct AsioMonitorRuntimeState {
    /// Runtime-owned monitor slot.
    monitor: Mutex<Option<AsioDeviceMonitor>>,
    /// Whether teardown finalizer was registered.
    shutdown_registered: AtomicBool,
}

/// Return runtime-owned ASIO monitor state.
fn asio_monitor_runtime_state(context: &BindingCallContext) -> Arc<AsioMonitorRuntimeState> {
    let runtime_state = context
        .runtime()
        .module_state
        .get_or_init(AsioMonitorRuntimeState::default);
    register_runtime_finalizer(context, &runtime_state);

    runtime_state
}

/// Register one runtime finalizer for ASIO monitor teardown.
fn register_runtime_finalizer(
    context: &BindingCallContext,
    runtime_state: &Arc<AsioMonitorRuntimeState>,
) {
    if runtime_state
        .shutdown_registered
        .swap(true, Ordering::AcqRel)
    {
        return;
    }

    let runtime_state = Arc::clone(runtime_state);
    context.runtime().finalizers.register(move || {
        let monitor = runtime_state
            .monitor
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        let Some(monitor) = monitor else {
            return;
        };

        monitor.stop.store(true, Ordering::Relaxed);
        let _ = monitor.handle.join();
    });
}

/// Return one startup error payload for ASIO monitor initialization.
fn startup_error(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some("destack.audio.event.open".to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Return whether ASIO native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    backend_supported()
}

/// Start ASIO native device-event monitoring.
pub(crate) fn start_native_device_event_monitor(context: &BindingCallContext) -> RuntimeResult<()> {
    let runtime_state = asio_monitor_runtime_state(context);
    let mut monitor_slot = runtime_state
        .monitor
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(monitor) = monitor_slot.as_mut() {
        monitor.reference_count += 1;
        return Ok(());
    }

    let stop = Arc::new(AtomicBool::new(false));
    let runtime_state = audio_core::audio_event_runtime_state(context);
    let stop_signal = Arc::clone(&stop);
    let callback_runtime_state = Arc::clone(&runtime_state);
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let handle = thread::spawn(move || {
        run_monitor_thread(callback_runtime_state, stop_signal, ready_sender)
    });

    let ready_result = ready_receiver
        .recv()
        .map_err(|_| startup_error("ASIO native event monitor exited before startup completed"))?;
    if let Err(error) = ready_result {
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        return Err(error);
    }

    *monitor_slot = Some(AsioDeviceMonitor {
        stop,
        handle,
        reference_count: 1,
    });

    Ok(())
}

/// Stop ASIO native device-event monitoring.
pub(crate) fn stop_native_device_event_monitor(_context: &BindingCallContext) {
    let runtime_state = asio_monitor_runtime_state(_context);
    let monitor = {
        let mut monitor_slot = runtime_state
            .monitor
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some(monitor) = monitor_slot.as_mut() else {
            return;
        };

        if monitor.reference_count > 1 {
            monitor.reference_count -= 1;
            return;
        }

        monitor_slot.take()
    };

    let Some(monitor) = monitor else {
        return;
    };

    monitor.stop.store(true, Ordering::Relaxed);
    let _ = monitor.handle.join();
}

/// Run one ASIO registry monitor thread.
fn run_monitor_thread(
    runtime_state: Arc<audio_core::AudioEventRuntimeState>,
    stop: Arc<AtomicBool>,
    ready_sender: SyncSender<RuntimeResult<()>>,
) {
    // open registry watchers for both 64-bit and 32-bit views
    let mut watchers = Vec::new();
    append_registry_watcher(&mut watchers, KEY_READ | KEY_NOTIFY | KEY_WOW64_64KEY);
    append_registry_watcher(&mut watchers, KEY_READ | KEY_NOTIFY | KEY_WOW64_32KEY);

    if watchers.is_empty() {
        let _ = ready_sender.send(Err(audio_core::audio_not_found(
            "destack.audio.event.open",
            "ASIO registry key was not found",
        )));
        return;
    }

    // arm all registry watcher events before reporting startup completion
    for watcher in &watchers {
        if let Err(error) = arm_registry_watcher(watcher) {
            cleanup_watchers(&mut watchers);
            let _ = ready_sender.send(Err(error));
            return;
        }
    }

    let _ = ready_sender.send(Ok(()));
    while !stop.load(Ordering::Relaxed) {
        let wait_handles = watchers
            .iter()
            .map(|watcher| watcher.event)
            .collect::<Vec<_>>();

        // wait for one registry-change notification event
        let wait_status = unsafe {
            WaitForMultipleObjects(wait_handles.len() as u32, wait_handles.as_ptr(), 0, 100)
        };
        if wait_status == WAIT_TIMEOUT {
            continue;
        }

        if wait_status >= WAIT_OBJECT_0.saturating_add(wait_handles.len() as u32) {
            break;
        }

        let watcher_index = (wait_status - WAIT_OBJECT_0) as usize;
        if watcher_index >= watchers.len() {
            break;
        }

        let _ = std::panic::catch_unwind(|| {
            audio_core::publish_device_snapshot_native(
                &runtime_state,
                audio_core::AudioBackend::Asio,
            );
        });

        if arm_registry_watcher(&watchers[watcher_index]).is_err() {
            break;
        }
    }

    cleanup_watchers(&mut watchers);
}

/// Append one ASIO registry watcher for one registry view.
fn append_registry_watcher(watchers: &mut Vec<AsioRegistryWatcher>, open_flags: u32) {
    let mut key = 0 as HKEY;
    let path = core_platform::wide_with_nul(ASIO_REGISTRY_PATH);
    let open_status =
        unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), 0, open_flags, &mut key) };
    if open_status != 0 || key == 0 {
        return;
    }

    // create one auto-reset event for registry notifications
    let event = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
    if event == 0 {
        unsafe {
            RegCloseKey(key);
        }
        return;
    }

    watchers.push(AsioRegistryWatcher { key, event });
}

/// Arm one registry watcher event for one subsequent change.
fn arm_registry_watcher(watcher: &AsioRegistryWatcher) -> RuntimeResult<()> {
    unsafe {
        let _ = ResetEvent(watcher.event);
    }

    let notify_status = unsafe {
        RegNotifyChangeKeyValue(
            watcher.key,
            1,
            REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
            watcher.event,
            1,
        )
    };
    if notify_status == 0 {
        return Ok(());
    }

    Err(startup_error(format!(
        "failed to register ASIO registry notification (status {notify_status})",
    )))
}

/// Close one watcher set and release all owned handles.
fn cleanup_watchers(watchers: &mut Vec<AsioRegistryWatcher>) {
    for watcher in watchers.drain(..) {
        unsafe {
            if watcher.event != 0 {
                let _ = CloseHandle(watcher.event);
            }
            if watcher.key != 0 {
                RegCloseKey(watcher.key);
            }
        }
    }
}
