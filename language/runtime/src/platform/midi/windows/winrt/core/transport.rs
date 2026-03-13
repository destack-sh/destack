use crate::platform::midi::{
    MIDI_DATA_FORMAT_FLAG_MIDI1_BYTES, MIDI_PROTOCOL_FLAG_MIDI1, MidiDataFormat,
    MidiDataFormatFlags, MidiProtocol, MidiProtocolFlags,
};

/// Return one exact transport-support tuple for the WinRT backend.
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

/// Convert one WinRT port-relative timestamp into runtime monotonic nanoseconds.
///
/// WinRT MIDI receive timestamps are reported as `TimeSpan` durations relative to
/// the opened `MidiInPort`, so the session records one monotonic open epoch and
/// projects each callback timestamp into the shared runtime monotonic domain.
pub(crate) fn winrt_relative_timestamp_to_mono_ns(open_epoch_ns: u64, duration_100ns: i64) -> u64 {
    if duration_100ns <= 0 {
        return open_epoch_ns;
    }

    let delta_ns = (duration_100ns as u128).saturating_mul(100);
    let delta_ns = delta_ns.min(u64::MAX as u128) as u64;

    open_epoch_ns.saturating_add(delta_ns)
}

#[cfg(test)]
mod tests {
    use super::winrt_relative_timestamp_to_mono_ns;

    /// Project WinRT relative timestamps from the opened-port epoch.
    #[test]
    fn test_winrt_relative_timestamp_to_mono_ns_uses_the_open_epoch() {
        let received_at_ns = winrt_relative_timestamp_to_mono_ns(2_000_000_000, 15_000);

        assert_eq!(received_at_ns, 2_001_500_000);
    }

    /// Clamp non-positive WinRT timestamps to the opened-port epoch.
    #[test]
    fn test_winrt_relative_timestamp_to_mono_ns_clamps_non_positive_offsets() {
        assert_eq!(winrt_relative_timestamp_to_mono_ns(123, 0), 123);
        assert_eq!(winrt_relative_timestamp_to_mono_ns(456, -1), 456);
    }

    /// Saturate large WinRT relative timestamps instead of wrapping.
    #[test]
    fn test_winrt_relative_timestamp_to_mono_ns_saturates_large_offsets() {
        let received_at_ns = winrt_relative_timestamp_to_mono_ns(u64::MAX - 10, i64::MAX);

        assert_eq!(received_at_ns, u64::MAX);
    }
}
