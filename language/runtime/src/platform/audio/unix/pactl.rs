#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::PlatformError;
#[cfg(not(target_os = "linux"))]
use crate::platform::audio::backend::backend_not_supported;
use crate::platform::audio::core as audio_core;
#[cfg(target_os = "linux")]
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::core as core_platform;
#[cfg(target_os = "linux")]
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(target_os = "linux")]
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};

use crate::platform::audio as audio_types;
#[cfg(target_os = "linux")]
use std::io::{BufRead, BufReader};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(target_os = "linux")]
use std::thread::{self, JoinHandle};

/// One persistent pactl monitor worker.
#[cfg(target_os = "linux")]
#[derive(Debug)]
struct PactlDeviceMonitor {
    /// Stop signal for the worker thread.
    stop: Arc<AtomicBool>,
    /// Running child process id.
    process_id: Arc<AtomicU32>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
}

#[cfg(target_os = "linux")]
impl AudioMonitorHandle for PactlDeviceMonitor {
    fn stop(self: Box<Self>) {
        self.stop.store(true, Ordering::Relaxed);
        kill_process(self.process_id.load(Ordering::Acquire));
        let _ = self.handle.join();
    }
}

/// Return one startup error payload for one pactl monitor failure.
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

/// Start native device-event monitoring for one pactl-based backend.
pub(crate) fn start_native_device_event_monitor(
    backend: audio_types::AudioBackend,
    backend_name: &'static str,
) -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    #[cfg(target_os = "linux")]
    {
        let _ = backend_name;
        let stop = Arc::new(AtomicBool::new(false));
        let process_id = Arc::new(AtomicU32::new(0));
        let stop_signal = Arc::clone(&stop);
        let process_id_signal = Arc::clone(&process_id);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let handle = start_with_policy(
            "destack-audio-pactl-monitor",
            "destack.audio.event.open",
            ExecutionPolicy::process(ExecutionMode::Loop),
            move || run_pactl_monitor_thread(backend, stop_signal, process_id_signal, ready_sender),
        )?;

        let ready_result = ready_receiver
            .recv()
            .map_err(|_| startup_error("pactl monitor exited before startup completed"))?;
        if let Err(error) = ready_result {
            stop.store(true, Ordering::Relaxed);
            kill_process(process_id.load(Ordering::Acquire));
            let _ = handle.join();
            return Err(error);
        }

        Ok(Box::new(PactlDeviceMonitor {
            stop,
            process_id,
            handle,
        }))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported(
            "destack.audio.event.open",
            backend_name,
        ))
    }
}

/// Run one pactl monitor worker thread for one backend.
#[cfg(target_os = "linux")]
fn run_pactl_monitor_thread(
    backend: audio_types::AudioBackend,
    stop: Arc<AtomicBool>,
    process_id: Arc<AtomicU32>,
    ready_sender: SyncSender<RuntimeResult<()>>,
) {
    // start one pactl subscribe process
    let child = Command::new("pactl")
        .arg("subscribe")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            let startup = if error.kind() == std::io::ErrorKind::NotFound {
                core_platform::io_not_found(
                    "destack.audio.event.open",
                    "pactl executable was not found on this host",
                )
            } else if error.kind() == std::io::ErrorKind::PermissionDenied {
                RuntimeError::from(PlatformError::io_with(
                    Some(PlatformErrorCode::IoPermissionDenied),
                    None,
                    None,
                    Some("destack.audio.event.open".to_string()),
                    None,
                    format!("failed to execute pactl subscribe: {error}"),
                ))
                .boxed()
            } else {
                startup_error(format!("failed to execute pactl subscribe: {error}"))
            };
            let _ = ready_sender.send(Err(startup));
            return;
        }
    };

    // take one stdout pipe for line-based monitor parsing
    let stdout = child.stdout.take();
    let Some(stdout) = stdout else {
        let _ = child.kill();
        let _ = child.wait();
        let _ = ready_sender.send(Err(startup_error("pactl monitor failed to capture stdout")));
        return;
    };

    process_id.store(child.id(), Ordering::Release);
    let _ = ready_sender.send(Ok(()));

    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        // stop path: terminate by external process kill from stop request
        if stop.load(Ordering::Relaxed) {
            break;
        }

        line.clear();
        let read_result = reader.read_line(&mut line);
        let bytes_read = match read_result {
            Ok(bytes_read) => bytes_read,
            Err(_) => break,
        };
        if bytes_read == 0 {
            break;
        }

        // forward device-related subscription notifications into runtime snapshots
        if should_publish_snapshot(&line) {
            let _ = std::panic::catch_unwind(|| {
                audio_core::publish_device_snapshot_native_if_service_live(backend);
            });
        }
    }

    let _ = child.kill();
    let _ = child.wait();
    process_id.store(0, Ordering::Release);
}

/// Return whether one pactl subscribe line should trigger one snapshot publish.
#[cfg(target_os = "linux")]
fn should_publish_snapshot(line: &str) -> bool {
    let line = line.trim().to_ascii_lowercase();
    if line.is_empty() {
        return false;
    }

    line.contains(" on sink")
        || line.contains(" on source")
        || line.contains(" on server")
        || line.contains(" on card")
        || line.contains(" on module")
}

/// Terminate one spawned process id when present.
#[cfg(target_os = "linux")]
fn kill_process(process_id: u32) {
    if process_id == 0 {
        return;
    }

    unsafe {
        let _ = libc::kill(process_id as i32, libc::SIGTERM);
    }
}
