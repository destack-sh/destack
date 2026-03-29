use std::collections::BTreeMap;
use std::ffi::{CStr, c_char, c_int, c_ulong};
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::core::{self as core_platform, DynamicLibrary};
use crate::platform::device::midi::core::binding_timestamp_now;
use crate::platform::{PlatformError, PlatformErrorCode};

use super::super::abi::{JackApi, JackClient, JackPort};
use super::session::{
    JackEndpointInfo, JackInputCallbackContext, JackOutputCallbackContext, JackTopologyState,
};
use super::transport::{INTERNAL_CLIENT_NAME_PREFIX, is_internal_client_name, split_port_name};

/// JACK option: do not auto-start the server.
const JACK_OPTION_NO_START_SERVER: u32 = 0x01;
/// JACK port flag: input port.
const JACK_PORT_IS_INPUT: c_ulong = 0x1;
/// JACK port flag: output port.
const JACK_PORT_IS_OUTPUT: c_ulong = 0x2;
/// JACK port flag: terminal port.
const JACK_PORT_IS_TERMINAL: c_ulong = 0x8;
/// JACK MIDI type name.
const JACK_MIDI_TYPE: &[u8] = b"8 bit raw midi\0";
/// One nanosecond multiplier for JACK microsecond timestamps.
const JACK_USECS_TO_NANOS: u64 = 1_000;

/// One loaded JACK library payload.
pub(crate) struct JackLibrary {
    /// The owning dynamic library handle.
    _library: DynamicLibrary,
    /// The loaded JACK symbol table.
    pub(crate) api: JackApi,
}

/// One opened JACK client handle.
pub(crate) struct JackClientHandle {
    /// Shared JACK library owner.
    pub(crate) library: Arc<JackLibrary>,
    /// Raw JACK client pointer.
    pub(crate) raw: *mut JackClient,
}

impl JackClientHandle {
    /// Return whether the handle currently owns one live client.
    fn is_live(&self) -> bool {
        !self.raw.is_null()
    }
}

unsafe impl Send for JackClientHandle {}
unsafe impl Sync for JackClientHandle {}

impl Drop for JackClientHandle {
    /// Deactivate and close one client on drop.
    fn drop(&mut self) {
        if !self.is_live() {
            return;
        }

        // client teardown
        unsafe {
            let _ = (self.library.api.jack_deactivate)(self.raw);
            let _ = (self.library.api.jack_client_close)(self.raw);
        }

        self.raw = std::ptr::null_mut();
    }
}

/// Check whether the JACK backend is reachable.
pub(crate) fn check_jack_support(operation: &'static str) -> RuntimeResult<()> {
    let _library = require_jack_library(operation)?;

    Ok(())
}

/// Return one generic JACK operation error.
pub(crate) fn jack_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    core_platform::io_operation_error(operation, Some(PlatformErrorCode::IoInvalidData), message)
}

/// Return one JACK support error.
pub(crate) fn jack_not_supported(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: {}",
        message.into()
    )))
    .boxed()
}

/// Return whether one JACK status code denotes success.
pub(crate) fn jack_succeeded(status: c_int) -> bool {
    status == 0
}

/// Return one loaded JACK library or one not-supported error.
pub(crate) fn require_jack_library(operation: &'static str) -> RuntimeResult<Arc<JackLibrary>> {
    load_jack_library().map_err(|error| jack_not_supported(operation, error))
}

/// Convert one JACK microsecond timestamp into runtime monotonic nanoseconds.
pub(crate) fn jack_time_usecs_to_mono_ns(
    open_epoch_ns: u64,
    open_jack_time_usecs: u64,
    event_time_usecs: u64,
) -> u64 {
    let delta_usecs = event_time_usecs.saturating_sub(open_jack_time_usecs);
    let delta_ns = delta_usecs.saturating_mul(JACK_USECS_TO_NANOS);

    open_epoch_ns.saturating_add(delta_ns)
}

/// Convert one JACK callback event offset into runtime monotonic nanoseconds.
pub(crate) fn jack_event_time_to_mono_ns(
    context: &JackInputCallbackContext,
    frame_count: u32,
    frame_offset: u32,
) -> u64 {
    let mut _current_frames = 0u32;
    let mut current_usecs = 0u64;
    let mut next_usecs = 0u64;
    let mut _period_usecs = 0f32;
    let cycle_status = unsafe {
        (context.library.api.jack_get_cycle_times)(
            context.client,
            &mut _current_frames,
            &mut current_usecs,
            &mut next_usecs,
            &mut _period_usecs,
        )
    };

    if cycle_status == 0 {
        let cycle_duration_usecs = next_usecs.saturating_sub(current_usecs);
        let clamped_frame_offset = frame_offset.min(frame_count);
        let event_offset_usecs = if frame_count == 0 {
            0
        } else {
            ((u128::from(clamped_frame_offset) * u128::from(cycle_duration_usecs))
                / u128::from(frame_count)) as u64
        };
        let event_time_usecs = current_usecs.saturating_add(event_offset_usecs);

        return jack_time_usecs_to_mono_ns(
            context.open_epoch_ns,
            context.open_jack_time_usecs,
            event_time_usecs,
        );
    }

    let event_time_usecs = unsafe { (context.library.api.jack_get_time)() };

    jack_time_usecs_to_mono_ns(
        context.open_epoch_ns,
        context.open_jack_time_usecs,
        event_time_usecs,
    )
}

/// Return one pointer to the JACK MIDI type string.
pub(crate) fn jack_midi_type_pointer() -> *const c_char {
    JACK_MIDI_TYPE.as_ptr().cast::<c_char>()
}

/// Return one probe client name.
pub(crate) fn probe_client_name() -> String {
    format!("destack-midi-probe-{}", std::process::id())
}

/// Return one monitor client name.
pub(crate) fn monitor_client_name() -> String {
    format!("destack-midi-monitor-{}", std::process::id())
}

/// Return one hidden input-session client name.
pub(crate) fn internal_input_client_name() -> String {
    format!(
        "{INTERNAL_CLIENT_NAME_PREFIX}input-{}",
        binding_timestamp_now()
    )
}

/// Return one hidden output-session client name.
pub(crate) fn internal_output_client_name() -> String {
    format!(
        "{INTERNAL_CLIENT_NAME_PREFIX}output-{}",
        binding_timestamp_now()
    )
}

/// Return one owned probe client.
pub(crate) fn open_probe_client(operation: &'static str) -> RuntimeResult<JackClientHandle> {
    let library = require_jack_library(operation)?;
    let client_name = probe_client_name();

    open_jack_client(library, operation, &client_name)
}

/// Open one JACK client for one operation.
pub(crate) fn open_jack_client(
    library: Arc<JackLibrary>,
    operation: &'static str,
    client_name: &str,
) -> RuntimeResult<JackClientHandle> {
    let client_name = core_platform::c_string_from_str(client_name, "clientName")?;
    let mut status = 0u32;

    // client open
    let client = unsafe {
        (library.api.jack_client_open)(
            client_name.as_ptr(),
            JACK_OPTION_NO_START_SERVER,
            &mut status,
        )
    };
    if client.is_null() {
        return Err(jack_not_supported(
            operation,
            format!("failed to open JACK client (status 0x{status:08x})"),
        ));
    }

    Ok(JackClientHandle {
        library,
        raw: client,
    })
}

/// Return one Rust string from one JACK C string pointer.
pub(crate) fn c_str_to_string(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }

    let text = unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned();
    if text.is_empty() {
        return None;
    }

    Some(text)
}

/// Return one newly registered local input port.
pub(crate) fn register_input_port(
    client: &JackClientHandle,
    name: &str,
    operation: &'static str,
) -> RuntimeResult<*mut JackPort> {
    register_port(
        client,
        name,
        JACK_PORT_IS_INPUT | JACK_PORT_IS_TERMINAL,
        operation,
    )
}

/// Return one newly registered local output port.
pub(crate) fn register_output_port(
    client: &JackClientHandle,
    name: &str,
    operation: &'static str,
) -> RuntimeResult<*mut JackPort> {
    register_port(
        client,
        name,
        JACK_PORT_IS_OUTPUT | JACK_PORT_IS_TERMINAL,
        operation,
    )
}

/// Unregister one local JACK port.
pub(crate) fn unregister_port(
    client: &JackClientHandle,
    port: *mut JackPort,
    operation: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe { (client.library.api.jack_port_unregister)(client.raw, port) };
    if !jack_succeeded(status) {
        return Err(jack_error(
            operation,
            format!("failed to unregister JACK port (status {status})"),
        ));
    }

    Ok(())
}

/// Register one JACK port with one explicit flag set.
fn register_port(
    client: &JackClientHandle,
    name: &str,
    flags: c_ulong,
    operation: &'static str,
) -> RuntimeResult<*mut JackPort> {
    let name = core_platform::c_string_from_str(name, "name")?;

    // port registration
    let port = unsafe {
        (client.library.api.jack_port_register)(
            client.raw,
            name.as_ptr(),
            jack_midi_type_pointer(),
            flags,
            0,
        )
    };
    if port.is_null() {
        return Err(jack_error(operation, "failed to register JACK MIDI port"));
    }

    Ok(port)
}

/// Activate one JACK client.
pub(crate) fn activate_client(
    client: &JackClientHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let status = unsafe { (client.library.api.jack_activate)(client.raw) };
    if !jack_succeeded(status) {
        return Err(jack_error(
            operation,
            format!("failed to activate JACK client (status {status})"),
        ));
    }

    Ok(())
}

/// Connect one JACK source port to one JACK destination port.
pub(crate) fn connect_ports(
    client: &JackClientHandle,
    source_port_name: &str,
    destination_port_name: &str,
    operation: &'static str,
) -> RuntimeResult<()> {
    let source_port_name = core_platform::c_string_from_str(source_port_name, "sourcePortName")?;
    let destination_port_name =
        core_platform::c_string_from_str(destination_port_name, "destinationPortName")?;

    let status = unsafe {
        (client.library.api.jack_connect)(
            client.raw,
            source_port_name.as_ptr(),
            destination_port_name.as_ptr(),
        )
    };
    if !jack_succeeded(status) {
        return Err(jack_error(
            operation,
            format!("failed to connect JACK ports (status {status})"),
        ));
    }

    Ok(())
}

/// Return one full JACK port name for one local port.
pub(crate) fn port_name(
    client: &JackClientHandle,
    port: *mut JackPort,
    operation: &'static str,
) -> RuntimeResult<String> {
    let port_name = unsafe { (client.library.api.jack_port_name)(port) };
    c_str_to_string(port_name).ok_or_else(|| jack_error(operation, "failed to read JACK port name"))
}

/// Retain one input callback context for the JACK callback lifetime.
pub(crate) fn retain_input_callback_context(context: &Arc<JackInputCallbackContext>) -> usize {
    Arc::into_raw(context.clone()) as usize
}

/// Release one retained input callback context token.
pub(crate) fn release_input_callback_context(token: usize) {
    if token == 0 {
        return;
    }

    unsafe {
        Arc::decrement_strong_count(token as *const JackInputCallbackContext);
    }
}

/// Retain one output callback context for the JACK callback lifetime.
pub(crate) fn retain_output_callback_context(context: &Arc<JackOutputCallbackContext>) -> usize {
    Arc::into_raw(context.clone()) as usize
}

/// Release one retained output callback context token.
pub(crate) fn release_output_callback_context(token: usize) {
    if token == 0 {
        return;
    }

    unsafe {
        Arc::decrement_strong_count(token as *const JackOutputCallbackContext);
    }
}

/// Return one current MIDI topology snapshot.
pub(crate) fn query_topology_snapshot(operation: &'static str) -> RuntimeResult<JackTopologyState> {
    let client = open_probe_client(operation)?;
    let input_rows = query_direction_snapshot(
        &client,
        crate::platform::device::MidiPortDirection::Input,
        operation,
    )?;
    let output_rows = query_direction_snapshot(
        &client,
        crate::platform::device::MidiPortDirection::Output,
        operation,
    )?;

    Ok(JackTopologyState {
        inputs: input_rows,
        outputs: output_rows,
    })
}

/// Query one direction-scoped JACK topology snapshot.
fn query_direction_snapshot(
    client: &JackClientHandle,
    direction: crate::platform::device::MidiPortDirection,
    operation: &'static str,
) -> RuntimeResult<BTreeMap<String, JackEndpointInfo>> {
    let flags = match direction {
        crate::platform::device::MidiPortDirection::Input => JACK_PORT_IS_OUTPUT,
        crate::platform::device::MidiPortDirection::Output => JACK_PORT_IS_INPUT,
    };
    let port_array = unsafe {
        (client.library.api.jack_get_ports)(
            client.raw,
            std::ptr::null(),
            jack_midi_type_pointer(),
            flags,
        )
    };
    if port_array.is_null() {
        return Ok(BTreeMap::new());
    }

    let mut rows = BTreeMap::new();
    let mut index = 0usize;

    // port array decode
    loop {
        let port_pointer = unsafe { *port_array.add(index) };
        if port_pointer.is_null() {
            break;
        }

        let Some(port_name) = c_str_to_string(port_pointer) else {
            unsafe {
                (client.library.api.jack_free)(port_array.cast());
            }

            return Err(jack_error(
                operation,
                "failed to decode JACK MIDI port name",
            ));
        };
        let (client_name, _) = split_port_name(&port_name);

        // hide internal session clients from topology
        if is_internal_client_name(client_name) {
            index = index.saturating_add(1);
            continue;
        }

        let port_name_c = core_platform::c_string_from_str(&port_name, "portName")?;
        let port =
            unsafe { (client.library.api.jack_port_by_name)(client.raw, port_name_c.as_ptr()) };
        if port.is_null() {
            index = index.saturating_add(1);
            continue;
        }

        let port_flags = unsafe { (client.library.api.jack_port_flags)(port) };
        let descriptor =
            super::super::descriptor::endpoint_descriptor(direction, &port_name, port_flags);
        let snapshot_key = descriptor.id.clone();
        rows.insert(
            snapshot_key,
            JackEndpointInfo {
                descriptor,
                backend_port_name: port_name,
            },
        );

        index = index.saturating_add(1);
    }

    unsafe {
        (client.library.api.jack_free)(port_array.cast());
    }

    Ok(rows)
}

/// Load one JACK dynamic library and required symbol table.
fn load_jack_library() -> Result<Arc<JackLibrary>, String> {
    let (library, api) =
        core_platform::load_library_with_api(&["libjack.so.0", "libjack.so"], load_jack_api)?;

    Ok(Arc::new(JackLibrary {
        _library: library,
        api,
    }))
}

/// Load one JACK symbol table from one open dynamic library handle.
fn load_jack_api(library: &DynamicLibrary, candidate: &str) -> Result<JackApi, String> {
    core_platform::load_dll_api_bytes!(library, candidate, JackApi {
        jack_client_open => b"jack_client_open\0",
        jack_client_close => b"jack_client_close\0",
        jack_activate => b"jack_activate\0",
        jack_deactivate => b"jack_deactivate\0",
        jack_set_process_callback => b"jack_set_process_callback\0",
        jack_set_port_registration_callback => b"jack_set_port_registration_callback\0",
        jack_set_port_connect_callback => b"jack_set_port_connect_callback\0",
        jack_on_shutdown => b"jack_on_shutdown\0",
        jack_port_register => b"jack_port_register\0",
        jack_port_unregister => b"jack_port_unregister\0",
        jack_port_name => b"jack_port_name\0",
        jack_port_by_name => b"jack_port_by_name\0",
        jack_port_flags => b"jack_port_flags\0",
        jack_port_get_buffer => b"jack_port_get_buffer\0",
        jack_get_ports => b"jack_get_ports\0",
        jack_connect => b"jack_connect\0",
        jack_get_cycle_times => b"jack_get_cycle_times\0",
        jack_get_time => b"jack_get_time\0",
        jack_free => b"jack_free\0",
        jack_midi_get_event_count => b"jack_midi_get_event_count\0",
        jack_midi_event_get => b"jack_midi_event_get\0",
        jack_midi_clear_buffer => b"jack_midi_clear_buffer\0",
        jack_midi_event_write => b"jack_midi_event_write\0",
    })
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

#[cfg(test)]
mod tests {
    use super::jack_time_usecs_to_mono_ns;

    /// Project JACK microsecond timestamps from the session epoch.
    #[test]
    fn test_jack_time_usecs_to_mono_ns_uses_the_open_epoch() {
        let received_at_ns = jack_time_usecs_to_mono_ns(2_000_000_000, 5_000_000, 5_250_000);

        assert_eq!(received_at_ns, 2_250_000_000);
    }

    /// Saturate large JACK timestamp deltas instead of wrapping.
    #[test]
    fn test_jack_time_usecs_to_mono_ns_saturates_large_deltas() {
        let received_at_ns = jack_time_usecs_to_mono_ns(u64::MAX - 10, 1, u64::MAX);

        assert_eq!(received_at_ns, u64::MAX);
    }
}
