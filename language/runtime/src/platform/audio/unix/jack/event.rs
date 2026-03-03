#[cfg(not(target_os = "linux"))]
use super::super::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use super::abi::JackClient;
#[cfg(target_os = "linux")]
use super::core::{jack_error, jack_succeeded, require_jack_library};
#[cfg(target_os = "linux")]
use super::host::{close_jack_client, open_jack_client};
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::audio::core as audio_core;
use crate::runtime::BindingCallContext;

#[cfg(target_os = "linux")]
use std::ffi::c_int;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "linux")]
use std::time::Duration;

/// One shared JACK monitor state.
#[cfg(target_os = "linux")]
#[derive(Debug)]
struct JackDeviceMonitor {
    /// Signal used to stop the monitor thread.
    stop: Arc<AtomicBool>,
    /// Pending callback signal shared with JACK callback threads.
    pending: Arc<AtomicBool>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
    /// Active reference count for subscriptions using this monitor.
    reference_count: usize,
}

/// Runtime-owned JACK monitor slot.
#[cfg(target_os = "linux")]
#[derive(Debug, Default)]
pub(crate) struct JackMonitorRuntimeState {
    /// Runtime-owned monitor slot.
    monitor: Mutex<Option<JackDeviceMonitor>>,
    /// Whether teardown finalizer was registered.
    shutdown_registered: AtomicBool,
}

/// Return runtime-owned JACK monitor state.
#[cfg(target_os = "linux")]
fn jack_monitor_runtime_state(context: &BindingCallContext) -> Arc<JackMonitorRuntimeState> {
    let runtime_state = context
        .runtime()
        .platform_state
        .audio
        .jack_monitor_runtime_state(JackMonitorRuntimeState::default);
    register_runtime_finalizer(context, &runtime_state);

    runtime_state
}

/// Register one runtime finalizer for JACK monitor teardown.
#[cfg(target_os = "linux")]
fn register_runtime_finalizer(
    context: &BindingCallContext,
    runtime_state: &Arc<JackMonitorRuntimeState>,
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

/// Return whether JACK native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        return super::core::is_backend_supported();
    }

    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Start one JACK native device-event monitor.
pub(crate) fn start_native_device_event_monitor(
    _context: &BindingCallContext,
) -> RuntimeResult<()> {
    #[cfg(target_os = "linux")]
    {
        let monitor_runtime_state = jack_monitor_runtime_state(_context);
        let mut monitor_slot = monitor_runtime_state
            .monitor
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(monitor) = monitor_slot.as_mut() {
            monitor.reference_count += 1;
            return Ok(());
        }

        let stop = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(AtomicBool::new(false));
        let runtime_state = audio_core::audio_event_runtime_state(_context);
        let stop_signal = Arc::clone(&stop);
        let pending_signal = Arc::clone(&pending);
        let callback_runtime_state = Arc::clone(&runtime_state);
        let poll_interval_ns = audio_core::resolved_event_monitor_poll_interval_ns(25_000_000);
        let sleep_interval = Duration::from_nanos(poll_interval_ns);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            run_device_monitor_thread(
                callback_runtime_state,
                stop_signal,
                pending_signal,
                ready_sender,
                sleep_interval,
            )
        });

        let ready_result = ready_receiver.recv().map_err(|_| {
            jack_error(
                "destack.audio.event.open",
                "JACK native event monitor exited before startup completed",
            )
        })?;
        if let Err(error) = ready_result {
            stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            return Err(error);
        }

        *monitor_slot = Some(JackDeviceMonitor {
            stop,
            pending,
            handle,
            reference_count: 1,
        });

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported("destack.audio.event.open", "jack"))
    }
}

/// Stop one JACK native device-event monitor.
pub(crate) fn stop_native_device_event_monitor(_context: &BindingCallContext) {
    #[cfg(target_os = "linux")]
    {
        let runtime_state = jack_monitor_runtime_state(_context);
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
}

/// Run one JACK device-event monitor thread.
#[cfg(target_os = "linux")]
fn run_device_monitor_thread(
    runtime_state: Arc<audio_core::AudioEventRuntimeState>,
    stop: Arc<AtomicBool>,
    pending: Arc<AtomicBool>,
    ready_sender: SyncSender<RuntimeResult<()>>,
    sleep_interval: Duration,
) {
    let library = match require_jack_library("destack.audio.event.open") {
        Ok(library) => library,
        Err(error) => {
            let _ = ready_sender.send(Err(error));
            return;
        }
    };

    let client_name = format!("destack-monitor-{}", std::process::id());
    let client = match open_jack_client(library, "destack.audio.event.open", &client_name) {
        Ok(client) => client,
        Err(error) => {
            let _ = ready_sender.send(Err(error));
            return;
        }
    };

    if let Err(error) = install_monitor_callbacks(library, client, &pending) {
        close_jack_client(library, client);
        let _ = ready_sender.send(Err(error));
        return;
    }

    let activate_status = unsafe { (library.api.jack_activate)(client) };
    if !jack_succeeded(activate_status) {
        close_jack_client(library, client);
        let _ = ready_sender.send(Err(jack_error(
            "destack.audio.event.open",
            format!("failed to activate JACK monitor client (status {activate_status})"),
        )));
        return;
    }

    let _ = ready_sender.send(Ok(()));
    pending.store(true, Ordering::Release);

    while !stop.load(Ordering::Relaxed) {
        if pending.swap(false, Ordering::AcqRel) {
            audio_core::publish_device_snapshot_native(
                &runtime_state,
                audio_core::AudioBackend::Jack,
            );
        }

        thread::sleep(sleep_interval);
    }

    unsafe {
        let _ = (library.api.jack_deactivate)(client);
    }
    close_jack_client(library, client);
}

/// Install callback hooks for JACK monitor-side graph notifications.
#[cfg(target_os = "linux")]
fn install_monitor_callbacks(
    library: &super::core::JackLibrary,
    client: *mut JackClient,
    pending: &Arc<AtomicBool>,
) -> RuntimeResult<()> {
    let callback_argument = Arc::as_ptr(pending) as *const AtomicBool as *mut std::ffi::c_void;

    let registration_status = unsafe {
        (library.api.jack_set_port_registration_callback)(
            client,
            Some(port_registration_callback),
            callback_argument,
        )
    };
    if !jack_succeeded(registration_status) {
        return Err(jack_error(
            "destack.audio.event.open",
            format!(
                "failed to install JACK port-registration callback (status {registration_status})",
            ),
        ));
    }

    let connect_status = unsafe {
        (library.api.jack_set_port_connect_callback)(
            client,
            Some(port_connect_callback),
            callback_argument,
        )
    };
    if !jack_succeeded(connect_status) {
        return Err(jack_error(
            "destack.audio.event.open",
            format!("failed to install JACK port-connect callback (status {connect_status})"),
        ));
    }

    unsafe {
        (library.api.jack_on_shutdown)(client, Some(shutdown_callback), callback_argument);
    }

    Ok(())
}

/// Handle one JACK port-registration callback.
#[cfg(target_os = "linux")]
unsafe extern "C" fn port_registration_callback(
    _port_id: u32,
    _is_registered: c_int,
    argument: *mut std::ffi::c_void,
) {
    let pending = argument.cast::<AtomicBool>();
    if pending.is_null() {
        return;
    }

    unsafe {
        (*pending).store(true, Ordering::Release);
    }
}

/// Handle one JACK port-connect callback.
#[cfg(target_os = "linux")]
unsafe extern "C" fn port_connect_callback(
    _source_port_id: u32,
    _destination_port_id: u32,
    _is_connected: c_int,
    argument: *mut std::ffi::c_void,
) {
    let pending = argument.cast::<AtomicBool>();
    if pending.is_null() {
        return;
    }

    unsafe {
        (*pending).store(true, Ordering::Release);
    }
}

/// Handle one JACK shutdown callback.
#[cfg(target_os = "linux")]
unsafe extern "C" fn shutdown_callback(argument: *mut std::ffi::c_void) {
    let pending = argument.cast::<AtomicBool>();
    if pending.is_null() {
        return;
    }

    unsafe {
        (*pending).store(true, Ordering::Release);
    }
}
