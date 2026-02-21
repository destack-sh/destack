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
/// Poll interval for event reads.
pub(crate) const EVENT_POLL_INTERVAL_NS: u64 = 5_000_000;
/// Minimum stream period in frames.
pub(crate) const MIN_STREAM_PERIOD_FRAMES: u32 = 64;
/// Shared-mode bit in one device share-mode mask.
pub(crate) const SHARE_MODE_SHARED_BIT: u32 = 1u32 << 0;
/// Exclusive-mode bit in one device share-mode mask.
pub(crate) const SHARE_MODE_EXCLUSIVE_BIT: u32 = 1u32 << 1;

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
/// Device supports shared stream mode.
pub(crate) const DEVICE_CAPABILITY_SHARED_MODE: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x1);
/// Device supports exclusive stream mode.
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
/// Device supports backend reroute notifications.
pub(crate) const DEVICE_CAPABILITY_REROUTE_EVENTS: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x100);
/// Device supports backend disconnect notifications.
pub(crate) const DEVICE_CAPABILITY_BACKEND_DISCONNECT_EVENTS: AudioDeviceCapabilityFlags =
    AudioDeviceCapabilityFlags(0x400);

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
