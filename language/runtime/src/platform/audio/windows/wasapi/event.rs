use std::ffi::c_void;
use std::sync::atomic::AtomicU32;

use super::abi::{
    imm_device_enumerator_register_endpoint_notification_callback,
    imm_device_enumerator_unregister_endpoint_notification_callback,
};
use super::constants::{HRESULT_OK, IID_IMM_NOTIFICATION_CLIENT};
use super::core::{
    ComApartment, ComPointer, create_device_enumerator, failed, hresult_error, initialize_com,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;
use crate::platform::audio::core::monitor::AudioMonitorHandle;
use crate::platform::core as core_platform;
use crate::runtime::service::executor::thread::ServiceThreadExecutor;
use crate::runtime::{ExecutionAffinity, ExecutionMode, ExecutionPolicy, spawn_service_thread};

use crate::platform::audio as audio_types;
use windows_sys::Win32::Media::Audio::{EDataFlow, ERole, IMMDeviceEnumerator};
use windows_sys::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
use windows_sys::core::{GUID, HRESULT, PCWSTR};

/// One persistent native monitor state for WASAPI endpoint notifications.
struct WasapiMonitorState {
    /// COM apartment lifetime for this monitor thread.
    _apartment: ComApartment,
    /// Owned device enumerator for callback registration.
    enumerator: ComPointer,
    /// Registered COM notification callback object.
    callback_pointer: *mut c_void,
}

impl Drop for WasapiMonitorState {
    /// Unregister the endpoint notification callback before teardown.
    fn drop(&mut self) {
        let enumerator_raw = self.enumerator.raw() as IMMDeviceEnumerator;

        let _ = unsafe {
            imm_device_enumerator_unregister_endpoint_notification_callback(
                enumerator_raw,
                self.callback_pointer,
            )
        };

        release_notification_client(self.callback_pointer);
    }
}

/// One native WASAPI monitor handle backed by a dedicated service thread.
struct WasapiDeviceMonitor {
    /// Dedicated executor that owns the monitor apartment and callback registration.
    _executor: ServiceThreadExecutor<WasapiMonitorState>,
}

impl AudioMonitorHandle for WasapiDeviceMonitor {
    fn stop(self: Box<Self>) {}
}

/// Start one WASAPI native device-event monitor.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    let executor = spawn_service_thread(
        "destack-audio-wasapi-monitor",
        ExecutionPolicy::process(ExecutionMode::Thread)
            .with_affinity(ExecutionAffinity::WindowsMta),
        build_monitor_state,
    )?;

    Ok(Box::new(WasapiDeviceMonitor {
        _executor: executor,
    }))
}

/// Build one live WASAPI monitor state on the dedicated service thread.
fn build_monitor_state() -> RuntimeResult<WasapiMonitorState> {
    let apartment = initialize_com()?;
    let enumerator = create_device_enumerator()?;
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

        return Err(hresult_error(
            "destack.audio.event.open",
            register_status,
            "failed to register WASAPI endpoint notification callback",
        ));
    }

    Ok(WasapiMonitorState {
        _apartment: apartment,
        enumerator,
        callback_pointer,
    })
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
        audio_core::publish_device_snapshot_native_if_service_live(
            audio_types::AudioBackend::Wasapi,
        );
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
