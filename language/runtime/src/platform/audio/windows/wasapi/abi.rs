use std::ffi::c_void;
use std::ptr;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Media::Audio::{
    DEVICE_STATE_ACTIVE, EDataFlow, ERole, IAudioCaptureClient, IAudioClient, IAudioRenderClient,
    IMMDevice, IMMDeviceCollection, IMMDeviceEnumerator, WAVEFORMATEX,
};
use windows_sys::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_sys::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY};
use windows_sys::core::{GUID, HRESULT, PCWSTR, PWSTR};

pub(super) unsafe fn release_com_pointer(pointer: *mut c_void) {
    if pointer.is_null() {
        return;
    }

    unsafe {
        let vtable = *(pointer as *mut *mut IUnknownVTable);
        ((*vtable).release)(pointer);
    }
}

pub(super) unsafe fn imm_device_enumerator_enum_audio_endpoints(
    enumerator: IMMDeviceEnumerator,
    flow: EDataFlow,
    out_collection: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(enumerator as *mut *mut IMMDeviceEnumeratorVTable);
        ((*vtable).enum_audio_endpoints)(enumerator, flow, DEVICE_STATE_ACTIVE, out_collection)
    }
}

pub(super) unsafe fn imm_device_enumerator_get_default_audio_endpoint(
    enumerator: IMMDeviceEnumerator,
    flow: EDataFlow,
    role: ERole,
    out_endpoint: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(enumerator as *mut *mut IMMDeviceEnumeratorVTable);
        ((*vtable).get_default_audio_endpoint)(enumerator, flow, role, out_endpoint)
    }
}

pub(super) unsafe fn imm_device_enumerator_get_device(
    enumerator: IMMDeviceEnumerator,
    endpoint_id: PCWSTR,
    out_endpoint: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(enumerator as *mut *mut IMMDeviceEnumeratorVTable);
        ((*vtable).get_device)(enumerator, endpoint_id, out_endpoint)
    }
}

pub(super) unsafe fn imm_device_enumerator_register_endpoint_notification_callback(
    enumerator: IMMDeviceEnumerator,
    callback: *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(enumerator as *mut *mut IMMDeviceEnumeratorVTable);
        ((*vtable).register_endpoint_notification_callback)(enumerator, callback)
    }
}

pub(super) unsafe fn imm_device_enumerator_unregister_endpoint_notification_callback(
    enumerator: IMMDeviceEnumerator,
    callback: *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(enumerator as *mut *mut IMMDeviceEnumeratorVTable);
        ((*vtable).unregister_endpoint_notification_callback)(enumerator, callback)
    }
}

pub(super) unsafe fn imm_device_collection_get_count(
    collection: IMMDeviceCollection,
    out_count: *mut u32,
) -> HRESULT {
    unsafe {
        let vtable = *(collection as *mut *mut IMMDeviceCollectionVTable);
        ((*vtable).get_count)(collection, out_count)
    }
}

pub(super) unsafe fn imm_device_collection_item(
    collection: IMMDeviceCollection,
    index: u32,
    out_endpoint: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(collection as *mut *mut IMMDeviceCollectionVTable);
        ((*vtable).item)(collection, index, out_endpoint)
    }
}

pub(super) unsafe fn imm_device_get_id(device: IMMDevice, out_id: *mut PWSTR) -> HRESULT {
    unsafe {
        let vtable = *(device as *mut *mut IMMDeviceVTable);
        ((*vtable).get_id)(device, out_id)
    }
}

pub(super) unsafe fn imm_device_activate(
    device: IMMDevice,
    iid: *const GUID,
    class_context: u32,
    activation_params: *const c_void,
    out_interface: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(device as *mut *mut IMMDeviceVTable);
        ((*vtable).activate)(device, iid, class_context, activation_params, out_interface)
    }
}

pub(super) unsafe fn imm_device_open_property_store(
    device: IMMDevice,
    mode: u32,
    out_property_store: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(device as *mut *mut IMMDeviceVTable);
        ((*vtable).open_property_store)(device, mode, out_property_store)
    }
}

pub(super) unsafe fn audio_client_initialize(
    audio_client: IAudioClient,
    share_mode: i32,
    stream_flags: u32,
    buffer_duration_hns: i64,
    periodicity_hns: i64,
    format: *const WAVEFORMATEX,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).initialize)(
            audio_client,
            share_mode,
            stream_flags,
            buffer_duration_hns,
            periodicity_hns,
            format,
            ptr::null(),
        )
    }
}

pub(super) unsafe fn audio_client_is_format_supported(
    audio_client: IAudioClient,
    share_mode: i32,
    format: *const WAVEFORMATEX,
    out_closest_match: *mut *mut WAVEFORMATEX,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).is_format_supported)(audio_client, share_mode, format, out_closest_match)
    }
}

pub(super) unsafe fn audio_client_get_mix_format(
    audio_client: IAudioClient,
    out_mix_format: *mut *mut WAVEFORMATEX,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).get_mix_format)(audio_client, out_mix_format)
    }
}

pub(super) unsafe fn audio_client_get_device_period(
    audio_client: IAudioClient,
    out_default_period_hns: *mut i64,
    out_minimum_period_hns: *mut i64,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).get_device_period)(audio_client, out_default_period_hns, out_minimum_period_hns)
    }
}

pub(super) unsafe fn audio_client_get_buffer_size(
    audio_client: IAudioClient,
    out_frames: *mut u32,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).get_buffer_size)(audio_client, out_frames)
    }
}

pub(super) unsafe fn audio_client_get_current_padding(
    audio_client: IAudioClient,
    out_padding: *mut u32,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).get_current_padding)(audio_client, out_padding)
    }
}

pub(super) unsafe fn audio_client_get_service(
    audio_client: IAudioClient,
    interface_id: *const GUID,
    out_interface: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).get_service)(audio_client, interface_id, out_interface)
    }
}

pub(super) unsafe fn audio_client_start(audio_client: IAudioClient) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).start)(audio_client)
    }
}

pub(super) unsafe fn audio_client_stop(audio_client: IAudioClient) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).stop)(audio_client)
    }
}

pub(super) unsafe fn audio_client_reset(audio_client: IAudioClient) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).reset)(audio_client)
    }
}

pub(super) unsafe fn audio_client_set_event_handle(
    audio_client: IAudioClient,
    event_handle: HANDLE,
) -> HRESULT {
    unsafe {
        let vtable = *(audio_client as *mut *mut IAudioClientVTable);
        ((*vtable).set_event_handle)(audio_client, event_handle)
    }
}

pub(super) unsafe fn audio_render_client_get_buffer(
    render_client: IAudioRenderClient,
    requested_frames: u32,
    out_data: *mut *mut u8,
) -> HRESULT {
    unsafe {
        let vtable = *(render_client as *mut *mut IAudioRenderClientVTable);
        ((*vtable).get_buffer)(render_client, requested_frames, out_data)
    }
}

pub(super) unsafe fn audio_render_client_release_buffer(
    render_client: IAudioRenderClient,
    written_frames: u32,
    flags: u32,
) -> HRESULT {
    unsafe {
        let vtable = *(render_client as *mut *mut IAudioRenderClientVTable);
        ((*vtable).release_buffer)(render_client, written_frames, flags)
    }
}

pub(super) unsafe fn audio_capture_client_get_next_packet_size(
    capture_client: IAudioCaptureClient,
    out_frames: *mut u32,
) -> HRESULT {
    unsafe {
        let vtable = *(capture_client as *mut *mut IAudioCaptureClientVTable);
        ((*vtable).get_next_packet_size)(capture_client, out_frames)
    }
}

pub(super) unsafe fn audio_capture_client_get_buffer(
    capture_client: IAudioCaptureClient,
    out_data: *mut *mut u8,
    out_frames: *mut u32,
    out_flags: *mut u32,
    out_device_position: *mut u64,
    out_qpc_position: *mut u64,
) -> HRESULT {
    unsafe {
        let vtable = *(capture_client as *mut *mut IAudioCaptureClientVTable);
        ((*vtable).get_buffer)(
            capture_client,
            out_data,
            out_frames,
            out_flags,
            out_device_position,
            out_qpc_position,
        )
    }
}

pub(super) unsafe fn audio_capture_client_release_buffer(
    capture_client: IAudioCaptureClient,
    read_frames: u32,
) -> HRESULT {
    unsafe {
        let vtable = *(capture_client as *mut *mut IAudioCaptureClientVTable);
        ((*vtable).release_buffer)(capture_client, read_frames)
    }
}

pub(super) unsafe fn property_store_get_value(
    property_store: IPropertyStore,
    property_key: *const PROPERTYKEY,
    out_value: *mut PROPVARIANT,
) -> HRESULT {
    unsafe {
        let vtable = *(property_store as *mut *mut IPropertyStoreVTable);
        ((*vtable).get_value)(property_store, property_key, out_value)
    }
}

#[repr(C)]
pub(super) struct IUnknownVTable {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
pub(super) struct IMMDeviceEnumeratorVTable {
    base: IUnknownVTable,
    enum_audio_endpoints:
        unsafe extern "system" fn(IMMDeviceEnumerator, EDataFlow, u32, *mut *mut c_void) -> HRESULT,
    get_default_audio_endpoint: unsafe extern "system" fn(
        IMMDeviceEnumerator,
        EDataFlow,
        ERole,
        *mut *mut c_void,
    ) -> HRESULT,
    get_device: unsafe extern "system" fn(IMMDeviceEnumerator, PCWSTR, *mut *mut c_void) -> HRESULT,
    register_endpoint_notification_callback:
        unsafe extern "system" fn(IMMDeviceEnumerator, *mut c_void) -> HRESULT,
    unregister_endpoint_notification_callback:
        unsafe extern "system" fn(IMMDeviceEnumerator, *mut c_void) -> HRESULT,
}

#[repr(C)]
pub(super) struct IMMDeviceCollectionVTable {
    base: IUnknownVTable,
    get_count: unsafe extern "system" fn(IMMDeviceCollection, *mut u32) -> HRESULT,
    item: unsafe extern "system" fn(IMMDeviceCollection, u32, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
pub(super) struct IMMDeviceVTable {
    base: IUnknownVTable,
    activate: unsafe extern "system" fn(
        IMMDevice,
        *const GUID,
        u32,
        *const c_void,
        *mut *mut c_void,
    ) -> HRESULT,
    open_property_store: unsafe extern "system" fn(IMMDevice, u32, *mut *mut c_void) -> HRESULT,
    get_id: unsafe extern "system" fn(IMMDevice, *mut PWSTR) -> HRESULT,
    get_state: unsafe extern "system" fn(IMMDevice, *mut u32) -> HRESULT,
}

#[repr(C)]
pub(super) struct IAudioClientVTable {
    base: IUnknownVTable,
    initialize: unsafe extern "system" fn(
        IAudioClient,
        i32,
        u32,
        i64,
        i64,
        *const WAVEFORMATEX,
        *const GUID,
    ) -> HRESULT,
    get_buffer_size: unsafe extern "system" fn(IAudioClient, *mut u32) -> HRESULT,
    get_stream_latency: unsafe extern "system" fn(IAudioClient, *mut i64) -> HRESULT,
    get_current_padding: unsafe extern "system" fn(IAudioClient, *mut u32) -> HRESULT,
    is_format_supported: unsafe extern "system" fn(
        IAudioClient,
        i32,
        *const WAVEFORMATEX,
        *mut *mut WAVEFORMATEX,
    ) -> HRESULT,
    get_mix_format: unsafe extern "system" fn(IAudioClient, *mut *mut WAVEFORMATEX) -> HRESULT,
    get_device_period: unsafe extern "system" fn(IAudioClient, *mut i64, *mut i64) -> HRESULT,
    start: unsafe extern "system" fn(IAudioClient) -> HRESULT,
    stop: unsafe extern "system" fn(IAudioClient) -> HRESULT,
    reset: unsafe extern "system" fn(IAudioClient) -> HRESULT,
    set_event_handle: unsafe extern "system" fn(IAudioClient, isize) -> HRESULT,
    get_service: unsafe extern "system" fn(IAudioClient, *const GUID, *mut *mut c_void) -> HRESULT,
}

#[repr(C)]
pub(super) struct IAudioRenderClientVTable {
    base: IUnknownVTable,
    get_buffer: unsafe extern "system" fn(IAudioRenderClient, u32, *mut *mut u8) -> HRESULT,
    release_buffer: unsafe extern "system" fn(IAudioRenderClient, u32, u32) -> HRESULT,
}

#[repr(C)]
pub(super) struct IAudioCaptureClientVTable {
    base: IUnknownVTable,
    get_buffer: unsafe extern "system" fn(
        IAudioCaptureClient,
        *mut *mut u8,
        *mut u32,
        *mut u32,
        *mut u64,
        *mut u64,
    ) -> HRESULT,
    release_buffer: unsafe extern "system" fn(IAudioCaptureClient, u32) -> HRESULT,
    get_next_packet_size: unsafe extern "system" fn(IAudioCaptureClient, *mut u32) -> HRESULT,
}

#[repr(C)]
pub(super) struct IPropertyStoreVTable {
    base: IUnknownVTable,
    get_count: unsafe extern "system" fn(IPropertyStore, *mut u32) -> HRESULT,
    get_at: unsafe extern "system" fn(IPropertyStore, u32, *mut PROPERTYKEY) -> HRESULT,
    get_value:
        unsafe extern "system" fn(IPropertyStore, *const PROPERTYKEY, *mut PROPVARIANT) -> HRESULT,
    set_value: unsafe extern "system" fn(
        IPropertyStore,
        *const PROPERTYKEY,
        *const PROPVARIANT,
    ) -> HRESULT,
    commit: unsafe extern "system" fn(IPropertyStore) -> HRESULT,
}
