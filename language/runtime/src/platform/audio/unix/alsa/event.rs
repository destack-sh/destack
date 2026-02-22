#[cfg(not(target_os = "linux"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use super::core::is_backend_supported as backend_supported;
#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::PlatformError;
#[cfg(target_os = "linux")]
use crate::platform::audio::core as audio_core;
#[cfg(target_os = "linux")]
use crate::platform::diagnostic::PlatformErrorCode;

#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(target_os = "linux")]
use std::thread::{self, JoinHandle};

/// One shared ALSA monitor state.
#[cfg(target_os = "linux")]
struct AlsaDeviceMonitor {
    /// Stop signal for the monitor thread.
    stop: Arc<AtomicBool>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
    /// Active reference count for subscriptions using this monitor.
    reference_count: usize,
}

/// One shared ALSA monitor slot.
#[cfg(target_os = "linux")]
static ALSA_DEVICE_MONITOR_SLOT: OnceLock<Mutex<Option<AlsaDeviceMonitor>>> = OnceLock::new();

/// Return one shared ALSA monitor slot.
#[cfg(target_os = "linux")]
fn monitor_slot() -> &'static Mutex<Option<AlsaDeviceMonitor>> {
    ALSA_DEVICE_MONITOR_SLOT.get_or_init(|| Mutex::new(None))
}

/// Return one startup error payload for ALSA monitor initialization.
#[cfg(target_os = "linux")]
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

/// Return whether ALSA native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        return backend_supported();
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Start ALSA native device-event monitoring.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let mut monitor_slot = monitor_slot()
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(monitor) = monitor_slot.as_mut() {
            monitor.reference_count += 1;
            return Ok(());
        }

        let stop = Arc::new(AtomicBool::new(false));
        let stop_signal = Arc::clone(&stop);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || run_monitor_thread(stop_signal, ready_sender));

        let ready_result = ready_receiver.recv().map_err(|_| {
            startup_error("ALSA native event monitor exited before startup completed")
        })?;
        if let Err(error) = ready_result {
            stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            return Err(error);
        }

        *monitor_slot = Some(AlsaDeviceMonitor {
            stop,
            handle,
            reference_count: 1,
        });

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported("destack.audio.event.open", "alsa"))
    }
}

/// Stop ALSA native device-event monitoring.
pub(crate) fn stop_native_device_event_monitor() {
    #[cfg(target_os = "linux")]
    {
        let monitor = {
            let mut monitor_slot = monitor_slot()
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
}

/// Run one inotify monitor worker for ALSA-related device tree paths.
#[cfg(target_os = "linux")]
fn run_monitor_thread(stop: Arc<AtomicBool>, ready_sender: SyncSender<RuntimeResult<()>>) {
    // open one inotify descriptor for device tree monitoring
    let inotify_fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
    if inotify_fd < 0 {
        let _ = ready_sender.send(Err(startup_error(
            "failed to initialize inotify for ALSA device monitoring",
        )));
        return;
    }

    let watch_mask = libc::IN_ATTRIB
        | libc::IN_CLOSE_WRITE
        | libc::IN_CREATE
        | libc::IN_DELETE
        | libc::IN_MOVED_FROM
        | libc::IN_MOVED_TO;

    let watch_paths = [
        "/dev/snd",
        "/dev",
        "/proc/asound",
        "/proc",
        "/etc/asound.conf",
    ];
    let mut watch_count = 0usize;
    for path in watch_paths {
        if add_watch(inotify_fd, path, watch_mask) {
            watch_count += 1;
        }
    }
    if watch_count == 0 {
        unsafe {
            libc::close(inotify_fd);
        }
        let _ = ready_sender.send(Err(audio_core::audio_not_found(
            "destack.audio.event.open",
            "no ALSA monitor paths were available for inotify",
        )));
        return;
    }

    let _ = ready_sender.send(Ok(()));

    let mut poll_descriptor = libc::pollfd {
        fd: inotify_fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let mut event_bytes = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        // poll one inotify descriptor with one bounded timeout
        let poll_status = unsafe { libc::poll(&mut poll_descriptor, 1, 100) };
        if poll_status <= 0 {
            continue;
        }

        if (poll_descriptor.revents & libc::POLLIN) == 0 {
            continue;
        }

        // drain one inotify event packet
        let read_count = unsafe {
            libc::read(
                inotify_fd,
                event_bytes.as_mut_ptr().cast::<libc::c_void>(),
                event_bytes.len(),
            )
        };
        if read_count <= 0 {
            continue;
        }

        let _ = std::panic::catch_unwind(|| {
            audio_core::publish_device_snapshot_native(audio_core::AudioBackend::Alsa);
        });
    }

    unsafe {
        libc::close(inotify_fd);
    }
}

/// Add one inotify watch for one filesystem path.
#[cfg(target_os = "linux")]
fn add_watch(inotify_fd: i32, path: &str, mask: u32) -> bool {
    let path = CString::new(path);
    let Ok(path) = path else {
        return false;
    };

    let descriptor = unsafe { libc::inotify_add_watch(inotify_fd, path.as_ptr(), mask) };
    descriptor >= 0
}
