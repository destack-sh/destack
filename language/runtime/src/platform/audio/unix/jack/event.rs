#[cfg(target_os = "linux")]
use super::abi::JackClient;
#[cfg(target_os = "linux")]
use super::core::{JackLibrary, jack_error, jack_succeeded, require_jack_library};
#[cfg(target_os = "linux")]
use super::host::{close_jack_client, open_jack_client};
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::audio as audio_types;
#[cfg(not(target_os = "linux"))]
use crate::platform::audio::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::monitor::AudioMonitorHandle;

#[cfg(target_os = "linux")]
use std::ffi::c_int;
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
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
    /// Running monitor thread.
    handle: JoinHandle<()>,
}

#[cfg(target_os = "linux")]
impl AudioMonitorHandle for JackDeviceMonitor {
    fn stop(self: Box<Self>) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.handle.join();
    }
}

/// Start one JACK native device-event monitor.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    #[cfg(target_os = "linux")]
    {
        let stop = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(AtomicBool::new(false));
        let stop_signal = Arc::clone(&stop);
        let pending_signal = Arc::clone(&pending);
        let poll_interval_ns = audio_core::resolved_event_monitor_poll_interval_ns(25_000_000);
        let sleep_interval = Duration::from_nanos(poll_interval_ns);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            run_device_monitor_thread(stop_signal, pending_signal, ready_sender, sleep_interval)
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

        Ok(Box::new(JackDeviceMonitor { stop, handle }))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported("destack.audio.event.open", "jack"))
    }
}

/// Run one JACK device-event monitor thread.
#[cfg(target_os = "linux")]
fn run_device_monitor_thread(
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
    let client = match open_jack_client(&library, "destack.audio.event.open", &client_name) {
        Ok(client) => client,
        Err(error) => {
            let _ = ready_sender.send(Err(error));
            return;
        }
    };

    if let Err(error) = install_monitor_callbacks(&library, client, &pending) {
        close_jack_client(&library, client);
        let _ = ready_sender.send(Err(error));
        return;
    }

    let activate_status = unsafe { (library.api.jack_activate)(client) };
    if !jack_succeeded(activate_status) {
        close_jack_client(&library, client);
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
            audio_core::publish_device_snapshot_native_if_service_live(
                audio_types::AudioBackend::Jack,
            );
        }

        thread::sleep(sleep_interval);
    }

    unsafe {
        let _ = (library.api.jack_deactivate)(client);
    }
    close_jack_client(&library, client);
}

/// Install callback hooks for JACK monitor-side graph notifications.
#[cfg(target_os = "linux")]
fn install_monitor_callbacks(
    library: &JackLibrary,
    client: *mut JackClient,
    pending: &Arc<AtomicBool>,
) -> RuntimeResult<()> {
    let callback_argument = Arc::as_ptr(pending) as *mut std::ffi::c_void;

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
