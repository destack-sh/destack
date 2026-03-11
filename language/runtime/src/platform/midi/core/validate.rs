use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::midi::{MidiDataFormat, MidiProtocol};

/// Validate one format and protocol pairing.
#[cfg_attr(not(any(target_os = "macos", windows)), allow(dead_code))]
pub(crate) fn validate_record_shape(
    operation: &'static str,
    data_format: MidiDataFormat,
    protocol: Option<MidiProtocol>,
) -> RuntimeResult<()> {
    // reject midi2 semantics on byte-stream transport
    if matches!(protocol, Some(MidiProtocol::Midi2)) && data_format != MidiDataFormat::Ump {
        return Err(
            RuntimeError::from(crate::platform::PlatformError::invalid_argument_value(
                "protocol",
                format!("{operation}: MIDI 2 requires UMP transport"),
            ))
            .boxed(),
        );
    }

    Ok(())
}
