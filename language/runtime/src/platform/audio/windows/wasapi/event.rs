use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::abi::{
    imm_device_enumerator_register_endpoint_notification_callback,
    imm_device_enumerator_unregister_endpoint_notification_callback,
};
use super::constants::{HRESULT_OK, IID_IMM_NOTIFICATION_CLIENT};
use super::host::{create_device_enumerator, failed, hresult_error, initialize_com};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};

use crate::platform::audio as audio_types;
use windows_sys::Win32::Media::Audio::{EDataFlow, ERole, IMMDeviceEnumerator};
use windows_sys::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
use windows_sys::core::{GUID, HRESULT, PCWSTR};

/// One persistent native monitor state for WASAPI endpoint notifications.
#[derive(Debug)]
struct WasapiDeviceMonitor {
    /// Signal used to stop the monitor thread.
    stop: Arc<AtomicBool>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
}

impl AudioMonitorHandle for WasapiDeviceMonitor {
    fn stop(self: Box<Self>) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.handle.join();
    }
}

/// Return one startup error payload for WASAPI native monitor initialization.
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

/// Start one WASAPI native device-event monitor.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_signal = Arc::clone(&stop);
    let poll_interval_ns = audio_core::resolved_event_monitor_poll_interval_ns(50_000_000);
    let sleep_interval = Duration::from_nanos(poll_interval_ns);
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let handle =
        thread::spawn(move || run_device_monitor_thread(stop_signal, ready_sender, sleep_interval));

    let ready_result = ready_receiver.recv().map_err(|_| {
        startup_error("WASAPI native event monitor exited before startup completed")
    })?;
    if let Err(error) = ready_result {
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        return Err(error);
    }

    Ok(Box::new(WasapiDeviceMonitor { stop, handle }))
}

/// Run one WASAPI native device-event monitor thread.
fn run_device_monitor_thread(
    stop: Arc<AtomicBool>,
    ready_sender: SyncSender<RuntimeResult<()>>,
    sleep_interval: Duration,
) {
    let apartment = match initialize_com() {
        Ok(apartment) => apartment,
        Err(error) => {
            let _ = ready_sender.send(Err(error));
            return;
        }
    };

    let enumerator = match create_device_enumerator() {
        Ok(enumerator) => enumerator,
        Err(error) => {
            let _ = ready_sender.send(Err(error));
            drop(apartment);
            return;
        }
    };
    let enumerator_raw = enumerator.raw() as IMMDeviceEnumerator;

    let callback_pointer = create_notification_client();
    let register_status = unsafe {
        imm_device_enumerator_register_endpoint_notification_callback(
            enumerator_raw,
            callback_pointer,
        )
    };
    if failed(register_status) {
        release_notification_client(callback_pointer);
        let _ = ready_sender.send(Err(hresult_error(
            "destack.audio.event.open",
            register_status,
            "failed to register WASAPI endpoint notification callback",
        )));
        drop(enumerator);
        drop(apartment);
        return;
    }

    let _ = ready_sender.send(Ok(()));

    while !stop.load(Ordering::Relaxed) {
        thread::sleep(sleep_interval);
    }

    let _ = unsafe {
        imm_device_enumerator_unregister_endpoint_notification_callback(
            enumerator_raw,
            callback_pointer,
        )
    };
    release_notification_client(callback_pointer);
    drop(enumerator);
    drop(apartment);
}

/// One COM notification callback object for WASAPI endpoint changes.
#[repr(C)]
struct WasapiNotificationClient {
    /// Vtable pointer for COM callback dispatch.
    vtable: *const WasapiNotificationClientVTable,
    /// Manual COM reference count.
    reference_count: AtomicU32,
}

/// Vtable layout for IMMNotificationClient callback object.
#[repr(C)]
struct WasapiNotificationClientVTable {
    /// QueryInterface method.
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    /// AddRef method.
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Release method.
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    /// OnDeviceStateChanged callback.
    on_device_state_changed: unsafe extern "system" fn(*mut c_void, PCWSTR, u32) -> HRESULT,
    /// OnDeviceAdded callback.
    on_device_added: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
    /// OnDeviceRemoved callback.
    on_device_removed: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
    /// OnDefaultDeviceChanged callback.
    on_default_device_changed:
        unsafe extern "system" fn(*mut c_void, EDataFlow, ERole, PCWSTR) -> HRESULT,
    /// OnPropertyValueChanged callback.
    on_property_value_changed:
        unsafe extern "system" fn(*mut c_void, PCWSTR, *const PROPERTYKEY) -> HRESULT,
}

// shared vtable for WASAPI endpoint notification callbacks
core_platform::define_com_callback_vtable!(
    static = WASAPI_NOTIFICATION_CLIENT_VTABLE,
    type = WasapiNotificationClientVTable,
    value = WasapiNotificationClientVTable,
    query_interface = query_interface,
    add_ref = add_ref,
    release = release,
    methods = {
        on_device_state_changed = on_device_state_changed,
        on_device_added = on_device_added,
        on_device_removed = on_device_removed,
        on_default_device_changed = on_default_device_changed,
        on_property_value_changed = on_property_value_changed
    }
);

/// Create one COM callback object for WASAPI endpoint notifications.
fn create_notification_client() -> *mut c_void {
    let callback = Box::new(WasapiNotificationClient {
        vtable: &WASAPI_NOTIFICATION_CLIENT_VTABLE,
        reference_count: AtomicU32::new(1),
    });

    Box::into_raw(callback) as *mut c_void
}

/// Release one COM callback object pointer.
fn release_notification_client(callback_pointer: *mut c_void) {
    unsafe {
        core_platform::com_release_with(callback_pointer, release);
    }
}

core_platform::define_com_iunknown_methods!(
    object = WasapiNotificationClient,
    from_raw = callback_from_raw,
    query_interface = query_interface,
    add_ref = add_ref,
    release = release,
    interfaces = [IID_IMM_NOTIFICATION_CLIENT]
);

/// Publish one snapshot refresh for one WASAPI device callback.
fn publish_notification_snapshot(this: *mut c_void) {
    let callback = unsafe { callback_from_raw(this) };
    core_platform::callback_boundary(|| {
        let _ = unsafe { callback.as_ref() };
        audio_core::publish_device_snapshot_native(audio_types::AudioBackend::Wasapi);
    });
}

/// Handle one WASAPI endpoint state-change callback.
unsafe extern "system" fn on_device_state_changed(
    this: *mut c_void,
    _device_id: PCWSTR,
    _new_state: u32,
) -> HRESULT {
    publish_notification_snapshot(this);
    HRESULT_OK
}

/// Handle one WASAPI endpoint-added callback.
unsafe extern "system" fn on_device_added(this: *mut c_void, _device_id: PCWSTR) -> HRESULT {
    publish_notification_snapshot(this);
    HRESULT_OK
}

/// Handle one WASAPI endpoint-removed callback.
unsafe extern "system" fn on_device_removed(this: *mut c_void, _device_id: PCWSTR) -> HRESULT {
    publish_notification_snapshot(this);
    HRESULT_OK
}

/// Handle one WASAPI default-endpoint change callback.
unsafe extern "system" fn on_default_device_changed(
    this: *mut c_void,
    _flow: EDataFlow,
    _role: ERole,
    _default_device_id: PCWSTR,
) -> HRESULT {
    publish_notification_snapshot(this);
    HRESULT_OK
}

/// Handle one WASAPI property-value change callback.
unsafe extern "system" fn on_property_value_changed(
    this: *mut c_void,
    _device_id: PCWSTR,
    _property_key: *const PROPERTYKEY,
) -> HRESULT {
    publish_notification_snapshot(this);
    HRESULT_OK
}
