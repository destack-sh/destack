use std::time::Duration;

use crate::platform::audio::{
    AudioBackendCapabilityFlags, AudioDeviceCapabilityFlags, AudioDeviceListFlags,
    AudioDeviceOpenFlags, AudioEventSubscriptionFlags, AudioStreamFlags,
    AudioStreamRequirementFlags, AudioStreamStatusFlags, AudioSupportedEventSubscriptionFlags,
    AudioSupportedStreamClockDomains, AudioSupportedStreamFlags,
    AudioSupportedStreamRequirementFlags,
};
use crate::platform::core as core_platform;
use crate::runtime::{BindingCallContext, with_binding_call_context};

/// Compatibility alias for backend-specific stream open flags.
pub(crate) type AudioBackendOpenFlags = AudioDeviceOpenFlags;

/// Resource label for audio device handles.
pub(crate) const AUDIO_DEVICE_RESOURCE_LABEL: &str = "audio.device";
/// Resource label for audio stream handles.
pub(crate) const AUDIO_STREAM_RESOURCE_LABEL: &str = "audio.stream";
/// Resource label for audio event handles.
pub(crate) const AUDIO_EVENT_RESOURCE_LABEL: &str = "audio.event";

/// Hard maximum read payload bytes accepted by one call.
pub(crate) const MAX_STREAM_READ_BYTES: u32 = 8 * 1024 * 1024;
/// Hard maximum queued stream latency budget in frames.
pub(crate) const MAX_QUEUED_FRAMES: usize = 96_000;
/// Default queue capacity for one event subscription.
pub(crate) const DEFAULT_EVENT_QUEUE_CAPACITY: u32 = 1024;
/// Default poll interval for event reads.
pub(crate) const EVENT_POLL_INTERVAL_NS: u64 = 5_000_000;
/// Minimum event poll interval accepted from one subscription request.
pub(crate) const MIN_EVENT_POLL_INTERVAL_NS: u64 = 100_000;
/// Maximum event poll interval accepted from one subscription request.
pub(crate) const MAX_EVENT_POLL_INTERVAL_NS: u64 = 1_000_000_000;
/// Minimum worker poll interval for backend stream worker loops.
pub(crate) const MIN_WORKER_POLL_INTERVAL_NS: u64 = 1_000_000;
/// Minimum stream period in frames.
pub(crate) const MIN_STREAM_PERIOD_FRAMES: u32 = 64;

/// Shared-mode bit in one device share-mode mask.
pub(crate) const SHARE_MODE_SHARED_BIT: u32 = 1u32 << 0;
/// Exclusive-mode bit in one device share-mode mask.
pub(crate) const SHARE_MODE_EXCLUSIVE_BIT: u32 = 1u32 << 1;

/// Include disconnected or unavailable endpoints when possible.
pub(crate) const DEVICE_LIST_INCLUDE_DISCONNECTED: AudioDeviceListFlags = AudioDeviceListFlags(0x1);
/// Include raw endpoints when the backend exposes separate raw rows.
pub(crate) const DEVICE_LIST_INCLUDE_RAW: AudioDeviceListFlags = AudioDeviceListFlags(0x2);
/// Include loopback endpoints when the backend exposes loopback rows.
pub(crate) const DEVICE_LIST_INCLUDE_LOOPBACK: AudioDeviceListFlags = AudioDeviceListFlags(0x4);
/// Include duplicate aliases when backend identity is ambiguous.
pub(crate) const DEVICE_LIST_INCLUDE_DUPLICATES: AudioDeviceListFlags = AudioDeviceListFlags(0x8);
/// Mask for all known device-list option bits.
pub(crate) const KNOWN_DEVICE_LIST_FLAGS_MASK: u32 = DEVICE_LIST_INCLUDE_DISCONNECTED.0
    | DEVICE_LIST_INCLUDE_RAW.0
    | DEVICE_LIST_INCLUDE_LOOPBACK.0
    | DEVICE_LIST_INCLUDE_DUPLICATES.0;

/// Request raw endpoint mode where available.
pub(crate) const DEVICE_OPEN_RAW: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x8);
/// Request ALSA no-resample mode when the backend supports it.
pub(crate) const BACKEND_OPEN_ALSA_NO_RESAMPLE: AudioBackendOpenFlags =
    AudioDeviceOpenFlags(0x1_0000);
/// Request JACK manual-connection mode when the backend supports it.
pub(crate) const BACKEND_OPEN_JACK_NO_AUTOCONNECT: AudioBackendOpenFlags =
    AudioDeviceOpenFlags(0x2_0000);
/// Mask for all known device-open option bits.
pub(crate) const KNOWN_DEVICE_OPEN_FLAGS_MASK: u32 =
    DEVICE_OPEN_RAW.0 | BACKEND_OPEN_ALSA_NO_RESAMPLE.0 | BACKEND_OPEN_JACK_NO_AUTOCONNECT.0;

/// Backend supports hotplug notifications.
pub(crate) const BACKEND_CAPABILITY_HOTPLUG_EVENTS: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x1);
/// Backend supports default-route change notifications.
pub(crate) const BACKEND_CAPABILITY_DEFAULT_ROUTE_EVENTS: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x2);
/// Backend supports backend-disconnect notifications.
pub(crate) const BACKEND_CAPABILITY_BACKEND_DISCONNECT_EVENTS: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x4);
/// Backend supports shared stream mode.
pub(crate) const BACKEND_CAPABILITY_SHARED_MODE: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x8);
/// Backend supports exclusive stream mode.
pub(crate) const BACKEND_CAPABILITY_EXCLUSIVE_MODE: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x10);
/// Backend supports loopback capture.
pub(crate) const BACKEND_CAPABILITY_LOOPBACK: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x20);
/// Backend supports non-interleaved stream buffers.
pub(crate) const BACKEND_CAPABILITY_NON_INTERLEAVED: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x40);
/// Backend supports device-clock or hardware timestamp correlation.
pub(crate) const BACKEND_CAPABILITY_DEVICE_CLOCK: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x80);
/// Backend supports scheduled write or presentation-time submission.
pub(crate) const BACKEND_CAPABILITY_SCHEDULED_WRITE: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x100);
/// Backend supports native device-event ingress in addition to polling fallback.
pub(crate) const BACKEND_CAPABILITY_NATIVE_EVENT_FEED: AudioBackendCapabilityFlags =
    AudioBackendCapabilityFlags(0x200);
/// Device supports shared stream mode.
pub(crate) const DEVICE_CAPABILITY_SHARED_MODE: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x1);
/// Device supports exclusive stream mode.
#[allow(dead_code)]
pub(crate) const DEVICE_CAPABILITY_EXCLUSIVE_MODE: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x2);
/// Device supports loopback capture.
pub(crate) const DEVICE_CAPABILITY_LOOPBACK: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x4);
/// Device supports full-duplex stream lanes.
pub(crate) const DEVICE_CAPABILITY_FULL_DUPLEX: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x8);
/// Device supports stream gain controls.
pub(crate) const DEVICE_CAPABILITY_STREAM_VOLUME: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x10);
/// Device supports stream mute controls.
pub(crate) const DEVICE_CAPABILITY_STREAM_MUTE: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x20);
/// Device supports scheduled write submission.
pub(crate) const DEVICE_CAPABILITY_SCHEDULED_WRITE: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x40);
/// Device supports device-clock or hardware timestamp correlation.
pub(crate) const DEVICE_CAPABILITY_DEVICE_CLOCK: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x80);
/// Device supports backend reroute notifications.
pub(crate) const DEVICE_CAPABILITY_REROUTE_EVENTS: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x100);
/// Device supports interruption notifications.
pub(crate) const DEVICE_CAPABILITY_INTERRUPTION_EVENTS: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x200);
/// Device supports backend disconnect notifications.
pub(crate) const DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x400);
/// Device supports bit-exact PCM with no backend-side conversion.
pub(crate) const DEVICE_CAPABILITY_BIT_EXACT_PCM: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x800);

/// Request non-interleaved stream buffers where supported.
pub(crate) const STREAM_FLAG_NON_INTERLEAVED: AudioStreamFlags = AudioStreamFlags(0x1);
/// Request minimized callback period where supported.
pub(crate) const STREAM_FLAG_MINIMIZE_LATENCY: AudioStreamFlags = AudioStreamFlags(0x2);
/// Request explicit sample-format matching with no backend substitution.
pub(crate) const STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT: AudioStreamFlags = AudioStreamFlags(0x8);
/// Disable backend-side automatic format conversion where possible.
pub(crate) const STREAM_FLAG_NO_AUTO_CONVERT: AudioStreamFlags = AudioStreamFlags(0x10);
/// Request backend xrun status reporting where available.
pub(crate) const STREAM_FLAG_REPORT_XRUN: AudioStreamFlags = AudioStreamFlags(0x20);
/// Mask for all known stream option bits.
pub(crate) const KNOWN_STREAM_FLAGS_MASK: u32 = STREAM_FLAG_NON_INTERLEAVED.0
    | STREAM_FLAG_MINIMIZE_LATENCY.0
    | STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0
    | STREAM_FLAG_NO_AUTO_CONVERT.0
    | STREAM_FLAG_REPORT_XRUN.0;

/// Require one non-interleaved stream layout.
pub(crate) const STREAM_REQUIRE_NON_INTERLEAVED: AudioStreamRequirementFlags =
    AudioStreamRequirementFlags(0x1);
/// Require scheduled write support.
pub(crate) const STREAM_REQUIRE_SCHEDULED_WRITE: AudioStreamRequirementFlags =
    AudioStreamRequirementFlags(0x2);
/// Require pause support.
pub(crate) const STREAM_REQUIRE_PAUSE: AudioStreamRequirementFlags =
    AudioStreamRequirementFlags(0x4);
/// Require hardware timestamp correlation.
pub(crate) const STREAM_REQUIRE_HARDWARE_TIMESTAMPS: AudioStreamRequirementFlags =
    AudioStreamRequirementFlags(0x8);
/// Require bit-exact PCM behavior.
pub(crate) const STREAM_REQUIRE_BIT_EXACT_PCM: AudioStreamRequirementFlags =
    AudioStreamRequirementFlags(0x10);
/// Mask for all known stream requirement bits.
pub(crate) const KNOWN_STREAM_REQUIREMENT_FLAGS_MASK: u32 = STREAM_REQUIRE_NON_INTERLEAVED.0
    | STREAM_REQUIRE_SCHEDULED_WRITE.0
    | STREAM_REQUIRE_PAUSE.0
    | STREAM_REQUIRE_HARDWARE_TIMESTAMPS.0
    | STREAM_REQUIRE_BIT_EXACT_PCM.0;

/// Supported-stream-flag bit for non-interleaved buffers.
pub(crate) const SUPPORTED_STREAM_FLAG_NON_INTERLEAVED: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x1);
/// Supported-stream-flag bit for low-latency callback policy.
pub(crate) const SUPPORTED_STREAM_FLAG_MINIMIZE_LATENCY: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x2);
/// Supported-stream-flag bit for explicit sample-format matching.
pub(crate) const SUPPORTED_STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x8);
/// Supported-stream-flag bit for disabling backend auto-convert.
pub(crate) const SUPPORTED_STREAM_FLAG_NO_AUTO_CONVERT: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x10);
/// Supported-stream-flag bit for xrun reporting.
pub(crate) const SUPPORTED_STREAM_FLAG_REPORT_XRUN: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x20);

/// Supported stream requirement bit for non-interleaved buffers.
pub(crate) const SUPPORTED_STREAM_REQUIREMENT_NON_INTERLEAVED:
    AudioSupportedStreamRequirementFlags = AudioSupportedStreamRequirementFlags(0x1);
/// Supported stream requirement bit for scheduled writes.
pub(crate) const SUPPORTED_STREAM_REQUIREMENT_SCHEDULED_WRITE:
    AudioSupportedStreamRequirementFlags = AudioSupportedStreamRequirementFlags(0x2);
/// Supported stream requirement bit for pause support.
pub(crate) const SUPPORTED_STREAM_REQUIREMENT_PAUSE: AudioSupportedStreamRequirementFlags =
    AudioSupportedStreamRequirementFlags(0x4);
/// Supported stream requirement bit for hardware timestamps.
pub(crate) const SUPPORTED_STREAM_REQUIREMENT_HARDWARE_TIMESTAMPS:
    AudioSupportedStreamRequirementFlags = AudioSupportedStreamRequirementFlags(0x8);
/// Supported stream requirement bit for bit-exact PCM.
pub(crate) const SUPPORTED_STREAM_REQUIREMENT_BIT_EXACT_PCM: AudioSupportedStreamRequirementFlags =
    AudioSupportedStreamRequirementFlags(0x10);

/// Supported event-subscription bit for device hotplug.
pub(crate) const SUPPORTED_EVENT_SUBSCRIPTION_DEVICE_HOTPLUG: AudioSupportedEventSubscriptionFlags =
    AudioSupportedEventSubscriptionFlags(0x1);
/// Supported event-subscription bit for default-route changes.
pub(crate) const SUPPORTED_EVENT_SUBSCRIPTION_DEFAULT_ROUTE: AudioSupportedEventSubscriptionFlags =
    AudioSupportedEventSubscriptionFlags(0x2);
/// Supported event-subscription bit for format changes.
pub(crate) const SUPPORTED_EVENT_SUBSCRIPTION_FORMAT_CHANGE: AudioSupportedEventSubscriptionFlags =
    AudioSupportedEventSubscriptionFlags(0x4);
/// Supported event-subscription bit for reroute changes.
pub(crate) const SUPPORTED_EVENT_SUBSCRIPTION_REROUTE: AudioSupportedEventSubscriptionFlags =
    AudioSupportedEventSubscriptionFlags(0x8);
/// Supported event-subscription bit for interruptions.
pub(crate) const SUPPORTED_EVENT_SUBSCRIPTION_INTERRUPTION: AudioSupportedEventSubscriptionFlags =
    AudioSupportedEventSubscriptionFlags(0x10);
/// Supported event-subscription bit for backend resets and disconnects.
pub(crate) const SUPPORTED_EVENT_SUBSCRIPTION_BACKEND: AudioSupportedEventSubscriptionFlags =
    AudioSupportedEventSubscriptionFlags(0x20);
/// Supported event-subscription bit for stream events.
pub(crate) const SUPPORTED_EVENT_SUBSCRIPTION_STREAM: AudioSupportedEventSubscriptionFlags =
    AudioSupportedEventSubscriptionFlags(0x40);
/// All supported event-subscription bits known by this runtime.
pub(crate) const KNOWN_SUPPORTED_EVENT_SUBSCRIPTION_FLAGS_MASK: u32 =
    SUPPORTED_EVENT_SUBSCRIPTION_DEVICE_HOTPLUG.0
        | SUPPORTED_EVENT_SUBSCRIPTION_DEFAULT_ROUTE.0
        | SUPPORTED_EVENT_SUBSCRIPTION_FORMAT_CHANGE.0
        | SUPPORTED_EVENT_SUBSCRIPTION_REROUTE.0
        | SUPPORTED_EVENT_SUBSCRIPTION_INTERRUPTION.0
        | SUPPORTED_EVENT_SUBSCRIPTION_BACKEND.0
        | SUPPORTED_EVENT_SUBSCRIPTION_STREAM.0;

/// Supported stream-clock bit for monotonic clocks.
pub(crate) const SUPPORTED_STREAM_CLOCK_MONOTONIC: AudioSupportedStreamClockDomains =
    AudioSupportedStreamClockDomains(0x1);
/// Supported stream-clock bit for wall clocks.
pub(crate) const SUPPORTED_STREAM_CLOCK_WALL: AudioSupportedStreamClockDomains =
    AudioSupportedStreamClockDomains(0x2);
/// Supported stream-clock bit for device clocks.
pub(crate) const SUPPORTED_STREAM_CLOCK_DEVICE: AudioSupportedStreamClockDomains =
    AudioSupportedStreamClockDomains(0x4);
/// Supported stream-clock bit for callback timestamps.
pub(crate) const SUPPORTED_STREAM_CLOCK_CALLBACK: AudioSupportedStreamClockDomains =
    AudioSupportedStreamClockDomains(0x8);
/// Supported stream-clock bit for input ADC timestamps.
pub(crate) const SUPPORTED_STREAM_CLOCK_INPUT_ADC: AudioSupportedStreamClockDomains =
    AudioSupportedStreamClockDomains(0x10);
/// Supported stream-clock bit for output DAC timestamps.
pub(crate) const SUPPORTED_STREAM_CLOCK_OUTPUT_DAC: AudioSupportedStreamClockDomains =
    AudioSupportedStreamClockDomains(0x20);

/// Supported-direction bit for playback lanes.
pub(crate) const DIRECTION_MASK_PLAYBACK: u32 = 1u32 << 0;
/// Supported-direction bit for capture lanes.
pub(crate) const DIRECTION_MASK_CAPTURE: u32 = 1u32 << 1;
/// Supported-direction bit for duplex lanes.
pub(crate) const DIRECTION_MASK_DUPLEX: u32 = 1u32 << 2;
/// Supported-direction bit for loopback lanes.
pub(crate) const DIRECTION_MASK_LOOPBACK: u32 = 1u32 << 3;

/// Input-underflow status bit.
pub(crate) const STREAM_STATUS_INPUT_UNDERFLOW: AudioStreamStatusFlags =
    AudioStreamStatusFlags(0x1);
/// Input-overflow status bit.
pub(crate) const STREAM_STATUS_INPUT_OVERFLOW: AudioStreamStatusFlags = AudioStreamStatusFlags(0x2);
/// Output-underflow status bit.
pub(crate) const STREAM_STATUS_OUTPUT_UNDERFLOW: AudioStreamStatusFlags =
    AudioStreamStatusFlags(0x4);
/// Output-overflow status bit.
pub(crate) const STREAM_STATUS_OUTPUT_OVERFLOW: AudioStreamStatusFlags =
    AudioStreamStatusFlags(0x8);

/// Event-subscription hotplug bit.
pub(crate) const EVENT_SUBSCRIBE_DEVICE_HOTPLUG: AudioEventSubscriptionFlags =
    AudioEventSubscriptionFlags(0x1);
/// Event-subscription default-route bit.
pub(crate) const EVENT_SUBSCRIBE_DEFAULT_ROUTE: AudioEventSubscriptionFlags =
    AudioEventSubscriptionFlags(0x2);
/// Event-subscription format-change bit.
pub(crate) const EVENT_SUBSCRIBE_FORMAT_CHANGE: AudioEventSubscriptionFlags =
    AudioEventSubscriptionFlags(0x4);
/// Event-subscription reroute bit.
pub(crate) const EVENT_SUBSCRIBE_REROUTE: AudioEventSubscriptionFlags =
    AudioEventSubscriptionFlags(0x8);
/// Event-subscription interruption bit.
pub(crate) const EVENT_SUBSCRIBE_INTERRUPTION: AudioEventSubscriptionFlags =
    AudioEventSubscriptionFlags(0x10);
/// Event-subscription backend-reset bit.
pub(crate) const EVENT_SUBSCRIBE_BACKEND: AudioEventSubscriptionFlags =
    AudioEventSubscriptionFlags(0x20);
/// Event-subscription stream-monitor bit.
pub(crate) const EVENT_SUBSCRIBE_STREAM: AudioEventSubscriptionFlags =
    AudioEventSubscriptionFlags(0x40);

/// Preferred null-device sample rate in hertz.
pub(crate) const NULL_DEVICE_PREFERRED_SAMPLE_RATE: u32 = 48_000;
/// Minimum null-device sample rate in hertz.
pub(crate) const NULL_DEVICE_MIN_SAMPLE_RATE: u32 = 8_000;
/// Maximum null-device sample rate in hertz.
pub(crate) const NULL_DEVICE_MAX_SAMPLE_RATE: u32 = 192_000;
/// Preferred null-device period in frames.
pub(crate) const NULL_DEVICE_PREFERRED_PERIOD_FRAMES: u32 = 256;
/// Maximum null-device period in frames.
pub(crate) const NULL_DEVICE_MAX_PERIOD_FRAMES: u32 = 8_192;
/// Preferred null-device stereo channel mask.
pub(crate) const NULL_DEVICE_PREFERRED_CHANNEL_MASK: u64 = 0b11;
/// Supported null-device channel mask.
pub(crate) const NULL_DEVICE_SUPPORTED_CHANNEL_MASK: u64 = 0xff;
/// Default stream gain for newly opened streams.
pub(crate) const DEFAULT_STREAM_VOLUME: f64 = 1.0;

/// Return one process monotonic timestamp in nanoseconds.
pub(crate) fn host_monotonic_nanos() -> u64 {
    core_platform::monotonic_now_ns()
}

/// Return the configured monitor poll interval for audio event monitor workers.
#[cfg(target_os = "linux")]
pub(crate) fn resolved_event_monitor_poll_interval_ns(default_ns: u64) -> u64 {
    let configured = with_binding_call_context(|context| {
        Ok(context.worker().options.audio.event_monitor_poll_interval_ns)
    })
    .ok()
    .flatten();

    configured
        .unwrap_or(default_ns)
        .clamp(MIN_EVENT_POLL_INTERVAL_NS, MAX_EVENT_POLL_INTERVAL_NS)
}

/// Return the configured default queue capacity for audio event subscriptions.
pub(crate) fn resolved_default_event_queue_capacity(ctx: &BindingCallContext) -> u32 {
    let configured = ctx.worker().options.audio.event_queue_capacity;
    let configured = core_platform::option_u64_to_u32(configured);
    configured.unwrap_or(DEFAULT_EVENT_QUEUE_CAPACITY).max(1)
}

/// Return the configured default poll interval for audio event subscriptions.
pub(crate) fn resolved_default_event_poll_interval_ns(ctx: &BindingCallContext) -> u64 {
    let configured = ctx.worker().options.audio.default_event_poll_interval_ns;
    configured
        .unwrap_or(EVENT_POLL_INTERVAL_NS)
        .clamp(MIN_EVENT_POLL_INTERVAL_NS, MAX_EVENT_POLL_INTERVAL_NS)
}

#[cfg(unix)]
/// Return the configured maximum bytes accepted per audio stream read call.
pub(crate) fn resolved_max_stream_read_bytes(ctx: &BindingCallContext) -> u32 {
    let configured = ctx.worker().options.audio.max_stream_read_bytes;
    let configured = core_platform::option_u64_to_u32(configured);
    configured
        .unwrap_or(MAX_STREAM_READ_BYTES)
        .clamp(1, MAX_STREAM_READ_BYTES)
}

/// Return the configured maximum queued stream frame budget.
pub(crate) fn resolved_max_queued_frames() -> usize {
    let configured =
        with_binding_call_context(|context| Ok(context.worker().options.audio.max_queued_frames))
            .ok()
            .flatten();
    let configured = core_platform::option_u64_to_usize(configured);

    configured
        .unwrap_or(MAX_QUEUED_FRAMES)
        .clamp(1, MAX_QUEUED_FRAMES)
}

/// Return one worker poll period derived from stream geometry and runtime overrides.
pub(crate) fn resolved_worker_poll_period(period_frames: u32, sample_rate: u32) -> Duration {
    let default_interval_ns = (period_frames as u64)
        .saturating_mul(1_000_000_000u64)
        .checked_div(sample_rate.max(1) as u64)
        .unwrap_or(MIN_WORKER_POLL_INTERVAL_NS)
        .max(MIN_WORKER_POLL_INTERVAL_NS);

    let configured = with_binding_call_context(|context| {
        Ok(context.worker().options.audio.worker_poll_interval_ns)
    })
    .ok()
    .flatten();
    let interval_ns = configured
        .unwrap_or(default_interval_ns)
        .max(MIN_WORKER_POLL_INTERVAL_NS);

    Duration::from_nanos(interval_ns)
}
