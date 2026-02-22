use std::ffi::c_ulong;

/// Prefix for one JACK playback stable id.
pub(super) const JACK_PLAYBACK_STABLE_ID_PREFIX: &str = "jack:playback:";
/// Prefix for one JACK capture stable id.
pub(super) const JACK_CAPTURE_STABLE_ID_PREFIX: &str = "jack:capture:";
/// Prefix for one JACK duplex stable id.
pub(super) const JACK_DUPLEX_STABLE_ID_PREFIX: &str = "jack:duplex:";

/// JACK option: do not start the server when unavailable.
pub(super) const JACK_OPTION_NO_START_SERVER: u32 = 0x01;
/// JACK port flag: input port.
pub(super) const JACK_PORT_IS_INPUT: c_ulong = 0x1;
/// JACK port flag: output port.
pub(super) const JACK_PORT_IS_OUTPUT: c_ulong = 0x2;
/// JACK port flag: physical endpoint.
pub(super) const JACK_PORT_IS_PHYSICAL: c_ulong = 0x4;
/// JACK port flag: terminal endpoint.
pub(super) const JACK_PORT_IS_TERMINAL: c_ulong = 0x8;
/// JACK audio port type name.
pub(super) const JACK_AUDIO_TYPE: &[u8] = b"32 bit float mono audio\0";
