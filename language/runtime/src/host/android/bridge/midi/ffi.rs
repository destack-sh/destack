use crate::host::android::abi::HOST_STATUS_INVALID_ARGUMENT;
use crate::host::android::bridge::bindings::invoke_android_binding_callback;
use crate::host::android::bridge::midi::types::{
    AndroidHostMidiCallbacks, AndroidHostMidiEventHeader, AndroidHostMidiInputRecordHeader,
    AndroidHostMidiOpenedPortHeader, AndroidHostMidiOutputRecordHeader,
    AndroidHostMidiPortDescriptorHeader,
};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Resolve and invoke one Android host MIDI callback.
fn call_android_midi_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostMidiCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.midi), invoke)
}

/// Describe Android MIDI backend support.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_describe_backend(
    runtime_id: u64,
    capability_flags: *mut u64,
    supported_data_formats: *mut u32,
    supported_protocols: *mut u32,
) -> u32 {
    if capability_flags.is_null()
        || supported_data_formats.is_null()
        || supported_protocols.is_null()
    {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.describe_backend,
        |callback| unsafe {
            callback(
                runtime_id,
                capability_flags,
                supported_data_formats,
                supported_protocols,
            )
        },
    )
}

/// List Android MIDI input ports.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_input_port_list(
    runtime_id: u64,
    flags: u32,
    headers: NativeSlice<AndroidHostMidiPortDescriptorHeader>,
    header_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if header_count_written.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.input_port_list,
        |callback| unsafe {
            callback(
                runtime_id,
                flags,
                headers,
                header_count_written,
                string_bytes,
                string_bytes_written,
            )
        },
    )
}

/// List Android MIDI output ports.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_output_port_list(
    runtime_id: u64,
    flags: u32,
    headers: NativeSlice<AndroidHostMidiPortDescriptorHeader>,
    header_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if header_count_written.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.output_port_list,
        |callback| unsafe {
            callback(
                runtime_id,
                flags,
                headers,
                header_count_written,
                string_bytes,
                string_bytes_written,
            )
        },
    )
}

/// Open one Android MIDI input port.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_input_port_open(
    runtime_id: u64,
    id: NativeStringRef,
    data_format: u32,
    protocol: u32,
    queue_capacity: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.input_port_open,
        |callback| unsafe {
            callback(
                runtime_id,
                id,
                data_format,
                protocol,
                queue_capacity,
                opened_port,
                string_bytes,
                string_bytes_written,
            )
        },
    )
}

/// Open one Android MIDI output port.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_output_port_open(
    runtime_id: u64,
    id: NativeStringRef,
    data_format: u32,
    protocol: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.output_port_open,
        |callback| unsafe {
            callback(
                runtime_id,
                id,
                data_format,
                protocol,
                opened_port,
                string_bytes,
                string_bytes_written,
            )
        },
    )
}

/// Create one Android virtual MIDI input port.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_input_virtual_create(
    runtime_id: u64,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: u32,
    protocol: u32,
    queue_capacity: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.input_virtual_create,
        |callback| unsafe {
            callback(
                runtime_id,
                name,
                manufacturer,
                model,
                version,
                data_format,
                protocol,
                queue_capacity,
                opened_port,
                string_bytes,
                string_bytes_written,
            )
        },
    )
}

/// Create one Android virtual MIDI output port.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_output_virtual_create(
    runtime_id: u64,
    name: NativeStringRef,
    manufacturer: NativeStringRef,
    model: NativeStringRef,
    version: NativeStringRef,
    data_format: u32,
    protocol: u32,
    opened_port: *mut AndroidHostMidiOpenedPortHeader,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if opened_port.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.output_virtual_create,
        |callback| unsafe {
            callback(
                runtime_id,
                name,
                manufacturer,
                model,
                version,
                data_format,
                protocol,
                opened_port,
                string_bytes,
                string_bytes_written,
            )
        },
    )
}

/// Close one Android MIDI input session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_input_port_close(
    runtime_id: u64,
    session_id: u64,
) -> u32 {
    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.input_port_close,
        |callback| unsafe { callback(runtime_id, session_id) },
    )
}

/// Close one Android MIDI output session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_output_port_close(
    runtime_id: u64,
    session_id: u64,
) -> u32 {
    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.output_port_close,
        |callback| unsafe { callback(runtime_id, session_id) },
    )
}

/// Read Android MIDI input records.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_input_read(
    runtime_id: u64,
    session_id: u64,
    max_records: u32,
    timeout_ns: u64,
    headers: NativeSlice<AndroidHostMidiInputRecordHeader>,
    record_count_written: *mut u32,
    blob_bytes: NativeSlice<u8>,
    blob_bytes_written: *mut u32,
) -> u32 {
    if record_count_written.is_null() || blob_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.input_read,
        |callback| unsafe {
            callback(
                runtime_id,
                session_id,
                max_records,
                timeout_ns,
                headers,
                record_count_written,
                blob_bytes,
                blob_bytes_written,
            )
        },
    )
}

/// Open one Android MIDI event subscription.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_event_open(
    runtime_id: u64,
    flags: u32,
    direction_mask: u32,
    session_id: *mut u64,
) -> u32 {
    if session_id.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.event_open,
        |callback| unsafe { callback(runtime_id, flags, direction_mask, session_id) },
    )
}

/// Read Android MIDI topology events.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_event_read(
    runtime_id: u64,
    session_id: u64,
    max_events: u32,
    timeout_ns: u64,
    headers: NativeSlice<AndroidHostMidiEventHeader>,
    event_count_written: *mut u32,
    string_bytes: NativeSlice<u8>,
    string_bytes_written: *mut u32,
) -> u32 {
    if event_count_written.is_null() || string_bytes_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.event_read,
        |callback| unsafe {
            callback(
                runtime_id,
                session_id,
                max_events,
                timeout_ns,
                headers,
                event_count_written,
                string_bytes,
                string_bytes_written,
            )
        },
    )
}

/// Close one Android MIDI event subscription.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_event_close(
    runtime_id: u64,
    session_id: u64,
) -> u32 {
    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.event_close,
        |callback| unsafe { callback(runtime_id, session_id) },
    )
}

/// Write Android MIDI output records.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_host_android_midi_output_write(
    runtime_id: u64,
    session_id: u64,
    headers: NativeSlice<AndroidHostMidiOutputRecordHeader>,
    record_count: u32,
    blob_bytes: NativeSlice<u8>,
    records_written: *mut u32,
) -> u32 {
    if records_written.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    call_android_midi_callback(
        runtime_id,
        |callbacks| callbacks.output_write,
        |callback| unsafe {
            callback(
                runtime_id,
                session_id,
                headers,
                record_count,
                blob_bytes,
                records_written,
            )
        },
    )
}
