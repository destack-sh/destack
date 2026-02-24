use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::abi::{
    imm_device_enumerator_register_endpoint_notification_callback,
    imm_device_enumerator_unregister_endpoint_notification_callback,
};
use super::constants::{HRESULT_OK, IID_IMM_NOTIFICATION_CLIENT};
use super::host::{create_device_enumerator, failed, hresult_error, initialize_com};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;

use windows_sys::Win32::Foundation::E_NOINTERFACE;
use windows_sys::Win32::Media::Audio::{EDataFlow, ERole, IMMDeviceEnumerator};
use windows_sys::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
use windows_sys::core::{GUID, HRESULT, PCWSTR};

/// One persistent native monitor state for WASAPI endpoint notifications.
struct WasapiDeviceMonitor {
    /// Signal used to stop the monitor thread.
    stop: Arc<AtomicBool>,
    /// Running monitor thread.
    handle: JoinHandle<()>,
    /// Active reference count for subscriptions using this monitor.
    reference_count: usize,
}

/// One shared WASAPI device monitor slot.
static WASAPI_DEVICE_MONITOR_SLOT: OnceLock<Mutex<Option<WasapiDeviceMonitor>>> = OnceLock::new();

/// Return one shared WASAPI device monitor slot.
fn monitor_slot() -> &'static Mutex<Option<WasapiDeviceMonitor>> {
    WASAPI_DEVICE_MONITOR_SLOT.get_or_init(|| Mutex::new(None))
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

/// Return whether WASAPI native device-event monitoring is available.
pub(crate) fn native_device_events_supported() -> bool {
    true
}

/// Start one WASAPI native device-event monitor.
pub(crate) fn start_native_device_event_monitor() -> RuntimeResult<()> {
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
        startup_error("WASAPI native event monitor exited before startup completed")
    })?;
    if let Err(error) = ready_result {
        stop.store(true, Ordering::Relaxed);
        let _ = handle.join();
        return Err(error);
    }

    *monitor_slot = Some(WasapiDeviceMonitor {
        stop,
        handle,
        reference_count: 1,
    });

    Ok(())
}

/// Stop one WASAPI native device-event monitor.
pub(crate) fn stop_native_device_event_monitor() {
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

/// Run one WASAPI native device-event monitor thread.
fn run_device_monitor_thread(stop: Arc<AtomicBool>, ready_sender: SyncSender<RuntimeResult<()>>) {
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
        thread::sleep(Duration::from_millis(50));
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

/// One COM `IUnknown` interface guid.
const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_c000_000000000046);

/// Shared vtable for WASAPI endpoint notification callbacks.
static WASAPI_NOTIFICATION_CLIENT_VTABLE: WasapiNotificationClientVTable =
    WasapiNotificationClientVTable {
        query_interface: query_interface,
        add_ref: add_ref,
        release: release,
        on_device_state_changed: on_device_state_changed,
        on_device_added: on_device_added,
        on_device_removed: on_device_removed,
        on_default_device_changed: on_default_device_changed,
        on_property_value_changed: on_property_value_changed,
    };

/// Return whether two COM interface identifiers are byte-for-byte equal.
fn guid_equals(left: &GUID, right: &GUID) -> bool {
    left.data1 == right.data1
        && left.data2 == right.data2
        && left.data3 == right.data3
        && left.data4 == right.data4
}

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
    if callback_pointer.is_null() {
        return;
    }

    unsafe {
        let _ = release(callback_pointer);
    }
}

/// Return one callback object pointer cast from one COM instance pointer.
unsafe fn callback_from_raw(this: *mut c_void) -> *mut WasapiNotificationClient {
    this.cast::<WasapiNotificationClient>()
}

/// Query one callback interface from one COM callback object.
unsafe extern "system" fn query_interface(
    this: *mut c_void,
    interface_id: *const GUID,
    out_interface: *mut *mut c_void,
) -> HRESULT {
    if out_interface.is_null() {
        return E_NOINTERFACE;
    }

    unsafe {
        *out_interface = ptr::null_mut();
    }

    if interface_id.is_null() {
        return E_NOINTERFACE;
    }

    let interface_id = unsafe { *interface_id };
    if guid_equals(&interface_id, &IID_IUNKNOWN)
        || guid_equals(&interface_id, &IID_IMM_NOTIFICATION_CLIENT)
    {
        unsafe {
            *out_interface = this;
            let _ = add_ref(this);
        }
        return HRESULT_OK;
    }

    E_NOINTERFACE
}

/// Increment one COM callback reference count.
unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    let callback = unsafe { callback_from_raw(this) };
    unsafe { (*callback).reference_count.fetch_add(1, Ordering::Relaxed) + 1 }
}

/// Decrement one COM callback reference count and free on zero.
unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    let callback = unsafe { callback_from_raw(this) };
    let remaining = unsafe { (*callback).reference_count.fetch_sub(1, Ordering::Release) - 1 };
    if remaining == 0 {
        std::sync::atomic::fence(Ordering::Acquire);
        unsafe {
            let _ = Box::from_raw(callback);
        }
    }

    remaining
}

/// Publish one snapshot refresh for one WASAPI device callback.
fn publish_notification_snapshot() {
    let _ = std::panic::catch_unwind(|| {
        audio_core::publish_device_snapshot_native(audio_core::AudioBackend::Wasapi);
    });
}

/// Handle one WASAPI endpoint state-change callback.
unsafe extern "system" fn on_device_state_changed(
    _this: *mut c_void,
    _device_id: PCWSTR,
    _new_state: u32,
) -> HRESULT {
    publish_notification_snapshot();
    HRESULT_OK
}

/// Handle one WASAPI endpoint-added callback.
unsafe extern "system" fn on_device_added(_this: *mut c_void, _device_id: PCWSTR) -> HRESULT {
    publish_notification_snapshot();
    HRESULT_OK
}

/// Handle one WASAPI endpoint-removed callback.
unsafe extern "system" fn on_device_removed(_this: *mut c_void, _device_id: PCWSTR) -> HRESULT {
    publish_notification_snapshot();
    HRESULT_OK
}

/// Handle one WASAPI default-endpoint change callback.
unsafe extern "system" fn on_default_device_changed(
    _this: *mut c_void,
    _flow: EDataFlow,
    _role: ERole,
    _default_device_id: PCWSTR,
) -> HRESULT {
    publish_notification_snapshot();
    HRESULT_OK
}

/// Handle one WASAPI property-value change callback.
unsafe extern "system" fn on_property_value_changed(
    _this: *mut c_void,
    _device_id: PCWSTR,
    _property_key: *const PROPERTYKEY,
) -> HRESULT {
    publish_notification_snapshot();
    HRESULT_OK
}
