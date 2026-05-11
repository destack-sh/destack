use super::constants::ASIO_REGISTRY_PATH;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::service::windows::WindowsRegisteredWait;

use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::platform::audio as audio_types;
use parking_lot::Mutex;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_NOTIFY, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    REG_NOTIFY_CHANGE_LAST_SET, REG_NOTIFY_CHANGE_NAME, RegCloseKey, RegNotifyChangeKeyValue,
    RegOpenKeyExW,
};
use windows_sys::Win32::System::Threading::{CreateEventW, ResetEvent};

/// One opened ASIO registry watcher.
struct AsioRegistryWatcher {
    /// Opened ASIO registry key handle.
    key: HKEY,
    /// Event handle notified on key changes.
    event: HANDLE,
    /// Registered shared wait callback when one is active.
    wait: Mutex<Option<WindowsRegisteredWait>>,
    /// Whether shutdown has started for this watcher.
    is_shutdown: AtomicBool,
}

// NOTE #Architecture: registry keys and wait events are host handles owned by this watcher.
// Access is synchronized externally, and callbacks may arrive from shared windows wait threads.
unsafe impl Send for AsioRegistryWatcher {}

// NOTE #Architecture: the watcher state is only mutated through atomics or the wait mutex.
unsafe impl Sync for AsioRegistryWatcher {}

impl AsioRegistryWatcher {
    /// Build one opened ASIO registry watcher for one registry view.
    fn open(open_flags: u32) -> Option<Arc<Self>> {
        let mut key = 0 as HKEY;
        let path = core_platform::wide_with_nul(ASIO_REGISTRY_PATH);
        let open_status =
            unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), 0, open_flags, &mut key) };
        if open_status != 0 || key == 0 {
            return None;
        }

        // create one auto-reset event for registry notifications
        let event = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
        if event == 0 {
            unsafe {
                RegCloseKey(key);
            }

            return None;
        }

        Some(Arc::new(Self {
            key,
            event,
            wait: Mutex::new(None),
            is_shutdown: AtomicBool::new(false),
        }))
    }

    /// Arm one registry-change notification and shared wait callback.
    fn arm(self: &Arc<Self>) -> RuntimeResult<()> {
        // stop rearming after shutdown
        if self.is_shutdown.load(Ordering::Acquire) {
            return Ok(());
        }

        // reset the event before registering the next notification
        let reset_status = unsafe { ResetEvent(self.event) };
        if reset_status == 0 {
            return Err(startup_error("failed to reset ASIO registry watcher event"));
        }

        // request the next key-change notification
        let notify_status = unsafe {
            RegNotifyChangeKeyValue(
                self.key,
                1,
                REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                self.event,
                1,
            )
        };
        if notify_status != 0 {
            return Err(startup_error(format!(
                "failed to register ASIO registry notification (status {notify_status})",
            )));
        }

        // register one one-shot shared wait callback for this event
        let watcher_pointer = Arc::as_ptr(self).cast_mut().cast::<c_void>();
        let registered_wait = WindowsRegisteredWait::register(
            "destack.audio.event.open",
            self.event,
            Some(asio_registry_wait_callback),
            watcher_pointer,
        )?;

        // publish the wait unless shutdown won the race
        let mut wait = self.wait.lock();
        if self.is_shutdown.load(Ordering::Acquire) {
            let mut registered_wait = registered_wait;
            let _ = registered_wait.unregister("destack.audio.event.close");

            return Ok(());
        }

        *wait = Some(registered_wait);

        Ok(())
    }

    /// Process one registry watcher callback and rearm the next wait.
    fn process_callback(self: &Arc<Self>) {
        // ignore late callbacks after shutdown
        if self.is_shutdown.load(Ordering::Acquire) {
            return;
        }

        // retire the current one-shot wait before rearming
        *self.wait.lock() = None;

        // publish the new device snapshot through the shared audio monitor service
        core_platform::callback_boundary(|| {
            audio_core::publish_device_snapshot_native_if_service_live(
                audio_types::AudioBackend::Asio,
            );
        });

        if let Err(error) = self.arm() {
            tracing::warn!("ASIO registry watcher rearm failed: {error}");
        }
    }

    /// Shut down one watcher and release its native handles.
    fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::Release);

        // wait for any active callback to retire before closing host handles
        if let Some(mut wait) = self.wait.lock().take()
            && let Err(error) = wait.unregister("destack.audio.event.close")
        {
            tracing::warn!("ASIO registry watcher unregistration failed: {error}");
        }

        unsafe {
            if self.event != 0 {
                let _ = CloseHandle(self.event);
            }
            if self.key != 0 {
                RegCloseKey(self.key);
            }
        }
    }
}

/// One shared ASIO monitor state.
struct AsioDeviceMonitor {
    /// Opened registry watchers across registry views.
    watchers: Vec<Arc<AsioRegistryWatcher>>,
}

impl AudioMonitorHandle for AsioDeviceMonitor {
    fn stop(self: Box<Self>) {
        for watcher in &self.watchers {
            watcher.shutdown();
        }
    }
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

/// Start ASIO native device-event monitoring.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    let mut watchers = Vec::new();

    // registry watchers
    if let Some(watcher) = AsioRegistryWatcher::open(KEY_READ | KEY_NOTIFY | KEY_WOW64_64KEY) {
        watchers.push(watcher);
    }
    if let Some(watcher) = AsioRegistryWatcher::open(KEY_READ | KEY_NOTIFY | KEY_WOW64_32KEY) {
        watchers.push(watcher);
    }

    // require at least one registry view to exist
    if watchers.is_empty() {
        return Err(core_platform::io_not_found(
            "destack.audio.event.open",
            "ASIO registry key was not found",
        ));
    }

    // arm every live watcher before publishing the monitor handle
    for watcher in &watchers {
        if let Err(error) = watcher.arm() {
            for watcher in &watchers {
                watcher.shutdown();
            }

            return Err(error);
        }
    }

    Ok(Box::new(AsioDeviceMonitor { watchers }))
}

/// Dispatch one shared registry wait callback.
unsafe extern "system" fn asio_registry_wait_callback(context: *mut c_void, _timed_out: u8) {
    if context.is_null() {
        return;
    }

    let watcher_pointer = context.cast::<AsioRegistryWatcher>();

    // retain one temporary strong reference while the callback runs
    unsafe {
        Arc::increment_strong_count(watcher_pointer);
    }
    let watcher = unsafe { Arc::from_raw(watcher_pointer) };

    watcher.process_callback();
}
