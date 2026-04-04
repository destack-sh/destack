#![cfg_attr(
    not(any(target_os = "android", target_os = "linux", windows)),
    allow(unused_imports)
)]

pub(crate) mod clock;
pub(crate) mod codec;
pub(crate) mod constants;
pub(crate) mod device;
pub(crate) mod error;
pub(crate) mod event;
pub(crate) mod model;
pub(crate) mod monitor;
pub(crate) mod runtime;
pub(crate) mod stream;

pub(crate) use clock::{clock_now_for_domain, stream_clock_snapshot};
#[allow(unused_imports)]
pub(crate) use codec::{
    all_sample_format_mask, decode_audio_bytes, encode_audio_bytes, frame_bytes, sample_bytes,
};
#[allow(unused_imports)]
pub(crate) use constants::{
    AUDIO_DEVICE_RESOURCE_LABEL, AUDIO_STREAM_RESOURCE_LABEL,
    BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS, BACKEND_CAPABILITY_DEFAULT_ROUTE_EVENTS,
    BACKEND_CAPABILITY_DEVICE_CLOCK, BACKEND_CAPABILITY_EXCLUSIVE_MODE,
    BACKEND_CAPABILITY_HOTPLUG_EVENTS, BACKEND_CAPABILITY_LOOPBACK,
    BACKEND_CAPABILITY_NATIVE_EVENT_FEED, BACKEND_CAPABILITY_NON_INTERLEAVED,
    BACKEND_CAPABILITY_SCHEDULED_WRITE, BACKEND_CAPABILITY_SHARED_MODE,
    DEVICE_CAPABILITY_DEVICE_CLOCK, DEVICE_CAPABILITY_EXCLUSIVE_MODE,
    DEVICE_CAPABILITY_FULL_DUPLEX, DEVICE_CAPABILITY_LOOPBACK, DEVICE_CAPABILITY_REROUTE_EVENTS,
    DEVICE_CAPABILITY_SCHEDULED_WRITE, DEVICE_CAPABILITY_SHARED_MODE,
    DEVICE_CAPABILITY_STREAM_MUTE, DEVICE_CAPABILITY_STREAM_VOLUME, DIRECTION_MASK_CAPTURE,
    DIRECTION_MASK_DUPLEX, DIRECTION_MASK_LOOPBACK, DIRECTION_MASK_PLAYBACK,
    MIN_STREAM_PERIOD_FRAMES, SHARE_MODE_EXCLUSIVE_BIT, SHARE_MODE_SHARED_BIT,
    STREAM_STATUS_INPUT_UNDERFLOW, STREAM_STATUS_OUTPUT_OVERFLOW, host_monotonic_nanos,
};
pub(crate) use device::{
    descriptor_from_device_state, descriptor_from_info, ensure_device_open_flags_supported,
    enumerate_devices_for_request, normalize_device_open_options, null_device,
    stream_device_from_state, supported_backend_device_list_flags,
    supported_backend_device_open_flags, supported_backend_event_subscription_flags,
    supported_backend_stream_clock_domains, supported_backend_stream_flags,
    supported_backend_stream_requirement_flags, supports_device_open_direction,
};
pub(crate) use error::{
    audio_would_block, ensure_capture_direction, ensure_playback_direction,
    ensure_stream_capability, ensure_stream_requirements_satisfied, read_utf8,
    resolve_device_host_state, resolve_stream_host_state, stream_shutdown_error,
    stream_state_is_terminal, validate_stream_config, validate_stream_open_options,
    validate_stream_open_options_for_backend,
};
pub(crate) use event::{publish_stream_event_native, refresh_device_subscriptions_for_rescan};
pub(crate) use model::{AudioDeviceHostState, AudioStreamFinalizer, HostDeviceDescriptor};
pub(crate) use monitor::AudioMonitorHandle;
pub(crate) use runtime::runtime_state;
#[allow(unused_imports)]
pub(crate) use stream::{
    host_stream_flush, host_stream_pause, host_stream_start, host_stream_stop, open_null_stream,
    record_stream_callback_timing, satisfied_stream_requirements, stream_availability_snapshot,
    stream_descriptor, stream_state_snapshot, stream_timing_snapshot,
    wait_for_stream_presentation_time, wait_for_worker_period,
};

#[cfg(windows)]
pub(crate) use codec::{decode_scalar_sample, encode_scalar_sample};

#[cfg(any(target_os = "android", target_os = "linux", windows))]
pub(crate) use constants::{
    AudioBackendOpenFlags, resolved_max_queued_frames, resolved_worker_poll_period,
};

#[cfg(any(test, target_os = "android", target_os = "linux", windows))]
pub(crate) use constants::DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS;

#[cfg(any(test, target_os = "android"))]
pub(crate) use constants::{
    DEVICE_LIST_INCLUDE_DISCONNECTED, DEVICE_OPEN_RAW, STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT,
    STREAM_FLAG_MINIMIZE_LATENCY, STREAM_FLAG_NO_AUTO_CONVERT, STREAM_FLAG_NON_INTERLEAVED,
    STREAM_FLAG_REPORT_XRUN, STREAM_REQUIRE_PAUSE, STREAM_REQUIRE_SCHEDULED_WRITE,
    SUPPORTED_STREAM_CLOCK_CALLBACK, SUPPORTED_STREAM_CLOCK_DEVICE,
    SUPPORTED_STREAM_CLOCK_INPUT_ADC, SUPPORTED_STREAM_CLOCK_MONOTONIC,
    SUPPORTED_STREAM_CLOCK_OUTPUT_DAC, SUPPORTED_STREAM_CLOCK_WALL,
};

#[cfg(test)]
pub(crate) use constants::{
    EVENT_SUBSCRIBE_DEFAULT_ROUTE, EVENT_SUBSCRIBE_DEVICE_HOTPLUG, EVENT_SUBSCRIBE_FORMAT_CHANGE,
    EVENT_SUBSCRIBE_REROUTE, EVENT_SUBSCRIBE_STREAM, MAX_EVENT_POLL_INTERVAL_NS,
    MIN_EVENT_POLL_INTERVAL_NS,
};

#[cfg(target_os = "linux")]
pub(crate) use constants::{BACKEND_OPEN_ALSA_NO_RESAMPLE, BACKEND_OPEN_JACK_NO_AUTOCONNECT};

#[cfg(unix)]
pub(crate) use constants::resolved_max_stream_read_bytes;

#[cfg(windows)]
pub(crate) use constants::{EVENT_POLL_INTERVAL_NS, MAX_STREAM_READ_BYTES};

#[cfg(any(target_os = "android", target_os = "linux", windows))]
pub(crate) use constants::{STREAM_STATUS_INPUT_OVERFLOW, STREAM_STATUS_OUTPUT_UNDERFLOW};

#[cfg(windows)]
pub(crate) use error::audio_not_found;

#[cfg(any(target_os = "android", target_os = "linux", windows))]
#[allow(unused_imports)]
pub(crate) use model::{
    AudioHostStreamOps, AudioStreamHostState, AudioStreamRuntimeCapabilities,
    AudioStreamStateInner, AudioStreamSync, initial_stream_state,
};

#[cfg(any(
    all(target_os = "android", feature = "audio-aaudio"),
    all(target_os = "android", feature = "audio-opensles"),
    all(target_os = "linux", feature = "audio-pipewire"),
    all(target_os = "linux", feature = "audio-pulseaudio"),
))]
pub(crate) use stream::mark_stream_backend_disconnected;

#[cfg(target_os = "linux")]
pub(crate) use constants::resolved_event_monitor_poll_interval_ns;

#[cfg(any(target_os = "linux", windows))]
pub(crate) use event::publish_device_snapshot_native_if_service_live;
