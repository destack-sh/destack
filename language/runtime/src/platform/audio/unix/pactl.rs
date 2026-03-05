#[cfg(not(target_os = "linux"))]
use super::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use crate::diagnostic::RuntimeError;
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;
use crate::platform::core as core_platform;
#[cfg(target_os = "linux")]
use crate::platform::diagnostic::PlatformErrorCode;
use crate::runtime::BindingCallContext;

#[cfg(target_os = "linux")]
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::io::{BufRead, BufReader};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};
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
    /// Active reference count for subscriptions using this monitor.
    reference_count: usize,
}

/// One shared pactl monitor slot keyed by audio backend.
#[cfg(target_os = "linux")]
#[derive(Debug, Default)]
pub(crate) struct PactlMonitorRuntimeState {
    /// Runtime-owned pactl monitor workers keyed by backend.
    monitors: Mutex<HashMap<audio_core::AudioBackend, PactlDeviceMonitor>>,
    /// Whether teardown finalizer was registered.
    shutdown_registered: AtomicBool,
}

/// Return runtime-owned pactl monitor state.
#[cfg(target_os = "linux")]
fn pactl_monitor_runtime_state(binding: &BindingCallContext) -> Arc<PactlMonitorRuntimeState> {
    let runtime_state = binding
        .agent()
        .platform_state
        .audio
        .pactl_monitor_runtime_state(PactlMonitorRuntimeState::default);
    register_runtime_finalizer(binding, &runtime_state);

    runtime_state
}

/// Register one runtime finalizer for pactl monitor teardown.
#[cfg(target_os = "linux")]
fn register_runtime_finalizer(
    binding: &BindingCallContext,
    runtime_state: &Arc<PactlMonitorRuntimeState>,
) {
    if runtime_state
        .shutdown_registered
        .swap(true, Ordering::AcqRel)
    {
        return;
    }

    let runtime_state = Arc::clone(runtime_state);
    binding.agent().finalizers.register(move || {
        let mut monitor_registry = runtime_state
            .monitors
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let monitors = monitor_registry
            .drain()
            .map(|(_, monitor)| monitor)
            .collect::<Vec<_>>();
        drop(monitor_registry);

        for monitor in monitors {
            monitor.stop.store(true, Ordering::Relaxed);
            kill_process(monitor.process_id.load(Ordering::Acquire));
            let _ = monitor.handle.join();
        }
    });
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

/// Return whether native device-event monitoring is available for one pactl-based backend.
pub(crate) fn native_device_events_supported(backend: audio_core::AudioBackend) -> bool {
    #[cfg(target_os = "linux")]
    {
        if backend != audio_core::AudioBackend::PulseAudio
            && backend != audio_core::AudioBackend::PipeWire
        {
            return false;
        }

        return super::backend::backend_supported(backend);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = backend;
        false
    }
}

/// Start native device-event monitoring for one pactl-based backend.
pub(crate) fn start_native_device_event_monitor(
    binding: &BindingCallContext,
    backend: audio_core::AudioBackend,
    backend_name: &'static str,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let _ = backend_name;
        let runtime_state = pactl_monitor_runtime_state(binding);

        let mut monitor_slot = runtime_state
            .monitors
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(monitor) = monitor_slot.get_mut(&backend) {
            monitor.reference_count += 1;
            return Ok(());
        }

        let stop = Arc::new(AtomicBool::new(false));
        let runtime_state = audio_core::audio_event_runtime_state(binding);
        let process_id = Arc::new(AtomicU32::new(0));
        let stop_signal = Arc::clone(&stop);
        let callback_runtime_state = Arc::clone(&runtime_state);
        let process_id_signal = Arc::clone(&process_id);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            run_pactl_monitor_thread(
                callback_runtime_state,
                backend,
                stop_signal,
                process_id_signal,
                ready_sender,
            )
        });

        let ready_result = ready_receiver
            .recv()
            .map_err(|_| startup_error("pactl monitor exited before startup completed"))?;
        if let Err(error) = ready_result {
            stop.store(true, Ordering::Relaxed);
            kill_process(process_id.load(Ordering::Acquire));
            let _ = handle.join();
            return Err(error);
        }

        monitor_slot.insert(
            backend,
            PactlDeviceMonitor {
                stop,
                process_id,
                handle,
                reference_count: 1,
            },
        );

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = backend;
        Err(backend_not_supported(
            "destack.audio.event.open",
            backend_name,
        ))
    }
}

/// Stop native device-event monitoring for one pactl-based backend.
pub(crate) fn stop_native_device_event_monitor(
    binding: &BindingCallContext,
    backend: audio_core::AudioBackend,
) {
    #[cfg(target_os = "linux")]
    {
        let runtime_state = pactl_monitor_runtime_state(binding);
        let monitor = {
            let mut monitor_slot = runtime_state
                .monitors
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let Some(monitor) = monitor_slot.get_mut(&backend) else {
                return;
            };

            if monitor.reference_count > 1 {
                monitor.reference_count -= 1;
                return;
            }

            monitor_slot.remove(&backend)
        };

        let Some(monitor) = monitor else {
            return;
        };

        monitor.stop.store(true, Ordering::Relaxed);
        kill_process(monitor.process_id.load(Ordering::Acquire));
        let _ = monitor.handle.join();
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = backend;
    }
}

/// Run one pactl monitor worker thread for one backend.
#[cfg(target_os = "linux")]
fn run_pactl_monitor_thread(
    runtime_state: Arc<audio_core::AudioEventRuntimeState>,
    backend: audio_core::AudioBackend,
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
                audio_core::publish_device_snapshot_native(&runtime_state, backend);
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
