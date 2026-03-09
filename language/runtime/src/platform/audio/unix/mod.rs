#[cfg(all(target_os = "android", feature = "audio-aaudio"))]
mod aaudio;
#[cfg(all(target_os = "linux", feature = "audio-alsa"))]
mod alsa;
mod backend;
mod clock;
mod core;
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[cfg(feature = "audio-coreaudio")]
mod coreaudio;
mod device;
mod event;
#[cfg(all(target_os = "linux", feature = "audio-jack"))]
mod jack;
#[cfg(all(target_os = "android", feature = "audio-opensles"))]
mod opensles;
#[cfg(all(
    target_os = "linux",
    any(feature = "audio-pipewire", feature = "audio-pulseaudio"),
))]
mod pactl;
#[cfg(all(target_os = "linux", feature = "audio-pipewire"))]
mod pipewire;
#[cfg(all(target_os = "linux", feature = "audio-pulseaudio"))]
mod pulseaudio;
mod stream;

pub(crate) use clock::{destack_audio_clock_now, destack_audio_stream_clock};
pub(crate) use core::{
    backend_stream_supported, backend_supported, backend_supports_native_device_monitor,
    enumerate_host_devices, preferred_host_backends, start_backend_native_device_events_impl,
};
pub(crate) use device::{
    destack_audio_backend_list, destack_audio_device_close, destack_audio_device_default,
    destack_audio_device_descriptor, destack_audio_device_list, destack_audio_device_open,
    destack_audio_device_rescan,
};
pub(crate) use event::{
    destack_audio_event_close, destack_audio_event_open, destack_audio_event_read,
    destack_audio_event_read_batch, destack_audio_event_try_read,
    destack_audio_event_try_read_batch,
};
pub(crate) use stream::{
    destack_audio_stream_abort, destack_audio_stream_availability, destack_audio_stream_close,
    destack_audio_stream_descriptor, destack_audio_stream_drain, destack_audio_stream_flush,
    destack_audio_stream_open, destack_audio_stream_pause, destack_audio_stream_read,
    destack_audio_stream_readv, destack_audio_stream_set_mute, destack_audio_stream_set_name,
    destack_audio_stream_set_volume, destack_audio_stream_start, destack_audio_stream_state,
    destack_audio_stream_stop, destack_audio_stream_support, destack_audio_stream_timing,
    destack_audio_stream_try_read, destack_audio_stream_try_readv, destack_audio_stream_try_write,
    destack_audio_stream_try_writev, destack_audio_stream_write, destack_audio_stream_write_at,
    destack_audio_stream_write_atv, destack_audio_stream_writev,
};
