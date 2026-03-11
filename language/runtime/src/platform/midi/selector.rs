use crate::platform::midi::MidiBackend;

/// Selector rows advertised by the platform MIDI surface.
pub(crate) const MIDI_SELECTOR_ROWS: [MidiBackend; 7] = [
    MidiBackend::Auto,
    MidiBackend::AlsaSequencer,
    MidiBackend::JackMidi,
    MidiBackend::CoreMIDI,
    MidiBackend::WinMM,
    MidiBackend::WinRT,
    MidiBackend::Null,
];

/// Return one stable label for one MIDI backend selector.
pub(crate) fn midi_backend_name(backend: MidiBackend) -> &'static str {
    match backend {
        MidiBackend::Auto => "auto",
        MidiBackend::AlsaSequencer => "alsa-sequencer",
        MidiBackend::JackMidi => "jack-midi",
        MidiBackend::CoreMIDI => "coremidi",
        MidiBackend::WinMM => "winmm",
        MidiBackend::WinRT => "winrt",
        MidiBackend::Null => "null",
    }
}
