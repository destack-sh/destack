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

#[cfg(target_os = "linux")]
use std::ffi::c_int;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(target_os = "linux")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "linux")]
use std::time::Duration;

/// One shared JACK monitor state.
#[cfg(target_os = "linux")]
struct JackDeviceMonitor {
    /// Signal used to stop the monitor thread.
    stop: Arc<AtomicBool>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
    /// Active reference count for subscriptions using this monitor.
    reference_count: usize,
}

/// One shared JACK monitor slot.
#[cfg(target_os = "linux")]
static JACK_DEVICE_MONITOR_SLOT: OnceLock<Mutex<Option<JackDeviceMonitor>>> = OnceLock::new();
/// One shared pending-change flag set by JACK callbacks.
#[cfg(target_os = "linux")]
static JACK_DEVICE_EVENT_PENDING: AtomicBool = AtomicBool::new(false);

/// Return one shared JACK monitor slot.
#[cfg(target_os = "linux")]
fn monitor_slot() -> &'static Mutex<Option<JackDeviceMonitor>> {
    JACK_DEVICE_MONITOR_SLOT.get_or_init(|| Mutex::new(None))
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
        let handle = thread::spawn(move || run_device_monitor_thread(stop_signal, ready_sender));

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

/// Run one JACK device-event monitor thread.
#[cfg(target_os = "linux")]
fn run_device_monitor_thread(stop: Arc<AtomicBool>, ready_sender: SyncSender<RuntimeResult<()>>) {
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

    if let Err(error) = install_monitor_callbacks(library, client) {
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
    JACK_DEVICE_EVENT_PENDING.store(true, Ordering::Release);

    while !stop.load(Ordering::Relaxed) {
        if JACK_DEVICE_EVENT_PENDING.swap(false, Ordering::AcqRel) {
            audio_core::publish_device_snapshot_native(audio_core::AudioBackend::Jack);
        }

        thread::sleep(Duration::from_millis(25));
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
) -> RuntimeResult<()> {
    let registration_status = unsafe {
        (library.api.jack_set_port_registration_callback)(
            client,
            Some(port_registration_callback),
            std::ptr::null_mut(),
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
            std::ptr::null_mut(),
        )
    };
    if !jack_succeeded(connect_status) {
        return Err(jack_error(
            "destack.audio.event.open",
            format!("failed to install JACK port-connect callback (status {connect_status})"),
        ));
    }

    unsafe {
        (library.api.jack_on_shutdown)(client, Some(shutdown_callback), std::ptr::null_mut());
    }

    Ok(())
}

/// Handle one JACK port-registration callback.
#[cfg(target_os = "linux")]
unsafe extern "C" fn port_registration_callback(
    _port_id: u32,
    _is_registered: c_int,
    _argument: *mut std::ffi::c_void,
) {
    JACK_DEVICE_EVENT_PENDING.store(true, Ordering::Release);
}

/// Handle one JACK port-connect callback.
#[cfg(target_os = "linux")]
unsafe extern "C" fn port_connect_callback(
    _source_port_id: u32,
    _destination_port_id: u32,
    _is_connected: c_int,
    _argument: *mut std::ffi::c_void,
) {
    JACK_DEVICE_EVENT_PENDING.store(true, Ordering::Release);
}

/// Handle one JACK shutdown callback.
#[cfg(target_os = "linux")]
unsafe extern "C" fn shutdown_callback(_argument: *mut std::ffi::c_void) {
    JACK_DEVICE_EVENT_PENDING.store(true, Ordering::Release);
}
