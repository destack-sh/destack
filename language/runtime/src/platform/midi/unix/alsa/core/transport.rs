use crate::diagnostic::RuntimeResult;
use crate::platform::core::invalid_argument;
use crate::platform::midi::core::endpoint_direction_name;
use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1, MidiDataFormat,
    MidiDataFormatFlags, MidiPortDirection, MidiProtocol, MidiProtocolFlags, MidiRecordFraming,
};

use super::super::abi::{
    SND_SEQ_ADDRESS_SUBSCRIBERS, SND_SEQ_PORT_CAP_NO_EXPORT, SND_SEQ_PORT_CAP_READ,
    SND_SEQ_PORT_CAP_SUBS_READ, SND_SEQ_PORT_CAP_SUBS_WRITE, SND_SEQ_PORT_CAP_WRITE,
    SND_SEQ_PORT_TYPE_APPLICATION, SND_SEQ_QUEUE_DIRECT, SND_SEQ_TIME_MODE_REL,
    SND_SEQ_TIME_STAMP_REAL, snd_seq_event_t, snd_seq_real_time_t,
};
use super::session::INTERNAL_CLIENT_NAME_PREFIX;

/// Return the exact transport support advertised by ALSA sequencer endpoints.
pub(crate) fn exact_transport_support() -> (
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
pub(crate) fn group_id(client_id: i32) -> String {
    format!("alsa:client:{client_id}")
}

/// Return one stable runtime id for one ALSA endpoint.
pub(crate) fn runtime_id(direction: MidiPortDirection, client_id: i32, port_id: i32) -> String {
    let direction_name = endpoint_direction_name(direction);

    format!("alsa:{direction_name}:{client_id}:{port_id}")
}

/// Return one stable backend id for one ALSA endpoint.
pub(crate) fn backend_id(client_id: i32, port_id: i32) -> String {
    format!("{client_id}:{port_id}")
}

/// Parse one stable ALSA backend id.
pub(crate) fn parse_backend_id(
    backend_id: &str,
    operation: &'static str,
) -> RuntimeResult<(i32, i32)> {
    let Some((client_id, port_id)) = backend_id.split_once(':') else {
        return Err(invalid_argument(
            "backendId",
            format!("{operation}: malformed ALSA backend id"),
        ));
    };

    let client_id = client_id.parse::<i32>().map_err(|_| {
        invalid_argument("backendId", format!("{operation}: invalid ALSA client id"))
    })?;
    let port_id = port_id
        .parse::<i32>()
        .map_err(|_| invalid_argument("backendId", format!("{operation}: invalid ALSA port id")))?;

    Ok((client_id, port_id))
}

/// Return whether one port should be exposed as one input source.
pub(crate) fn is_input_source(capability: u32) -> bool {
    let required = SND_SEQ_PORT_CAP_READ | SND_SEQ_PORT_CAP_SUBS_READ;

    capability & required == required
}

/// Return whether one port should be exposed as one output destination.
pub(crate) fn is_output_destination(capability: u32) -> bool {
    let required = SND_SEQ_PORT_CAP_WRITE | SND_SEQ_PORT_CAP_SUBS_WRITE;

    capability & required == required
}

/// Return whether one port looks virtual.
pub(crate) fn is_virtual_port(port_type: u32) -> bool {
    port_type & SND_SEQ_PORT_TYPE_APPLICATION != 0
}

/// Return whether one client name belongs to Destack internal plumbing.
pub(crate) fn is_internal_client_name(name: &str) -> bool {
    name.starts_with(INTERNAL_CLIENT_NAME_PREFIX)
}

/// Return the hidden port capabilities for one internal source port.
pub(crate) fn hidden_source_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_READ | SND_SEQ_PORT_CAP_SUBS_READ | SND_SEQ_PORT_CAP_NO_EXPORT
}

/// Return the hidden port capabilities for one internal destination port.
pub(crate) fn hidden_destination_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_WRITE | SND_SEQ_PORT_CAP_SUBS_WRITE | SND_SEQ_PORT_CAP_NO_EXPORT
}

/// Return the visible capabilities for one virtual source port.
pub(crate) fn virtual_source_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_READ | SND_SEQ_PORT_CAP_SUBS_READ
}

/// Return the visible capabilities for one virtual destination port.
pub(crate) fn virtual_destination_port_capability() -> u32 {
    SND_SEQ_PORT_CAP_WRITE | SND_SEQ_PORT_CAP_SUBS_WRITE
}

/// Return the default ALSA port type for all Destack-managed ports.
pub(crate) fn default_port_type() -> u32 {
    SND_SEQ_PORT_TYPE_APPLICATION
}

/// Initialize one outbound event for one local ALSA source port.
pub(crate) fn initialize_output_event(event: &mut snd_seq_event_t, local_port_id: i32) {
    *event = unsafe { std::mem::zeroed() };
    event.queue = SND_SEQ_QUEUE_DIRECT;
    event.source.client = 0;
    event.source.port = local_port_id as u8;
    event.dest.client = SND_SEQ_ADDRESS_SUBSCRIBERS;
    event.dest.port = 0;
}

/// Schedule one outbound ALSA event relative to the session queue clock.
pub(crate) fn schedule_output_event(event: &mut snd_seq_event_t, queue_id: i32, delay_ns: u64) {
    event.queue = queue_id as u8;
    event.flags |= SND_SEQ_TIME_STAMP_REAL | SND_SEQ_TIME_MODE_REL;
    event.time.timestamp.time = alsa_real_time_from_ns(delay_ns);
}

/// Return one receive timestamp from one ALSA event when realtime timing is available.
pub(crate) fn input_event_received_at_ns(
    event: &snd_seq_event_t,
    queue_start_ns: u64,
) -> Option<u64> {
    if event.queue == SND_SEQ_QUEUE_DIRECT {
        return None;
    }

    if event.flags & SND_SEQ_TIME_STAMP_REAL == 0 {
        return None;
    }

    let timestamp = unsafe { event.time.timestamp.time };
    let offset_ns = alsa_real_time_to_ns(timestamp);

    Some(queue_start_ns.saturating_add(offset_ns))
}

/// Return one ALSA record framing decision and the next SysEx carry state.
pub(crate) fn alsa_record_framing(data: &[u8], is_inside_sysex: bool) -> (MidiRecordFraming, bool) {
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

/// Return one ALSA realtime payload from one nanosecond offset.
fn alsa_real_time_from_ns(offset_ns: u64) -> snd_seq_real_time_t {
    let seconds = offset_ns / 1_000_000_000;
    let nanos = offset_ns % 1_000_000_000;

    snd_seq_real_time_t {
        tv_sec: seconds.min(u32::MAX as u64) as u32,
        tv_nsec: nanos as u32,
    }
}

/// Return one nanosecond offset from one ALSA realtime payload.
fn alsa_real_time_to_ns(timestamp: snd_seq_real_time_t) -> u64 {
    let seconds_ns = (timestamp.tv_sec as u64).saturating_mul(1_000_000_000);
    let nanos = (timestamp.tv_nsec as u64).min(999_999_999);

    seconds_ns.saturating_add(nanos)
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
    }
}
