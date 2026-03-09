#[cfg(unix)]
pub(crate) use super::unix::*;

#[cfg(windows)]
pub(crate) use super::windows::*;

#[cfg(not(any(unix, windows)))]
pub(crate) use super::unsupported::*;

pub(crate) use super::backend::{
    destack_audio_midi_flush, destack_audio_midi_port_close, destack_audio_midi_port_list,
    destack_audio_midi_port_open, destack_audio_midi_read, destack_audio_midi_try_read,
    destack_audio_midi_write,
};
