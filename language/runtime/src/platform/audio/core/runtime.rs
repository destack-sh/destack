use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex};

use crate::platform::audio::{
    AudioBackend, AudioEventDeliveryMode, AudioEventKind, AudioEventSource,
    AudioEventSubscriptionFlags, AudioEventSubscriptionOptions,
};
use crate::platform::resource::ResourceId;
use crate::runtime::{BindingCallContext, RuntimeEventLog, RuntimeStreamRegistry, WorkerId};

use super::constants::{
    EVENT_SUBSCRIBE_BACKEND, EVENT_SUBSCRIBE_DEFAULT_ROUTE, EVENT_SUBSCRIBE_DEVICE_HOTPLUG,
    EVENT_SUBSCRIBE_FORMAT_CHANGE, EVENT_SUBSCRIBE_INTERRUPTION, EVENT_SUBSCRIBE_REROUTE,
    EVENT_SUBSCRIBE_STREAM,
};
use super::model::{
    AudioDeviceMonitorBaseline, AudioEventRecord, AudioEventStream, AudioStreamMonitorBaseline,
};

/// Mask for all recognized subscription flags.
pub(crate) const KNOWN_EVENT_SUBSCRIPTION_FLAGS_MASK: u32 = EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0
    | EVENT_SUBSCRIBE_DEFAULT_ROUTE.0
    | EVENT_SUBSCRIBE_FORMAT_CHANGE.0
    | EVENT_SUBSCRIBE_REROUTE.0
    | EVENT_SUBSCRIBE_INTERRUPTION.0
    | EVENT_SUBSCRIBE_BACKEND.0
    | EVENT_SUBSCRIBE_STREAM.0;

/// Mask for flags that require stream tracking.
pub(crate) const STREAM_EVENT_SUBSCRIPTION_FLAGS_MASK: u32 =
    EVENT_SUBSCRIBE_INTERRUPTION.0 | EVENT_SUBSCRIBE_BACKEND.0 | EVENT_SUBSCRIBE_STREAM.0;

/// Mask for flags that request device-level event tracking.
pub(crate) const DEVICE_EVENT_SUBSCRIPTION_FLAGS_MASK: u32 = EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0
    | EVENT_SUBSCRIBE_DEFAULT_ROUTE.0
    | EVENT_SUBSCRIBE_FORMAT_CHANGE.0
    | EVENT_SUBSCRIBE_REROUTE.0;

/// Runtime-owned mutable state for one worker audio module instance.
pub(crate) struct AudioRuntimeState {
    /// Owning worker identifier.
    pub(crate) worker_id: WorkerId,
    /// Runtime-owned live audio event log.
    pub(crate) event_log: Mutex<RuntimeEventLog<AudioEventRecord>>,
    /// Wake signal for audio event readers.
    pub(crate) event_signal: Condvar,
    /// Registered audio event streams for this runtime.
    pub(crate) stream_registry: RuntimeStreamRegistry<AudioEventStream>,
    /// Runtime-owned backend monitor baselines keyed by backend.
    pub(crate) device_monitor_baselines: Mutex<HashMap<AudioBackend, AudioDeviceMonitorBaseline>>,
    /// Runtime-owned stream monitor baselines keyed by stream resource id.
    pub(crate) stream_monitor_baselines: Mutex<HashMap<ResourceId, AudioStreamMonitorBaseline>>,
}

impl fmt::Debug for AudioRuntimeState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AudioRuntimeState")
            .finish_non_exhaustive()
    }
}

impl AudioRuntimeState {
    /// Build one runtime-owned audio state object for one worker.
    pub(crate) fn new(worker_id: WorkerId) -> Self {
        Self {
            worker_id,
            event_log: Mutex::new(RuntimeEventLog::default()),
            event_signal: Condvar::new(),
            stream_registry: RuntimeStreamRegistry::default(),
            device_monitor_baselines: Mutex::new(HashMap::new()),
            stream_monitor_baselines: Mutex::new(HashMap::new()),
        }
    }
}

/// Return runtime-owned state for audio event routing.
pub(crate) fn runtime_state(ctx: &BindingCallContext) -> Arc<AudioRuntimeState> {
    ctx.worker().platform_state.audio.runtime_state(ctx)
}

/// Return whether one stream tracks device-level events.
pub(crate) fn tracks_device_events(options: AudioEventSubscriptionOptions) -> bool {
    if options.flags.0 == 0 {
        return true;
    }

    (options.flags.0 & DEVICE_EVENT_SUBSCRIPTION_FLAGS_MASK) != 0
}

/// Return one subscription flag selector for one event kind.
pub(crate) fn subscription_flag_for_event(
    kind: AudioEventKind,
) -> Option<AudioEventSubscriptionFlags> {
    match kind {
        AudioEventKind::DeviceAdded | AudioEventKind::DeviceRemoved => {
            Some(EVENT_SUBSCRIBE_DEVICE_HOTPLUG)
        }
        AudioEventKind::DefaultPlaybackChanged
        | AudioEventKind::DefaultCaptureChanged
        | AudioEventKind::DefaultLoopbackChanged => Some(EVENT_SUBSCRIBE_DEFAULT_ROUTE),
        AudioEventKind::DeviceFormatChanged => Some(EVENT_SUBSCRIBE_FORMAT_CHANGE),
        AudioEventKind::DeviceRerouted => Some(EVENT_SUBSCRIBE_REROUTE),
        AudioEventKind::InterruptionBegan | AudioEventKind::InterruptionEnded => {
            Some(EVENT_SUBSCRIBE_INTERRUPTION)
        }
        AudioEventKind::BackendDisconnected | AudioEventKind::BackendReset => {
            Some(EVENT_SUBSCRIBE_BACKEND)
        }
        AudioEventKind::StreamStateChanged
        | AudioEventKind::StreamXRun
        | AudioEventKind::StreamDeviceChanged => Some(EVENT_SUBSCRIBE_STREAM),
    }
}

/// Return the native-only event-subscription support mask for one backend.
pub(crate) fn native_only_supported_subscription_flags(backend: AudioBackend) -> u32 {
    let mut flags = EVENT_SUBSCRIBE_STREAM.0;

    if backend == AudioBackend::CoreAudio {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_DEFAULT_ROUTE.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::Wasapi {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_DEFAULT_ROUTE.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::Jack {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::PulseAudio || backend == AudioBackend::PipeWire {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
        flags |= EVENT_SUBSCRIBE_DEFAULT_ROUTE.0;
        flags |= EVENT_SUBSCRIBE_FORMAT_CHANGE.0;
        flags |= EVENT_SUBSCRIBE_REROUTE.0;
    }

    if backend == AudioBackend::Alsa || backend == AudioBackend::Asio {
        flags |= EVENT_SUBSCRIBE_DEVICE_HOTPLUG.0;
    }

    if backend == AudioBackend::AAudio || backend == AudioBackend::OpenSLES {
        flags |= EVENT_SUBSCRIBE_BACKEND.0;
    }

    flags
}

/// Return whether one subscription enables one event category.
pub(crate) fn event_subscription_enabled(
    options: AudioEventSubscriptionOptions,
    flag: AudioEventSubscriptionFlags,
) -> bool {
    if options.flags.0 == 0 {
        return true;
    }

    (options.flags.0 & flag.0) != 0
}

/// Return whether one live record is visible to one stream.
pub(crate) fn stream_accepts_record(stream: &AudioEventStream, record: &AudioEventRecord) -> bool {
    if stream.options.backend != record.backend {
        return false;
    }

    if record.source == AudioEventSource::Native
        && stream.options.delivery_mode == AudioEventDeliveryMode::PollOnly
    {
        return false;
    }

    if record.source == AudioEventSource::SyntheticPoll
        && stream.options.delivery_mode == AudioEventDeliveryMode::NativeOnly
    {
        return false;
    }

    let Some(flag) = subscription_flag_for_event(record.kind) else {
        return false;
    };

    if !event_subscription_enabled(stream.options, flag) {
        return false;
    }

    if let Some(target_stream) = stream.options.stream
        && let Some(record_stream) = record.stream
    {
        return record_stream == target_stream;
    }

    true
}
