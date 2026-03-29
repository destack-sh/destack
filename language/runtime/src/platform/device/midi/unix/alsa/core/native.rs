use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::core::{self as core_platform};
use crate::platform::device::midi::core::binding_timestamp_now;

use super::super::abi::{
    SND_SEQ_EVENT_START, snd_seq_addr_t, snd_seq_port_info_t, snd_seq_port_subscribe_t,
};
use super::session::{AlsaHandle, AlsaLibrary, AlsaMidiParser, AlsaSequencerQueue};

/// Return one ALSA error string for one status code.
pub(crate) fn alsa_error_string(library: &Arc<AlsaLibrary>, status: i32) -> String {
    let value = unsafe { (library.api.snd_strerror)(status) };
    core_platform::string_from_c_str(value).unwrap_or_else(|| format!("ALSA error {status}"))
}

/// Return one runtime error for one ALSA failure.
pub(crate) fn alsa_operation_error(
    library: &Arc<AlsaLibrary>,
    operation: &'static str,
    context: &str,
    status: i32,
) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        None,
        format!("{context} failed: {}", alsa_error_string(library, status)),
    )
}

/// Open one ALSA sequencer handle.
pub(crate) fn open_sequencer_handle(
    library: &Arc<AlsaLibrary>,
    client_name: &str,
    mode: i32,
    operation: &'static str,
) -> RuntimeResult<AlsaHandle> {
    let client_name = core_platform::c_string_from_str(client_name, "name")?;
    let sequencer_name = c"default";
    let mut raw = std::ptr::null_mut();

    // handle open
    let status = unsafe { (library.api.snd_seq_open)(&mut raw, sequencer_name.as_ptr(), mode, 0) };
    if status < 0 || raw.is_null() {
        return Err(alsa_operation_error(
            library,
            operation,
            "snd_seq_open",
            status,
        ));
    }

    // client name
    let status = unsafe { (library.api.snd_seq_set_client_name)(raw, client_name.as_ptr()) };
    if status < 0 {
        unsafe {
            let _ = (library.api.snd_seq_close)(raw);
        }

        return Err(alsa_operation_error(
            library,
            operation,
            "snd_seq_set_client_name",
            status,
        ));
    }

    // client id
    let client_id = unsafe { (library.api.snd_seq_client_id)(raw) };
    if client_id < 0 {
        unsafe {
            let _ = (library.api.snd_seq_close)(raw);
        }

        return Err(alsa_operation_error(
            library,
            operation,
            "snd_seq_client_id",
            client_id,
        ));
    }

    Ok(AlsaHandle {
        library: library.clone(),
        raw,
        client_id,
    })
}

/// Create one ALSA MIDI parser.
pub(crate) fn create_midi_parser(
    library: &Arc<AlsaLibrary>,
    operation: &'static str,
) -> RuntimeResult<AlsaMidiParser> {
    let mut raw = std::ptr::null_mut();

    // parser creation
    let status = unsafe { (library.api.snd_midi_event_new)(4096, &mut raw) };
    if status < 0 || raw.is_null() {
        return Err(alsa_operation_error(
            library,
            operation,
            "snd_midi_event_new",
            status,
        ));
    }

    // parser normalization
    unsafe {
        (library.api.snd_midi_event_init)(raw);
        (library.api.snd_midi_event_no_status)(raw, 1);
    }

    Ok(AlsaMidiParser {
        library: library.clone(),
        raw,
    })
}

/// Create one simple ALSA port.
pub(crate) fn create_simple_port(
    handle: &AlsaHandle,
    name: &str,
    capability: u32,
    port_type: u32,
    operation: &'static str,
) -> RuntimeResult<i32> {
    let name = core_platform::c_string_from_str(name, "name")?;
    let port_id = unsafe {
        (handle.library.api.snd_seq_create_simple_port)(
            handle.raw,
            name.as_ptr(),
            capability,
            port_type,
        )
    };
    if port_id < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            operation,
            "snd_seq_create_simple_port",
            port_id,
        ));
    }

    Ok(port_id)
}

/// Delete one simple ALSA port.
pub(crate) fn delete_simple_port(handle: &AlsaHandle, port_id: i32) -> RuntimeResult<()> {
    let status = unsafe { (handle.library.api.snd_seq_delete_simple_port)(handle.raw, port_id) };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.alsa.port.delete",
            "snd_seq_delete_simple_port",
            status,
        ));
    }

    Ok(())
}

/// Subscribe one local destination port from one remote source with realtime timestamps.
pub(crate) fn subscribe_from_with_timestamps(
    handle: &AlsaHandle,
    local_port_id: i32,
    remote_client_id: i32,
    remote_port_id: i32,
    queue_id: i32,
) -> RuntimeResult<()> {
    let mut subscription = std::ptr::null_mut::<snd_seq_port_subscribe_t>();

    // subscription allocation
    let status = unsafe { (handle.library.api.snd_seq_port_subscribe_malloc)(&mut subscription) };
    if status < 0 || subscription.is_null() {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.input.port.open",
            "snd_seq_port_subscribe_malloc",
            status,
        ));
    }

    let sender = snd_seq_addr_t {
        client: remote_client_id as u8,
        port: remote_port_id as u8,
    };
    let dest = snd_seq_addr_t {
        client: handle.client_id as u8,
        port: local_port_id as u8,
    };

    // subscription fields
    unsafe {
        (handle.library.api.snd_seq_port_subscribe_set_sender)(subscription, &sender);
        (handle.library.api.snd_seq_port_subscribe_set_dest)(subscription, &dest);
        (handle.library.api.snd_seq_port_subscribe_set_queue)(subscription, queue_id);
        (handle.library.api.snd_seq_port_subscribe_set_time_update)(subscription, 1);
        (handle.library.api.snd_seq_port_subscribe_set_time_real)(subscription, 1);
    }

    // subscription submit
    let status = unsafe { (handle.library.api.snd_seq_subscribe_port)(handle.raw, subscription) };
    unsafe {
        (handle.library.api.snd_seq_port_subscribe_free)(subscription);
    }
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.input.port.open",
            "snd_seq_subscribe_port",
            status,
        ));
    }

    Ok(())
}

/// Remove one timestamped subscription between one remote source and one local destination.
pub(crate) fn unsubscribe_from_with_timestamps(
    handle: &AlsaHandle,
    local_port_id: i32,
    remote_client_id: i32,
    remote_port_id: i32,
) -> RuntimeResult<()> {
    let mut subscription = std::ptr::null_mut::<snd_seq_port_subscribe_t>();

    // subscription allocation
    let status = unsafe { (handle.library.api.snd_seq_port_subscribe_malloc)(&mut subscription) };
    if status < 0 || subscription.is_null() {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.input.port.close",
            "snd_seq_port_subscribe_malloc",
            status,
        ));
    }

    let sender = snd_seq_addr_t {
        client: remote_client_id as u8,
        port: remote_port_id as u8,
    };
    let dest = snd_seq_addr_t {
        client: handle.client_id as u8,
        port: local_port_id as u8,
    };

    // subscription fields
    unsafe {
        (handle.library.api.snd_seq_port_subscribe_set_sender)(subscription, &sender);
        (handle.library.api.snd_seq_port_subscribe_set_dest)(subscription, &dest);
    }

    // subscription removal
    let status = unsafe { (handle.library.api.snd_seq_unsubscribe_port)(handle.raw, subscription) };
    unsafe {
        (handle.library.api.snd_seq_port_subscribe_free)(subscription);
    }
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.input.port.close",
            "snd_seq_unsubscribe_port",
            status,
        ));
    }

    Ok(())
}

/// Connect one local source port to one remote destination.
pub(crate) fn connect_to(
    handle: &AlsaHandle,
    local_port_id: i32,
    remote_client_id: i32,
    remote_port_id: i32,
) -> RuntimeResult<()> {
    let status = unsafe {
        (handle.library.api.snd_seq_connect_to)(
            handle.raw,
            local_port_id,
            remote_client_id,
            remote_port_id,
        )
    };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.output.port.open",
            "snd_seq_connect_to",
            status,
        ));
    }

    Ok(())
}

/// Disconnect one local source port from one remote destination.
pub(crate) fn disconnect_to(
    handle: &AlsaHandle,
    local_port_id: i32,
    remote_client_id: i32,
    remote_port_id: i32,
) -> RuntimeResult<()> {
    let status = unsafe {
        (handle.library.api.snd_seq_disconnect_to)(
            handle.raw,
            local_port_id,
            remote_client_id,
            remote_port_id,
        )
    };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.output.port.close",
            "snd_seq_disconnect_to",
            status,
        ));
    }

    Ok(())
}

/// Allocate and start one ALSA queue for realtime timestamping or scheduling.
pub(crate) fn create_queue(
    handle: &AlsaHandle,
    name: &str,
    operation: &'static str,
) -> RuntimeResult<AlsaSequencerQueue> {
    let name = core_platform::c_string_from_str(name, "name")?;
    let queue_id =
        unsafe { (handle.library.api.snd_seq_alloc_named_queue)(handle.raw, name.as_ptr()) };
    if queue_id < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            operation,
            "snd_seq_alloc_named_queue",
            queue_id,
        ));
    }

    // queue start
    let before_start_ns = binding_timestamp_now();
    let status = unsafe {
        (handle.library.api.snd_seq_control_queue)(
            handle.raw,
            queue_id,
            SND_SEQ_EVENT_START,
            0,
            std::ptr::null_mut(),
        )
    };
    if status < 0 {
        let _ = unsafe { (handle.library.api.snd_seq_free_queue)(handle.raw, queue_id) };
        return Err(alsa_operation_error(
            &handle.library,
            operation,
            "snd_seq_control_queue",
            status,
        ));
    }

    let status = unsafe { (handle.library.api.snd_seq_drain_output)(handle.raw) };
    if status < 0 {
        let _ = unsafe { (handle.library.api.snd_seq_free_queue)(handle.raw, queue_id) };
        return Err(alsa_operation_error(
            &handle.library,
            operation,
            "snd_seq_drain_output",
            status,
        ));
    }
    let after_start_ns = binding_timestamp_now();
    let start_epoch_ns =
        before_start_ns.saturating_add((after_start_ns.saturating_sub(before_start_ns)) / 2);

    Ok(AlsaSequencerQueue {
        id: queue_id,
        start_epoch_ns,
    })
}

/// Release one ALSA queue.
pub(crate) fn free_queue(handle: &AlsaHandle, queue_id: i32) -> RuntimeResult<()> {
    let status = unsafe { (handle.library.api.snd_seq_free_queue)(handle.raw, queue_id) };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.device.midi.alsa.queue.free",
            "snd_seq_free_queue",
            status,
        ));
    }

    Ok(())
}

/// Enable realtime timestamping for one local destination port.
pub(crate) fn enable_port_realtime_timestamps(
    handle: &AlsaHandle,
    port_id: i32,
    queue_id: i32,
    operation: &'static str,
) -> RuntimeResult<()> {
    let mut info = std::ptr::null_mut::<snd_seq_port_info_t>();

    // port-info allocation
    let status = unsafe { (handle.library.api.snd_seq_port_info_malloc)(&mut info) };
    if status < 0 || info.is_null() {
        return Err(alsa_operation_error(
            &handle.library,
            operation,
            "snd_seq_port_info_malloc",
            status,
        ));
    }

    // current port info
    let status = unsafe { (handle.library.api.snd_seq_get_port_info)(handle.raw, port_id, info) };
    if status < 0 {
        unsafe {
            (handle.library.api.snd_seq_port_info_free)(info);
        }
        return Err(alsa_operation_error(
            &handle.library,
            operation,
            "snd_seq_get_port_info",
            status,
        ));
    }

    // timestamp settings
    unsafe {
        (handle.library.api.snd_seq_port_info_set_timestamping)(info, 1);
        (handle.library.api.snd_seq_port_info_set_timestamp_real)(info, 1);
        (handle.library.api.snd_seq_port_info_set_timestamp_queue)(info, queue_id);
    }

    // port-info update
    let status = unsafe { (handle.library.api.snd_seq_set_port_info)(handle.raw, port_id, info) };
    unsafe {
        (handle.library.api.snd_seq_port_info_free)(info);
    }
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            operation,
            "snd_seq_set_port_info",
            status,
        ));
    }

    Ok(())
}

/// Decode one runtime-owned string from one native string reference.
pub(crate) fn native_string(value: NativeStringRef) -> RuntimeResult<String> {
    let value = unsafe { value.as_str()? };

    Ok(value.to_string())
}

/// Decode one optional runtime-owned string from one native string reference.
pub(crate) fn native_optional_string(
    value: Option<NativeStringRef>,
) -> RuntimeResult<Option<String>> {
    match value {
        Some(value) => Ok(Some(native_string(value)?)),
        None => Ok(None),
    }
}
