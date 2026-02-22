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
/// Disable backend-side automatic format conversion where possible.
pub(crate) const DEVICE_OPEN_NO_AUTO_CONVERT: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x4);
/// Disable backend-side automatic stream routing where possible.
pub(crate) const DEVICE_OPEN_NO_AUTO_ROUTE: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x8);
/// Request hardware offload lanes where available.
pub(crate) const DEVICE_OPEN_HARDWARE_OFFLOAD: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x10);
/// Request realtime callback thread scheduling where available.
pub(crate) const DEVICE_OPEN_REALTIME_THREAD: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x20);
/// Request raw endpoint mode where available.
pub(crate) const DEVICE_OPEN_RAW: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x40);
/// Request explicit sample-format matching with no backend substitution.
pub(crate) const DEVICE_OPEN_EXPLICIT_SAMPLE_FORMAT: AudioDeviceOpenFlags =
    AudioDeviceOpenFlags(0x80);
/// Request backend-default PCM routing for ALSA-style stacks.
pub(crate) const DEVICE_OPEN_USE_DEFAULT_PCM: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x100);
/// Request JACK-style no-autoconnect behavior.
pub(crate) const DEVICE_OPEN_JACK_DONT_CONNECT: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x200);
/// Request hard-exclusive endpoint ownership where available.
pub(crate) const DEVICE_OPEN_HOG_DEVICE: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x400);
/// Request minimized callback period where available.
pub(crate) const DEVICE_OPEN_MINIMIZE_LATENCY: AudioDeviceOpenFlags = AudioDeviceOpenFlags(0x800);
/// Mask for all known device-open option bits.
pub(crate) const KNOWN_DEVICE_OPEN_FLAGS_MASK: u32 = DEVICE_OPEN_FOLLOW_DEFAULT_ROUTE.0
    | DEVICE_OPEN_LOW_LATENCY.0
    | DEVICE_OPEN_NO_AUTO_CONVERT.0
    | DEVICE_OPEN_NO_AUTO_ROUTE.0
    | DEVICE_OPEN_HARDWARE_OFFLOAD.0
    | DEVICE_OPEN_REALTIME_THREAD.0
    | DEVICE_OPEN_RAW.0
    | DEVICE_OPEN_EXPLICIT_SAMPLE_FORMAT.0
    | DEVICE_OPEN_USE_DEFAULT_PCM.0
    | DEVICE_OPEN_JACK_DONT_CONNECT.0
    | DEVICE_OPEN_HOG_DEVICE.0
    | DEVICE_OPEN_MINIMIZE_LATENCY.0;

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

/// Request one event-driven callback scheduling mode on WASAPI-like backends.
pub(crate) const BACKEND_OPEN_WASAPI_EVENT_CALLBACK: AudioBackendOpenFlags =
    AudioBackendOpenFlags(0x1);
/// Request one exclusive endpoint path on backends that expose explicit exclusive mode.
pub(crate) const BACKEND_OPEN_REQUIRE_EXCLUSIVE: AudioBackendOpenFlags = AudioBackendOpenFlags(0x2);
/// Request one host loopback path when supported by the selected backend.
pub(crate) const BACKEND_OPEN_REQUIRE_LOOPBACK: AudioBackendOpenFlags = AudioBackendOpenFlags(0x4);
/// Request no-autoconnect behavior on JACK-like backends.
pub(crate) const BACKEND_OPEN_JACK_NO_AUTOCONNECT: AudioBackendOpenFlags =
    AudioBackendOpenFlags(0x8);
/// Request no-resample behavior on ALSA-style backends.
pub(crate) const BACKEND_OPEN_ALSA_NO_RESAMPLE: AudioBackendOpenFlags = AudioBackendOpenFlags(0x10);
/// Request hog mode on CoreAudio-like backends.
pub(crate) const BACKEND_OPEN_COREAUDIO_HOG_MODE: AudioBackendOpenFlags =
    AudioBackendOpenFlags(0x20);
/// Request hardware timestamps when backend support exists.
pub(crate) const BACKEND_OPEN_REQUIRE_HARDWARE_TIMESTAMPS: AudioBackendOpenFlags =
    AudioBackendOpenFlags(0x40);
/// Request bit-exact PCM path with no backend-side format conversion.
pub(crate) const BACKEND_OPEN_REQUIRE_BIT_EXACT_PCM: AudioBackendOpenFlags =
    AudioBackendOpenFlags(0x80);

/// Request non-interleaved stream buffers where supported.
pub(crate) const STREAM_FLAG_NON_INTERLEAVED: AudioStreamFlags = AudioStreamFlags(0x1);
/// Request minimized callback period where supported.
pub(crate) const STREAM_FLAG_MINIMIZE_LATENCY: AudioStreamFlags = AudioStreamFlags(0x2);
/// Request hard-exclusive endpoint ownership where available.
pub(crate) const STREAM_FLAG_HOG_DEVICE: AudioStreamFlags = AudioStreamFlags(0x4);
/// Request realtime callback scheduling where supported.
pub(crate) const STREAM_FLAG_SCHEDULE_REALTIME: AudioStreamFlags = AudioStreamFlags(0x8);
/// Request backend-default PCM routing for ALSA-style stacks.
pub(crate) const STREAM_FLAG_ALSA_USE_DEFAULT: AudioStreamFlags = AudioStreamFlags(0x10);
/// Request JACK-style no-autoconnect behavior.
pub(crate) const STREAM_FLAG_JACK_DONT_CONNECT: AudioStreamFlags = AudioStreamFlags(0x20);
/// Request explicit sample-format matching with no backend substitution.
pub(crate) const STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT: AudioStreamFlags = AudioStreamFlags(0x40);
/// Disable backend-side automatic format conversion where possible.
pub(crate) const STREAM_FLAG_NO_AUTO_CONVERT: AudioStreamFlags = AudioStreamFlags(0x80);
/// Request backend xrun status reporting where available.
pub(crate) const STREAM_FLAG_REPORT_XRUN: AudioStreamFlags = AudioStreamFlags(0x100);
/// Request input no-drop capture behavior where available.
pub(crate) const STREAM_FLAG_NEVER_DROP_INPUT: AudioStreamFlags = AudioStreamFlags(0x200);
/// Request backend output-buffer priming behavior where available.
pub(crate) const STREAM_FLAG_PRIME_OUTPUT_BUFFERS: AudioStreamFlags = AudioStreamFlags(0x400);
/// Mask for all known stream option bits.
pub(crate) const KNOWN_STREAM_FLAGS_MASK: u32 = STREAM_FLAG_NON_INTERLEAVED.0
    | STREAM_FLAG_MINIMIZE_LATENCY.0
    | STREAM_FLAG_HOG_DEVICE.0
    | STREAM_FLAG_SCHEDULE_REALTIME.0
    | STREAM_FLAG_ALSA_USE_DEFAULT.0
    | STREAM_FLAG_JACK_DONT_CONNECT.0
    | STREAM_FLAG_EXPLICIT_SAMPLE_FORMAT.0
    | STREAM_FLAG_NO_AUTO_CONVERT.0
    | STREAM_FLAG_REPORT_XRUN.0
    | STREAM_FLAG_NEVER_DROP_INPUT.0
    | STREAM_FLAG_PRIME_OUTPUT_BUFFERS.0;

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
