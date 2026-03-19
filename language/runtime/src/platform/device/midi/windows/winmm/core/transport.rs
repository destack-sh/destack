use crate::platform::device::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1, MidiDataFormat,
    MidiDataFormatFlags, MidiProtocol, MidiProtocolFlags,
};

/// Return one exact transport-support tuple for the WinMM backend.
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

/// Convert one WinMM callback timestamp in milliseconds into runtime monotonic nanoseconds.
pub(crate) fn winmm_timestamp_to_mono_ns(start_epoch_ns: u64, timestamp_ms: u32) -> u64 {
    let delta_ns = u64::from(timestamp_ms).saturating_mul(1_000_000);

    start_epoch_ns.saturating_add(delta_ns)
}

/// Build one runtime input id for one WinMM device index.
pub(crate) fn input_runtime_id(device_id: u32) -> String {
    format!("winmm:input:{device_id}")
}

/// Build one runtime output id for one WinMM device index.
pub(crate) fn output_runtime_id(device_id: u32) -> String {
    format!("winmm:output:{device_id}")
}

#[cfg(test)]
mod tests {
    use super::winmm_timestamp_to_mono_ns;

    /// Project WinMM callback timestamps from the session start epoch.
    #[test]
    fn test_winmm_timestamp_to_mono_ns_uses_session_start_epoch() {
        let start_epoch_ns = 2_000_000_000;
        let received_at_ns = winmm_timestamp_to_mono_ns(start_epoch_ns, 1500);

        assert_eq!(received_at_ns, 3_500_000_000);
    }

    /// Saturate large WinMM callback timestamps instead of wrapping.
    #[test]
    fn test_winmm_timestamp_to_mono_ns_saturates_on_large_offsets() {
        let received_at_ns = winmm_timestamp_to_mono_ns(u64::MAX - 10, 1);

        assert_eq!(received_at_ns, u64::MAX);
    }
}
