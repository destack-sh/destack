use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{self as core_platform, DynamicLibrary};
pub(super) use crate::platform::midi::core::{
    BoundedQueue, MIDI_EVENT_RESOURCE_LABEL, MIDI_INPUT_RESOURCE_LABEL, MIDI_OUTPUT_RESOURCE_LABEL,
    binding_timestamp_now, direction_mask_includes, endpoint_direction_name, event_poll_interval,
    event_queue_capacity, event_snapshot_list_flags, input_queue_capacity,
};
use crate::platform::midi::core::{MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue};
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1, MidiBackend, MidiDataFormat,
    MidiDataFormatFlags, MidiEventOverflowPolicy, MidiEventSubscriptionFlags, MidiPortDirection,
    MidiPortDirectionFlags, MidiProtocol, MidiProtocolFlags, MidiRecordFraming,
};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeStringRef, resource};
use crate::runtime::BindingCallContext;

use super::abi::{
    AlsaApi, SND_SEQ_ADDRESS_SUBSCRIBERS, SND_SEQ_PORT_CAP_NO_EXPORT, SND_SEQ_PORT_CAP_READ,
    SND_SEQ_PORT_CAP_SUBS_READ, SND_SEQ_PORT_CAP_SUBS_WRITE, SND_SEQ_PORT_CAP_WRITE,
    SND_SEQ_PORT_TYPE_APPLICATION, SND_SEQ_QUEUE_DIRECT, snd_midi_event_t, snd_seq_event_t,
    snd_seq_t,
};
use super::service::AlsaService;

/// Prefix used for internal hidden ALSA clients.
pub(super) const INTERNAL_CLIENT_NAME_PREFIX: &str = "Destack MIDI Internal";

/// One opened ALSA sequencer dynamic library.
pub(super) struct AlsaLibrary {
    /// The owning dynamic-library handle.
    pub(super) _library: DynamicLibrary,
    /// The loaded symbol table.
    pub(super) api: AlsaApi,
}

/// One owned ALSA sequencer handle.
pub(super) struct AlsaHandle {
    /// Shared library owner.
    pub(super) library: Arc<AlsaLibrary>,
    /// Raw ALSA sequencer handle.
    pub(super) raw: *mut snd_seq_t,
    /// The current ALSA client id.
    pub(super) client_id: i32,
}

/// One owned ALSA MIDI parser.
pub(super) struct AlsaMidiParser {
    /// Shared library owner.
    pub(super) library: Arc<AlsaLibrary>,
    /// Raw parser handle.
    pub(super) raw: *mut snd_midi_event_t,
}

/// One stable ALSA endpoint key.
pub(super) type AlsaEndpointKey = (i32, i32);

/// One cached ALSA endpoint row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AlsaEndpointInfo {
    /// Descriptor snapshot for this endpoint.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// ALSA client id.
    pub(super) client_id: i32,
    /// ALSA port id.
    pub(super) port_id: i32,
}

/// One process-global snapshot of ALSA sequencer topology.
#[derive(Default)]
pub(super) struct AlsaTopologyState {
    /// Current input endpoint rows.
    pub(super) inputs: BTreeMap<AlsaEndpointKey, AlsaEndpointInfo>,
    /// Current output endpoint rows.
    pub(super) outputs: BTreeMap<AlsaEndpointKey, AlsaEndpointInfo>,
}

/// One opened ALSA input-session kind.
pub(super) enum AlsaInputSessionKind {
    /// One host source connected into one hidden destination port.
    Source {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The local destination port id.
        local_port_id: i32,
        /// The connected remote source client id.
        remote_client_id: i32,
        /// The connected remote source port id.
        remote_port_id: i32,
        /// Stop flag for the reader thread.
        stop_flag: Arc<AtomicBool>,
        /// The reader thread handle.
        reader_thread: Option<JoinHandle<()>>,
    },
    /// One runtime-owned virtual destination port.
    VirtualDestination {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The visible destination port id.
        local_port_id: i32,
        /// Stop flag for the reader thread.
        stop_flag: Arc<AtomicBool>,
        /// The reader thread handle.
        reader_thread: Option<JoinHandle<()>>,
    },
}

/// One opened ALSA output-session kind.
pub(super) enum AlsaOutputSessionKind {
    /// One hidden source connected to one remote destination.
    Destination {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The local source port id.
        local_port_id: i32,
        /// The connected remote destination client id.
        remote_client_id: i32,
        /// The connected remote destination port id.
        remote_port_id: i32,
        /// The MIDI encoder state.
        parser: Mutex<AlsaMidiParser>,
    },
    /// One runtime-owned virtual source port.
    VirtualSource {
        /// The owned ALSA client handle.
        handle: AlsaHandle,
        /// The visible source port id.
        local_port_id: i32,
        /// The MIDI encoder state.
        parser: Mutex<AlsaMidiParser>,
    },
}

/// One opened ALSA input session.
pub(super) struct AlsaInputSession {
    /// Shared service owner.
    pub(super) _service: Arc<AlsaService>,
    /// Current descriptor snapshot.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// Shared input queue.
    pub(super) queue: Arc<BoundedQueue<MidiInputRecordValue>>,
    /// Deferred terminal reader failure.
    pub(super) terminal_error: Arc<Mutex<Option<AlsaInputTerminalError>>>,
    /// Session resources that must be released.
    pub(super) kind: Mutex<AlsaInputSessionKind>,
}

/// One terminal input-reader failure.
#[derive(Debug, Clone)]
pub(super) enum AlsaInputTerminalError {
    /// The reader could not create one ALSA MIDI parser.
    ParserInitializationFailed,
    /// The reader could not acquire any poll descriptors.
    PollDescriptorUnavailable,
    /// The reader could not query the current poll descriptors.
    PollDescriptorQueryFailed,
    /// The sequencer input stream failed after the session was opened.
    SequencerInputFailed(String),
}

impl AlsaInputTerminalError {
    /// Return one stable runtime error message for this terminal failure.
    pub(super) fn message(&self) -> String {
        match self {
            Self::ParserInitializationFailed => {
                "ALSA input reader could not initialize one MIDI parser".to_string()
            }
            Self::PollDescriptorUnavailable => {
                "ALSA input reader could not acquire any poll descriptors".to_string()
            }
            Self::PollDescriptorQueryFailed => {
                "ALSA input reader could not query poll descriptors".to_string()
            }
            Self::SequencerInputFailed(error) => {
                format!("ALSA input reader lost sequencer input: {error}")
            }
        }
    }
}

/// One opened ALSA output session.
pub(super) struct AlsaOutputSession {
    /// Shared service owner.
    pub(super) _service: Arc<AlsaService>,
    /// Current descriptor snapshot.
    pub(super) descriptor: MidiPortDescriptorValue,
    /// Selected data format.
    pub(super) data_format: MidiDataFormat,
    /// Selected protocol.
    pub(super) protocol: Option<MidiProtocol>,
    /// Session resources that must be released.
    pub(super) kind: AlsaOutputSessionKind,
}

/// One stable key for one direction-scoped snapshot row.
pub(super) type SnapshotKey = String;

/// One event delivery mode for one opened subscription.
#[derive(Clone)]
pub(super) enum AlsaEventDeliveryKind {
    /// Use the native ALSA announce feed.
    Native {
        /// Shared native event registry.
        registry: Arc<Mutex<super::service::AlsaNativeEventRegistry>>,
        /// Registration id in the native registry.
        registration_id: u64,
    },
    /// Use synthetic polling snapshots.
    Poll,
}

/// One opened MIDI event subscription.
#[derive(Clone)]
pub(super) struct AlsaEventSession {
    /// Selected backend for the subscription.
    pub(super) backend: MidiBackend,
    /// Included directions.
    pub(super) direction_mask: MidiPortDirectionFlags,
    /// Subscription flags.
    pub(super) flags: MidiEventSubscriptionFlags,
    /// Overflow policy.
    pub(super) overflow_policy: MidiEventOverflowPolicy,
    /// Poll interval for synthetic snapshots.
    pub(super) poll_interval: Duration,
    /// Selected delivery kind.
    pub(super) delivery_kind: AlsaEventDeliveryKind,
    /// Pending event queue.
    pub(super) queue: Arc<BoundedQueue<MidiEventValue>>,
    /// Next sequence number.
    pub(super) next_sequence: u64,
    /// Previous endpoint snapshot.
    pub(super) snapshot: BTreeMap<SnapshotKey, (MidiPortDirection, MidiPortDescriptorValue)>,
}

/// Resource payload for one input session.
pub(super) struct AlsaInputResource {
    /// Shared session state.
    pub(super) session: Arc<AlsaInputSession>,
}

/// Resource payload for one output session.
pub(super) struct AlsaOutputResource {
    /// Shared session state.
    pub(super) session: Arc<AlsaOutputSession>,
}

/// Resource payload for one event subscription.
#[derive(Clone)]
pub(super) struct AlsaEventResource {
    /// Shared subscription state.
    pub(super) session: Arc<Mutex<AlsaEventSession>>,
}

unsafe impl Send for AlsaHandle {}
unsafe impl Sync for AlsaHandle {}
unsafe impl Send for AlsaMidiParser {}

impl Drop for AlsaHandle {
    /// Close one owned ALSA sequencer handle.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        unsafe {
            let _ = (self.library.api.snd_seq_close)(self.raw);
        }
    }
}

impl Drop for AlsaMidiParser {
    /// Release one ALSA MIDI parser.
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        unsafe {
            (self.library.api.snd_midi_event_free)(self.raw);
        }
    }
}

impl Drop for AlsaInputSession {
    /// Release ALSA resources for one input session.
    fn drop(&mut self) {
        let mut kind = self.kind.lock();

        // stop the reader thread before tearing down the ALSA handle
        match &mut *kind {
            AlsaInputSessionKind::Source {
                stop_flag,
                reader_thread,
                ..
            }
            | AlsaInputSessionKind::VirtualDestination {
                stop_flag,
                reader_thread,
                ..
            } => {
                stop_flag.store(true, Ordering::Release);

                if let Some(reader_thread) = reader_thread.take() {
                    let _ = reader_thread.join();
                }
            }
        }

        // session-specific teardown
        match &*kind {
            AlsaInputSessionKind::Source {
                handle,
                local_port_id,
                remote_client_id,
                remote_port_id,
                ..
            } => {
                let _ = disconnect_from(handle, *local_port_id, *remote_client_id, *remote_port_id);
                let _ = delete_simple_port(handle, *local_port_id);
            }
            AlsaInputSessionKind::VirtualDestination {
                handle,
                local_port_id,
                ..
            } => {
                let _ = delete_simple_port(handle, *local_port_id);
            }
        }

        self.queue.close();
    }
}

impl Drop for AlsaOutputSession {
    /// Release ALSA resources for one output session.
    fn drop(&mut self) {
        match &self.kind {
            AlsaOutputSessionKind::Destination {
                handle,
                local_port_id,
                remote_client_id,
                remote_port_id,
                ..
            } => {
                let _ = disconnect_to(handle, *local_port_id, *remote_client_id, *remote_port_id);
                let _ = delete_simple_port(handle, *local_port_id);
            }
            AlsaOutputSessionKind::VirtualSource {
                handle,
                local_port_id,
                ..
            } => {
                let _ = delete_simple_port(handle, *local_port_id);
            }
        }
    }
}

/// Return the exact transport support advertised by ALSA sequencer endpoints.
pub(super) fn exact_transport_support() -> (
    MidiDataFormatFlags,
    Option<MidiDataFormat>,
    MidiProtocolFlags,
    Option<MidiProtocol>,
) {
    (
        MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES.0),
        Some(MidiDataFormat::Midi1Bytes),
        MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0),
        Some(MidiProtocol::Midi1),
    )
}

/// Return one endpoint group id for one ALSA client.
pub(super) fn group_id(client_id: i32) -> String {
    format!("alsa:client:{client_id}")
}

/// Return one stable runtime id for one ALSA endpoint.
pub(super) fn runtime_id(direction: MidiPortDirection, client_id: i32, port_id: i32) -> String {
    let direction_name = endpoint_direction_name(direction);

    format!("alsa:{direction_name}:{client_id}:{port_id}")
}

/// Return one stable backend id for one ALSA endpoint.
pub(super) fn backend_id(client_id: i32, port_id: i32) -> String {
    format!("{client_id}:{port_id}")
}

/// Parse one stable ALSA backend id.
pub(super) fn parse_backend_id(
    backend_id: &str,
    operation: &'static str,
) -> RuntimeResult<(i32, i32)> {
    let Some((client_id, port_id)) = backend_id.split_once(':') else {
        return Err(core_platform::invalid_argument(
            "backendId",
            format!("{operation}: malformed ALSA backend id"),
        ));
    };

    let client_id = client_id.parse::<i32>().map_err(|_| {
        core_platform::invalid_argument("backendId", format!("{operation}: invalid ALSA client id"))
    })?;
    let port_id = port_id.parse::<i32>().map_err(|_| {
        core_platform::invalid_argument("backendId", format!("{operation}: invalid ALSA port id"))
    })?;

    Ok((client_id, port_id))
}

/// Return whether one port should be exposed as one input source.
pub(super) fn is_input_source(capability: u32) -> bool {
    let required = SND_SEQ_PORT_CAP_READ | SND_SEQ_PORT_CAP_SUBS_READ;

    capability & required == required
}

/// Return whether one port should be exposed as one output destination.
pub(super) fn is_output_destination(capability: u32) -> bool {
    let required = SND_SEQ_PORT_CAP_WRITE | SND_SEQ_PORT_CAP_SUBS_WRITE;

    capability & required == required
}

/// Return whether one port looks virtual.
pub(super) fn is_virtual_port(port_type: u32) -> bool {
    port_type & SND_SEQ_PORT_TYPE_APPLICATION != 0
}

/// Return whether one client name belongs to Destack internal plumbing.
pub(super) fn is_internal_client_name(name: &str) -> bool {
    name.starts_with(INTERNAL_CLIENT_NAME_PREFIX)
}

/// Return one Rust string from one C string pointer.
pub(super) fn c_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }

    let value = unsafe { CStr::from_ptr(value) };
    let value = value.to_str().ok()?;

    Some(value.to_string())
}

/// Return one NUL-terminated string for ALSA calls.
pub(super) fn owned_c_string(
    operation: &'static str,
    value: &str,
    field: &str,
) -> RuntimeResult<CString> {
    CString::new(value).map_err(|_| {
        core_platform::invalid_argument(field, format!("{operation}: string contains NUL bytes"))
    })
}

/// Return one ALSA error string for one status code.
pub(super) fn alsa_error_string(library: &Arc<AlsaLibrary>, status: i32) -> String {
    let value = unsafe { (library.api.snd_strerror)(status) };
    c_string(value).unwrap_or_else(|| format!("ALSA error {status}"))
}

/// Return one runtime error for one ALSA failure.
pub(super) fn alsa_operation_error(
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
pub(super) fn open_sequencer_handle(
    library: &Arc<AlsaLibrary>,
    client_name: &str,
    mode: i32,
    operation: &'static str,
) -> RuntimeResult<AlsaHandle> {
    let client_name = owned_c_string(operation, client_name, "name")?;
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
pub(super) fn create_midi_parser(
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
pub(super) fn create_simple_port(
    handle: &AlsaHandle,
    name: &str,
    capability: u32,
    port_type: u32,
    operation: &'static str,
) -> RuntimeResult<i32> {
    let name = owned_c_string(operation, name, "name")?;
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
pub(super) fn delete_simple_port(handle: &AlsaHandle, port_id: i32) -> RuntimeResult<()> {
    let status = unsafe { (handle.library.api.snd_seq_delete_simple_port)(handle.raw, port_id) };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.midi.alsa.port.delete",
            "snd_seq_delete_simple_port",
            status,
        ));
    }

    Ok(())
}

/// Connect one local destination port from one remote source.
pub(super) fn connect_from(
    handle: &AlsaHandle,
    local_port_id: i32,
    remote_client_id: i32,
    remote_port_id: i32,
) -> RuntimeResult<()> {
    let status = unsafe {
        (handle.library.api.snd_seq_connect_from)(
            handle.raw,
            local_port_id,
            remote_client_id,
            remote_port_id,
        )
    };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.midi.input.port.open",
            "snd_seq_connect_from",
            status,
        ));
    }

    Ok(())
}

/// Disconnect one local destination port from one remote source.
pub(super) fn disconnect_from(
    handle: &AlsaHandle,
    local_port_id: i32,
    remote_client_id: i32,
    remote_port_id: i32,
) -> RuntimeResult<()> {
    let status = unsafe {
        (handle.library.api.snd_seq_disconnect_from)(
            handle.raw,
            local_port_id,
            remote_client_id,
            remote_port_id,
        )
    };
    if status < 0 {
        return Err(alsa_operation_error(
            &handle.library,
            "destack.midi.input.port.close",
            "snd_seq_disconnect_from",
            status,
        ));
    }

    Ok(())
}

/// Connect one local source port to one remote destination.
pub(super) fn connect_to(
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
            "destack.midi.output.port.open",
            "snd_seq_connect_to",
            status,
        ));
    }

    Ok(())
}

/// Disconnect one local source port from one remote destination.
pub(super) fn disconnect_to(
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
            "destack.midi.output.port.close",
            "snd_seq_disconnect_to",
            status,
        ));
    }

    Ok(())
}

/// Return the hidden port capabilities for one internal source port.
pub(super) fn hidden_source_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_READ | SND_SEQ_PORT_CAP_SUBS_READ | SND_SEQ_PORT_CAP_NO_EXPORT
}

/// Return the hidden port capabilities for one internal destination port.
pub(super) fn hidden_destination_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_WRITE | SND_SEQ_PORT_CAP_SUBS_WRITE | SND_SEQ_PORT_CAP_NO_EXPORT
}

/// Return the visible capabilities for one virtual source port.
pub(super) fn virtual_source_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_READ | SND_SEQ_PORT_CAP_SUBS_READ
}

/// Return the visible capabilities for one virtual destination port.
pub(super) fn virtual_destination_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_WRITE | SND_SEQ_PORT_CAP_SUBS_WRITE
}

/// Return the default ALSA port type for all Destack-managed ports.
pub(super) fn default_port_type() -> u32 {
    SND_SEQ_PORT_TYPE_APPLICATION
}

/// Initialize one outbound event for one local ALSA source port.
pub(super) fn initialize_output_event(event: &mut snd_seq_event_t, local_port_id: i32) {
    *event = unsafe { std::mem::zeroed() };
    event.queue = SND_SEQ_QUEUE_DIRECT;
    event.source.client = 0;
    event.source.port = local_port_id as u8;
    event.dest.client = SND_SEQ_ADDRESS_SUBSCRIBERS;
    event.dest.port = 0;
}

/// Return one ALSA record framing decision and the next SysEx carry state.
pub(super) fn alsa_record_framing(data: &[u8], is_inside_sysex: bool) -> (MidiRecordFraming, bool) {
    let starts_sysex = data.first() == Some(&0xF0);
    let ends_sysex = data.last() == Some(&0xF7);

    // whole sysex records do not carry fragment state forward
    if starts_sysex && ends_sysex {
        return (MidiRecordFraming::Complete, false);
    }

    // one explicit start enters fragment mode
    if starts_sysex {
        return (MidiRecordFraming::Start, true);
    }

    // one trailing terminator closes one fragment chain
    if ends_sysex && is_inside_sysex {
        return (MidiRecordFraming::End, false);
    }

    // records in the middle of one sysex chain inherit continuation framing
    if is_inside_sysex {
        return (MidiRecordFraming::Continue, true);
    }

    (MidiRecordFraming::Complete, false)
}

/// Allocate one resource entry for one input session.
pub(super) fn insert_input_resource(
    binding: &BindingCallContext,
    session: Arc<AlsaInputSession>,
) -> resource::MidiInputPortHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiInputPort)
        .with_label(MIDI_INPUT_RESOURCE_LABEL)
        .with_payload(AlsaInputResource { session });
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::MidiInputPortHandle(resource_id)
}

/// Allocate one resource entry for one output session.
pub(super) fn insert_output_resource(
    binding: &BindingCallContext,
    session: Arc<AlsaOutputSession>,
) -> resource::MidiOutputPortHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiOutputPort)
        .with_label(MIDI_OUTPUT_RESOURCE_LABEL)
        .with_payload(AlsaOutputResource { session });
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::MidiOutputPortHandle(resource_id)
}

/// Allocate one resource entry for one event subscription.
pub(super) fn insert_event_resource(
    binding: &BindingCallContext,
    session: Arc<Mutex<AlsaEventSession>>,
) -> resource::MidiEventHandle {
    let entry = ResourceEntry::new(ResourceKind::MidiEvent)
        .with_label(MIDI_EVENT_RESOURCE_LABEL)
        .with_payload(AlsaEventResource { session });
    let resource_id =
        binding
            .agent()
            .resources
            .insert(binding.world(), entry, Some(binding.engine()));

    resource::MidiEventHandle(resource_id)
}

#[cfg(test)]
mod tests {
    use super::alsa_record_framing;
    use crate::platform::midi::MidiRecordFraming;

    /// Preserve SysEx fragment framing across ALSA reader boundaries.
    #[test]
    fn test_alsa_record_framing_tracks_sysex_fragment_state() {
        let (framing, is_inside_sysex) = alsa_record_framing(&[0xF0, 0x7D, 0x01], false);
        assert_eq!(framing, MidiRecordFraming::Start);
        assert!(is_inside_sysex);

        let (framing, is_inside_sysex) = alsa_record_framing(&[0x02, 0x03], is_inside_sysex);
        assert_eq!(framing, MidiRecordFraming::Continue);
        assert!(is_inside_sysex);

        let (framing, is_inside_sysex) = alsa_record_framing(&[0x04, 0xF7], is_inside_sysex);
        assert_eq!(framing, MidiRecordFraming::End);
        assert!(!is_inside_sysex);

        let (framing, is_inside_sysex) = alsa_record_framing(&[0x90, 0x3C, 0x40], false);
        assert_eq!(framing, MidiRecordFraming::Complete);
        assert!(!is_inside_sysex);
    }
}

/// Decode one runtime-owned string from one native string reference.
pub(super) fn native_string(value: NativeStringRef) -> RuntimeResult<String> {
    let value = unsafe { value.as_str()? };

    Ok(value.to_string())
}

/// Decode one optional runtime-owned string from one native string reference.
pub(super) fn native_optional_string(
    value: Option<NativeStringRef>,
) -> RuntimeResult<Option<String>> {
    match value {
        Some(value) => Ok(Some(native_string(value)?)),
        None => Ok(None),
    }
}
