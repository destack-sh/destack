mod clock;
mod codec;
mod constants;
mod device;
mod error;
mod event;
mod model;
mod monitor;
mod runtime;
mod stream;

pub(crate) use clock::{clock_now_for_domain, stream_clock_snapshot};
pub(crate) use codec::{
    all_sample_format_mask, decode_audio_bytes, encode_audio_bytes, frame_bytes, sample_bytes,
};
#[cfg(any(target_os = "android", target_os = "linux"))]
pub(crate) use codec::{clamp_audio_scalar, sample_format_bit};
#[cfg(any(target_os = "macos", windows))]
pub(crate) use codec::{decode_scalar_sample, encode_scalar_sample};
#[cfg(any(
    all(windows, feature = "audio-wasapi"),
    all(windows, feature = "audio-asio"),
    all(target_os = "macos", feature = "audio-coreaudio"),
    all(target_os = "linux", feature = "audio-alsa"),
    all(target_os = "android", feature = "audio-aaudio"),
))]
pub(crate) use constants::DEVICE_CAPABILITY_EXCLUSIVE_MODE;
#[cfg(any(target_os = "linux", windows))]
pub(crate) use constants::resolved_event_monitor_poll_interval_ns;
pub(crate) use constants::{
    AUDIO_DEVICE_RESOURCE_LABEL, AUDIO_EVENT_RESOURCE_LABEL, AUDIO_STREAM_RESOURCE_LABEL,
    AudioBackendOpenFlags, BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS,
    BACKEND_CAPABILITY_DEFAULT_ROUTE_EVENTS, BACKEND_CAPABILITY_DEVICE_CLOCK,
    BACKEND_CAPABILITY_EXCLUSIVE_MODE, BACKEND_CAPABILITY_HOTPLUG_EVENTS,
    BACKEND_CAPABILITY_LOOPBACK, BACKEND_CAPABILITY_NON_INTERLEAVED,
    BACKEND_CAPABILITY_SCHEDULED_WRITE, BACKEND_CAPABILITY_SHARED_MODE,
    BACKEND_OPEN_ALSA_NO_RESAMPLE, BACKEND_OPEN_JACK_NO_AUTOCONNECT, DEFAULT_STREAM_VOLUME,
    DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS, DEVICE_CAPABILITY_BIT_EXACT_PCM,
    DEVICE_CAPABILITY_DEVICE_CLOCK, DEVICE_CAPABILITY_FULL_DUPLEX,
    DEVICE_CAPABILITY_INTERRUPTION_EVENTS, DEVICE_CAPABILITY_LOOPBACK,
    DEVICE_CAPABILITY_REROUTE_EVENTS, DEVICE_CAPABILITY_SCHEDULED_WRITE,
    DEVICE_CAPABILITY_SHARED_MODE, DEVICE_CAPABILITY_STREAM_MUTE, DEVICE_CAPABILITY_STREAM_VOLUME,
    DEVICE_LIST_INCLUDE_DISCONNECTED, DEVICE_LIST_INCLUDE_LOOPBACK, DEVICE_LIST_INCLUDE_RAW,
    DEVICE_OPEN_FOLLOW_DEFAULT_ROUTE, DEVICE_OPEN_LOW_LATENCY, DEVICE_OPEN_RAW,
    DEVICE_OPEN_REALTIME_THREAD, DIRECTION_MASK_CAPTURE, DIRECTION_MASK_DUPLEX,
    DIRECTION_MASK_LOOPBACK, DIRECTION_MASK_PLAYBACK, EVENT_SUBSCRIBE_BACKEND,
    EVENT_SUBSCRIBE_DEFAULT_ROUTE, EVENT_SUBSCRIBE_DEVICE_HOTPLUG, EVENT_SUBSCRIBE_FORMAT_CHANGE,
    EVENT_SUBSCRIBE_INTERRUPTION, EVENT_SUBSCRIBE_REROUTE, EVENT_SUBSCRIBE_STREAM,
    KNOWN_DEVICE_LIST_FLAGS_MASK, KNOWN_DEVICE_OPEN_FLAGS_MASK, KNOWN_STREAM_FLAGS_MASK,
    KNOWN_STREAM_REQUIREMENT_FLAGS_MASK, KNOWN_SUPPORTED_EVENT_SUBSCRIPTION_FLAGS_MASK,
    KNOWN_SUPPORTED_STREAM_FLAGS_MASK, MAX_EVENT_POLL_INTERVAL_NS, MIN_EVENT_POLL_INTERVAL_NS,
    MIN_STREAM_PERIOD_FRAMES, NULL_DEVICE_MAX_PERIOD_FRAMES, NULL_DEVICE_MAX_SAMPLE_RATE,
    NULL_DEVICE_MIN_SAMPLE_RATE, NULL_DEVICE_PREFERRED_CHANNEL_MASK,
    NULL_DEVICE_PREFERRED_PERIOD_FRAMES, NULL_DEVICE_PREFERRED_SAMPLE_RATE,
    NULL_DEVICE_SUPPORTED_CHANNEL_MASK, SHARE_MODE_EXCLUSIVE_BIT, SHARE_MODE_SHARED_BIT,
    STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT, STREAM_FLAG_MINIMIZE_LATENCY, STREAM_FLAG_NEVER_DROP_INPUT,
    STREAM_FLAG_NO_AUTO_CONVERT, STREAM_FLAG_NON_INTERLEAVED, STREAM_FLAG_PRIME_OUTPUT_BUFFERS,
    STREAM_FLAG_REPORT_XRUN, STREAM_FLAG_SCHEDULE_REALTIME, STREAM_REQUIRE_BIT_EXACT_PCM,
    STREAM_REQUIRE_HARDWARE_TIMESTAMPS, STREAM_REQUIRE_NON_INTERLEAVED, STREAM_REQUIRE_PAUSE,
    STREAM_REQUIRE_SCHEDULED_WRITE, STREAM_STATUS_INPUT_OVERFLOW, STREAM_STATUS_INPUT_UNDERFLOW,
    STREAM_STATUS_OUTPUT_OVERFLOW, STREAM_STATUS_OUTPUT_UNDERFLOW,
    SUPPORTED_EVENT_SUBSCRIPTION_DEFAULT_ROUTE, SUPPORTED_STREAM_CLOCK_CALLBACK,
    SUPPORTED_STREAM_CLOCK_DEVICE, SUPPORTED_STREAM_CLOCK_INPUT_ADC,
    SUPPORTED_STREAM_CLOCK_MONOTONIC, SUPPORTED_STREAM_CLOCK_OUTPUT_DAC,
    SUPPORTED_STREAM_CLOCK_WALL, SUPPORTED_STREAM_FLAG_NON_INTERLEAVED,
    SUPPORTED_STREAM_REQUIREMENT_BIT_EXACT_PCM, SUPPORTED_STREAM_REQUIREMENT_HARDWARE_TIMESTAMPS,
    SUPPORTED_STREAM_REQUIREMENT_NON_INTERLEAVED, SUPPORTED_STREAM_REQUIREMENT_PAUSE,
    SUPPORTED_STREAM_REQUIREMENT_SCHEDULED_WRITE, host_monotonic_nanos,
    resolved_default_event_poll_interval_ns, resolved_default_event_queue_capacity,
    resolved_max_queued_frames, resolved_max_stream_read_bytes, resolved_stream_wait_slice_ns,
    resolved_worker_poll_period,
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
    audio_busy, audio_would_block, ensure_capture_direction, ensure_playback_direction,
    ensure_stream_capability, ensure_stream_requirements_satisfied, read_utf8,
    resolve_device_host_state, resolve_event_stream, resolve_stream_host_state,
    stream_shutdown_error, stream_state_is_terminal, validate_stream_config,
    validate_stream_open_options, validate_stream_open_options_for_backend,
};
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
pub(crate) use model::AudioHostStreamOps;
pub(crate) use model::{
    AudioDeviceHostState, AudioDeviceMonitorBaseline, AudioEventRecord, AudioEventStream,
    AudioEventStreamState, AudioStreamHostState, AudioStreamMonitorBaseline,
    AudioStreamRuntimeCapabilities, AudioStreamStateInner, AudioStreamSync, HostDeviceDescriptor,
    audio_device_monitor_baseline, audio_stream_monitor_baseline, initial_audio_event_stream_state,
    initial_stream_state,
};
pub(crate) use monitor::{AudioMonitorHandle, AudioMonitorServiceRegistry};
pub(crate) use runtime::{
    AudioRuntimeState, KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK, STREAM_EVENT_SUBSCRIPTION_FLAGS_MASK,
    event_subscription_enabled, native_only_supported_subscription_flags, runtime_state,
    stream_accepts_record, tracks_device_events,
};
#[cfg(any(
    all(target_os = "android", feature = "audio-aaudio"),
    all(target_os = "android", feature = "audio-opensles"),
    all(target_os = "linux", feature = "audio-pipewire"),
    all(target_os = "linux", feature = "audio-pulseaudio"),
))]
pub(crate) use stream::mark_stream_backend_disconnected;
#[cfg(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    windows
))]
pub(crate) use stream::record_stream_callback_timing;
pub(crate) use stream::{
    host_stream_flush, host_stream_pause, host_stream_start, host_stream_stop, open_null_stream,
    satisfied_stream_requirements, stream_availability_snapshot, stream_descriptor,
    stream_state_snapshot, stream_timing_snapshot,
};

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
pub(crate) use event::publish::publish_device_snapshot_native;
pub(crate) use event::publish::{
    publish_stream_event_native, refresh_device_subscriptions_for_rescan,
};
pub(crate) use event::stream::{
    close_event_stream, open_event_stream, read_event, read_event_batch, try_read_event,
    try_read_event_batch,
};
