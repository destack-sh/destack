#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use crate::platform::midi::MidiBackend;

/// Selector rows advertised by the platform MIDI surface.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) const MIDI_SELECTOR_ROWS: [MidiBackend; 7] = [
    MidiBackend::Auto,
    MidiBackend::Alsa,
    MidiBackend::JackMidi,
    MidiBackend::CoreMIDI,
    MidiBackend::WinMM,
    MidiBackend::WinRT,
    MidiBackend::Null,
];

/// Return one stable label for one MIDI backend selector.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) fn midi_backend_name(backend: MidiBackend) -> &'static str {
    match backend {
        MidiBackend::Auto => "auto",
        MidiBackend::Alsa => "alsa",
        MidiBackend::JackMidi => "jack-midi",
        MidiBackend::CoreMIDI => "coremidi",
        MidiBackend::WinMM => "winmm",
        MidiBackend::WinRT => "winrt",
        MidiBackend::Null => "null",
    }
}
