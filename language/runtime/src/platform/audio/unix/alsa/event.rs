#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::PlatformError;
#[cfg(target_os = "linux")]
use crate::platform::audio as audio_types;
#[cfg(not(target_os = "linux"))]
use crate::platform::audio::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::monitor::AudioMonitorHandle;
#[cfg(target_os = "linux")]
use crate::platform::core as core_platform;
#[cfg(target_os = "linux")]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(target_os = "linux")]
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};
#[cfg(target_os = "linux")]
#[cfg(target_os = "linux")]
use std::ffi::CString;
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(target_os = "linux")]
use std::thread::{self, JoinHandle};

/// One shared ALSA monitor state.
#[cfg(target_os = "linux")]
#[derive(Debug)]
struct AlsaDeviceMonitor {
    /// Stop signal for the monitor thread.
    stop: Arc<AtomicBool>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
}

#[cfg(target_os = "linux")]
impl AudioMonitorHandle for AlsaDeviceMonitor {
    fn stop(self: Box<Self>) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.handle.join();
    }
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

/// Start ALSA native device-event monitoring.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    #[cfg(target_os = "linux")]
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_signal = Arc::clone(&stop);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let handle = start_with_policy(
            "destack-audio-alsa-monitor",
            "destack.audio.event.open",
            ExecutionPolicy::process(ExecutionMode::Loop),
            move || run_monitor_thread(stop_signal, ready_sender),
        )?;

        let ready_result = ready_receiver.recv().map_err(|_| {
            startup_error("ALSA native event monitor exited before startup completed")
        })?;
        if let Err(error) = ready_result {
            stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            return Err(error);
        }

        Ok(Box::new(AlsaDeviceMonitor { stop, handle }))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported("destack.audio.event.open", "alsa"))
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
        let _ = ready_sender.send(Err(core_platform::io_not_found(
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
            audio_core::publish_device_snapshot_native_if_service_live(
                audio_types::AudioBackend::Alsa,
            );
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
