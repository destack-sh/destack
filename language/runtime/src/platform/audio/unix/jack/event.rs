#[cfg(target_os = "linux")]
use super::abi::JackClient;
#[cfg(target_os = "linux")]
use super::core::{JackLibrary, jack_error, jack_succeeded, require_jack_library};
#[cfg(target_os = "linux")]
use super::core::{close_jack_client, open_jack_client};
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "linux")]
use crate::platform::audio as audio_types;
#[cfg(not(target_os = "linux"))]
use crate::platform::audio::backend::backend_not_supported;
#[cfg(target_os = "linux")]
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::monitor::AudioMonitorHandle;
#[cfg(target_os = "linux")]
use crate::runtime::{ExecutionMode, ExecutionPolicy, start_with_policy};
#[cfg(target_os = "linux")]
#[cfg(target_os = "linux")]
use std::ffi::c_int;
#[cfg(target_os = "linux")]
use std::sync::mpsc::{self, SyncSender};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Condvar, Mutex};
#[cfg(target_os = "linux")]
use std::thread::{self, JoinHandle};

/// One JACK monitor wake state.
#[cfg(target_os = "linux")]
#[derive(Debug, Default)]
struct JackMonitorSignalState {
    /// Whether one refresh pass is pending.
    is_pending: bool,
    /// Whether the monitor should stop.
    is_stopped: bool,
}

/// One JACK monitor wake handle shared with callbacks.
#[cfg(target_os = "linux")]
#[derive(Debug, Default)]
struct JackMonitorSignal {
    /// Shared wake state.
    state: Mutex<JackMonitorSignalState>,
    /// Wake channel for callback driven refresh.
    wake: Condvar,
}

#[cfg(target_os = "linux")]
impl JackMonitorSignal {
    /// Request one refresh pass.
    fn request_refresh(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.is_pending = true;
        self.wake.notify_one();
    }

    /// Request one monitor shutdown.
    fn request_stop(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.is_stopped = true;
        self.wake.notify_one();
    }
}

/// One shared JACK monitor state.
#[cfg(target_os = "linux")]
#[derive(Debug)]
struct JackDeviceMonitor {
    /// Wake handle shared with the monitor thread.
    signal: Arc<JackMonitorSignal>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
}

#[cfg(target_os = "linux")]
impl AudioMonitorHandle for JackDeviceMonitor {
    fn stop(self: Box<Self>) {
        self.signal.request_stop();
        let _ = self.handle.join();
    }
}

/// Start one JACK native device-event monitor.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    #[cfg(target_os = "linux")]
    {
        let signal = Arc::new(JackMonitorSignal::default());
        let thread_signal = Arc::clone(&signal);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let handle = start_with_policy(
            "destack-audio-jack-monitor",
            "destack.audio.event.open",
            ExecutionPolicy::process(ExecutionMode::Loop),
            move || run_device_monitor_thread(thread_signal, ready_sender),
        )?;

        let ready_result = ready_receiver.recv().map_err(|_| {
            jack_error(
                "destack.audio.event.open",
                "JACK native event monitor exited before startup completed",
            )
        })?;
        if let Err(error) = ready_result {
            signal.request_stop();
            let _ = handle.join();
            return Err(error);
        }

        Ok(Box::new(JackDeviceMonitor { signal, handle }))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err(backend_not_supported("destack.audio.event.open", "jack"))
    }
}

/// Run one JACK device-event monitor thread.
#[cfg(target_os = "linux")]
fn run_device_monitor_thread(
    signal: Arc<JackMonitorSignal>,
    ready_sender: SyncSender<RuntimeResult<()>>,
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

    if let Err(error) = install_monitor_callbacks(&library, client, &signal) {
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
    signal.request_refresh();

    loop {
        // wait for the next callback or stop request
        let mut state = signal
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while !state.is_pending && !state.is_stopped {
            state = signal
                .wake
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }

        // stop the monitor once teardown wins the race
        if state.is_stopped {
            break;
        }

        state.is_pending = false;
        drop(state);

        // publish one fresh JACK snapshot on demand
        audio_core::publish_device_snapshot_native_if_service_live(audio_types::AudioBackend::Jack);
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
    signal: &Arc<JackMonitorSignal>,
) -> RuntimeResult<()> {
    let callback_argument = Arc::as_ptr(signal) as *mut std::ffi::c_void;

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
    let signal = argument.cast::<JackMonitorSignal>();
    if signal.is_null() {
        return;
    }

    unsafe {
        (*signal).request_refresh();
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
    let signal = argument.cast::<JackMonitorSignal>();
    if signal.is_null() {
        return;
    }

    unsafe {
        (*signal).request_refresh();
    }
}

/// Handle one JACK shutdown callback.
#[cfg(target_os = "linux")]
unsafe extern "C" fn shutdown_callback(argument: *mut std::ffi::c_void) {
    let signal = argument.cast::<JackMonitorSignal>();
    if signal.is_null() {
        return;
    }

    unsafe {
        (*signal).request_refresh();
    }
}
