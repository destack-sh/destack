use crate::platform::device::{
    MIDI_DATA_FORMAT_FLAG_UMP, MIDI_PROTOCOL_FLAG_MIDI1, MIDI_PROTOCOL_FLAG_MIDI2, MidiDataFormat,
    MidiDataFormatFlags, MidiProtocol, MidiProtocolFlags,
};

/// Return one exact transport-support tuple for one protocol set.
pub(crate) fn exact_transport_support(
    supported_protocols: MidiProtocolFlags,
) -> (
    MidiDataFormatFlags,
    Option<MidiDataFormat>,
    MidiProtocolFlags,
    Option<MidiProtocol>,
) {
    let default_protocol = if supported_protocols.0 & MIDI_PROTOCOL_FLAG_MIDI2.0 != 0 {
        Some(MidiProtocol::Midi2)
    } else if supported_protocols.0 & MIDI_PROTOCOL_FLAG_MIDI1.0 != 0 {
        Some(MidiProtocol::Midi1)
    } else {
        None
    };

    let supported_data_formats = if supported_protocols.0 == 0 {
        MidiDataFormatFlags(0)
    } else {
        MidiDataFormatFlags(MIDI_DATA_FORMAT_FLAG_UMP.0)
    };
    let default_data_format = if supported_protocols.0 == 0 {
        None
    } else {
        Some(MidiDataFormat::Ump)
    };

    (
        supported_data_formats,
        default_data_format,
        supported_protocols,
        default_protocol,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prefer MIDI 2 semantics when the Windows endpoint advertises both protocol generations.
    #[test]
    fn test_exact_transport_support_prefers_midi2_when_available() {
        let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
            exact_transport_support(MidiProtocolFlags(
                MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0,
            ));

        assert_eq!(supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_UMP.0);
        assert_eq!(default_data_format, Some(MidiDataFormat::Ump));
        assert_eq!(
            supported_protocols.0,
            MIDI_PROTOCOL_FLAG_MIDI1.0 | MIDI_PROTOCOL_FLAG_MIDI2.0,
        );
        assert_eq!(default_protocol, Some(MidiProtocol::Midi2));
    }

    /// Preserve MIDI 1 defaults when the Windows endpoint advertises only MIDI 1 semantics.
    #[test]
    fn test_exact_transport_support_preserves_midi1_defaults() {
        let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
            exact_transport_support(MidiProtocolFlags(MIDI_PROTOCOL_FLAG_MIDI1.0));

        assert_eq!(supported_data_formats.0, MIDI_DATA_FORMAT_FLAG_UMP.0);
        assert_eq!(default_data_format, Some(MidiDataFormat::Ump));
        assert_eq!(supported_protocols.0, MIDI_PROTOCOL_FLAG_MIDI1.0);
        assert_eq!(default_protocol, Some(MidiProtocol::Midi1));
    }

    /// Keep empty protocol rows fully zeroed for unsupported endpoints.
    #[test]
    fn test_exact_transport_support_zeroes_empty_protocol_rows() {
        let (supported_data_formats, default_data_format, supported_protocols, default_protocol) =
            exact_transport_support(MidiProtocolFlags(0));

        assert_eq!(supported_data_formats.0, 0);
        assert_eq!(default_data_format, None);
        assert_eq!(supported_protocols.0, 0);
        assert_eq!(default_protocol, None);
    }
}
