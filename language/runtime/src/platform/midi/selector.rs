#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
use crate::platform::midi::MidiBackend;

/// Selector rows advertised by the platform MIDI surface.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
pub(crate) const MIDI_SELECTOR_ROWS: [MidiBackend; 9] = [
    MidiBackend::Auto,
    MidiBackend::Alsa,
    MidiBackend::JackMidi,
    MidiBackend::CoreMIDI,
    MidiBackend::WindowsMidi,
    MidiBackend::WinMM,
    MidiBackend::WinRT,
    MidiBackend::AndroidMidi,
    MidiBackend::Null,
];

/// Return one stable label for one MIDI backend selector.
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "ios",
    windows
))]
pub(crate) fn midi_backend_name(backend: MidiBackend) -> &'static str {
    match backend {
        MidiBackend::Auto => "auto",
        MidiBackend::Alsa => "alsa",
        MidiBackend::JackMidi => "jack-midi",
        MidiBackend::CoreMIDI => "coremidi",
        MidiBackend::WindowsMidi => "windows-midi",
        MidiBackend::WinMM => "winmm",
        MidiBackend::WinRT => "winrt",
        MidiBackend::AndroidMidi => "android-midi",
        MidiBackend::Null => "null",
    }
}
