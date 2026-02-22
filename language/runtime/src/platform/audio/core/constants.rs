use super::*;

/// Resource label for audio device handles.
pub(crate) const AUDIO_DEVICE_RESOURCE_LABEL: &str = "audio.device";
/// Resource label for audio stream handles.
pub(crate) const AUDIO_STREAM_RESOURCE_LABEL: &str = "audio.stream";
/// Resource label for audio event handles.
pub(crate) const AUDIO_EVENT_RESOURCE_LABEL: &str = "audio.event";

/// Maximum read payload bytes accepted by one call.
pub(crate) const MAX_STREAM_READ_BYTES: u32 = 8 * 1024 * 1024;
/// Maximum queued stream latency budget in frames.
pub(crate) const MAX_QUEUED_FRAMES: usize = 96_000;
/// Default queue capacity for one event subscription.
pub(crate) const DEFAULT_EVENT_QUEUE_CAPACITY: u32 = 1024;
/// Poll interval for event reads.
pub(crate) const EVENT_POLL_INTERVAL_NS: u64 = 5_000_000;
/// Minimum event poll interval accepted from one subscription request.
pub(crate) const MIN_EVENT_POLL_INTERVAL_NS: u64 = 100_000;
/// Maximum event poll interval accepted from one subscription request.
pub(crate) const MAX_EVENT_POLL_INTERVAL_NS: u64 = 1_000_000_000;
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

/// Follow default-route migrations for default endpoint identifiers.
pub(crate) const DEVICE_OPEN_FOLLOW_DEFAULT_ROUTE: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x1);
/// Request low-latency scheduling policy where supported.
pub(crate) const DEVICE_OPEN_LOW_LATENCY: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x2);
/// Request realtime callback thread scheduling where available.
pub(crate) const DEVICE_OPEN_REALTIME_THREAD: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x4);
/// Request raw endpoint mode where available.
pub(crate) const DEVICE_OPEN_RAW: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x8);
/// Mask for all known device-open option bits.
pub(crate) const KNOWN_DEVICE_OPEN_FLAGS_MASK: u32 = DEVICE_OPEN_FOLLOW_DEFAULT_ROUTE.0
    | DEVICE_OPEN_LOW_LATENCY.0
    | DEVICE_OPEN_REALTIME_THREAD.0
    | DEVICE_OPEN_RAW.0;

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
/// Device supports shared stream mode.
pub(crate) const DEVICE_CAPABILITY_SHARED_MODE: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x1);
/// Device supports exclusive stream mode.
#[cfg(any(
    all(windows, feature = "audio-wasapi"),
    all(windows, feature = "audio-asio"),
    all(target_vendor = "apple", feature = "audio-coreaudio"),
    all(target_os = "linux", feature = "audio-alsa"),
    all(target_os = "android", feature = "audio-aaudio"),
))]
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
/// Request realtime callback scheduling where supported.
pub(crate) const STREAM_FLAG_SCHEDULE_REALTIME: AudioStreamFlags = AudioStreamFlags(0x4);
/// Request explicit sample-format matching with no backend substitution.
pub(crate) const STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT: AudioStreamFlags = AudioStreamFlags(0x8);
/// Disable backend-side automatic format conversion where possible.
pub(crate) const STREAM_FLAG_NO_AUTO_CONVERT: AudioStreamFlags = AudioStreamFlags(0x10);
/// Request backend xrun status reporting where available.
pub(crate) const STREAM_FLAG_REPORT_XRUN: AudioStreamFlags = AudioStreamFlags(0x20);
/// Request input no-drop capture behavior where available.
pub(crate) const STREAM_FLAG_NEVER_DROP_INPUT: AudioStreamFlags = AudioStreamFlags(0x40);
/// Request backend output-buffer priming behavior where available.
pub(crate) const STREAM_FLAG_PRIME_OUTPUT_BUFFERS: AudioStreamFlags = AudioStreamFlags(0x80);
/// Mask for all known stream option bits.
pub(crate) const KNOWN_STREAM_FLAGS_MASK: u32 = STREAM_FLAG_NON_INTERLEAVED.0
    | STREAM_FLAG_MINIMIZE_LATENCY.0
    | STREAM_FLAG_SCHEDULE_REALTIME.0
    | STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0
    | STREAM_FLAG_NO_AUTO_CONVERT.0
    | STREAM_FLAG_REPORT_XRUN.0
    | STREAM_FLAG_NEVER_DROP_INPUT.0
    | STREAM_FLAG_PRIME_OUTPUT_BUFFERS.0;

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
/// Supported-stream-flag bit for low-latency policy.
pub(crate) const SUPPORTED_STREAM_FLAG_MINIMIZE_LATENCY: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x2);
/// Supported-stream-flag bit for realtime scheduling.
pub(crate) const SUPPORTED_STREAM_FLAG_SCHEDULE_REALTIME: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x4);
/// Supported-stream-flag bit for explicit format selection.
pub(crate) const SUPPORTED_STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x8);
/// Supported-stream-flag bit for no auto-convert policy.
pub(crate) const SUPPORTED_STREAM_FLAG_NO_AUTO_CONVERT: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x10);
/// Supported-stream-flag bit for xrun reporting.
pub(crate) const SUPPORTED_STREAM_FLAG_REPORT_XRUN: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x20);
/// Supported-stream-flag bit for never-drop-input policy.
pub(crate) const SUPPORTED_STREAM_FLAG_NEVER_DROP_INPUT: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x40);
/// Supported-stream-flag bit for output priming.
pub(crate) const SUPPORTED_STREAM_FLAG_PRIME_OUTPUT_BUFFERS: AudioSupportedStreamFlags =
    AudioSupportedStreamFlags(0x80);
/// All supported-stream-flag bits known by this runtime.
pub(crate) const KNOWN_SUPPORTED_STREAM_FLAGS_MASK: u32 = SUPPORTED_STREAM_FLAG_NON_INTERLEAVED.0
    | SUPPORTED_STREAM_FLAG_MINIMIZE_LATENCY.0
    | SUPPORTED_STREAM_FLAG_SCHEDULE_REALTIME.0
    | SUPPORTED_STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0
    | SUPPORTED_STREAM_FLAG_NO_AUTO_CONVERT.0
    | SUPPORTED_STREAM_FLAG_REPORT_XRUN.0
    | SUPPORTED_STREAM_FLAG_NEVER_DROP_INPUT.0
    | SUPPORTED_STREAM_FLAG_PRIME_OUTPUT_BUFFERS.0;

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

/// Global monotonic epoch used by callback threads.
pub(crate) static MONO_EPOCH: OnceLock<Instant> = OnceLock::new();

/// Return one process monotonic timestamp in nanoseconds.
pub(crate) fn host_monotonic_nanos() -> u64 {
    let elapsed = MONO_EPOCH.get_or_init(Instant::now).elapsed();
    elapsed.as_nanos().min(u64::MAX as u128) as u64
}
