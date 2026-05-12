use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, Weak};
use std::thread::JoinHandle;

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::{
    AudioBackend, AudioChannelLayout, AudioDeviceCapabilityFlags, AudioDeviceDirection,
    AudioDeviceOpenOptions, AudioEventKind, AudioEventOverflowPolicy, AudioEventSource,
    AudioEventSubscriptionOptions, AudioShareMode, AudioStreamConfig, AudioStreamFlags,
    AudioStreamRequirementFlags, AudioStreamStateKind, AudioStreamStatusFlags,
};
use crate::platform::resource::{self, ResourceFinalizer, ResourceId};
use crate::runtime::WorkerCallbackHandle;

use super::constants::{DEFAULT_STREAM_VOLUME, host_monotonic_nanos};
use super::runtime::AudioRuntimeState;

/// One normalized host device descriptor.
#[derive(Debug, Clone)]
pub(crate) struct HostDeviceDescriptor {
    /// Stable runtime identifier.
    pub(crate) id: String,
    /// Stable runtime group identifier.
    pub(crate) group_id: String,
    /// Host display name.
    pub(crate) name: String,
    /// Host transport name.
    pub(crate) transport: String,
    /// Backend selector.
    pub(crate) backend: AudioBackend,
    /// Direction capabilities.
    pub(crate) direction: AudioDeviceDirection,
    /// Supported open directions for this endpoint.
    pub(crate) supported_directions: u32,
    /// Whether this endpoint is connected.
    pub(crate) connected: bool,
    /// Whether this endpoint is one raw endpoint lane.
    pub(crate) is_raw: bool,
    /// Whether this endpoint is default playback.
    pub(crate) is_default_playback: bool,
    /// Whether this endpoint is default capture.
    pub(crate) is_default_capture: bool,
    /// Whether this endpoint is default loopback.
    pub(crate) is_default_loopback: bool,
    /// Device capability flags.
    pub(crate) capability_flags: AudioDeviceCapabilityFlags,
    /// Preferred sample rate in hertz.
    pub(crate) preferred_sample_rate: u32,
    /// Minimum sample rate in hertz.
    pub(crate) min_sample_rate: u32,
    /// Maximum sample rate in hertz.
    pub(crate) max_sample_rate: u32,
    /// Preferred period in frames.
    pub(crate) preferred_period_frames: u32,
    /// Minimum channel count.
    pub(crate) min_channels: u16,
    /// Maximum channel count.
    pub(crate) max_channels: u16,
    /// Preferred layout selector.
    pub(crate) preferred_layout: AudioChannelLayout,
    /// Preferred channel mask.
    pub(crate) preferred_channel_mask: u64,
    /// Supported channel mask.
    pub(crate) supported_channel_mask: u64,
    /// Minimum period frames.
    pub(crate) min_period_frames: u32,
    /// Maximum period frames.
    pub(crate) max_period_frames: u32,
    /// Supported format mask.
    pub(crate) format_mask: u32,
    /// Supported share-mode mask.
    pub(crate) share_mode_mask: u32,
    /// Whether this is the synthetic null backend.
    pub(crate) is_null: bool,
}

/// Opened audio device payload.
#[derive(Debug, Clone)]
pub(crate) struct AudioDeviceHostState {
    /// Device metadata snapshot.
    pub(crate) info: HostDeviceDescriptor,
    /// Open-time direction for this handle.
    pub(crate) opened_direction: AudioDeviceDirection,
    /// Open-time options.
    pub(crate) options: AudioDeviceOpenOptions,
}

/// Per-stream runtime capability snapshot.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AudioStreamRuntimeCapabilities {
    /// Whether scheduled writes are supported.
    pub(crate) supports_write_at: bool,
    /// Whether pause and resume are supported.
    pub(crate) supports_pause: bool,
    /// Whether non-interleaved I/O is supported.
    pub(crate) supports_non_interleaved: bool,
    /// Whether stream gain control is supported.
    pub(crate) supports_volume: bool,
    /// Whether stream mute control is supported.
    pub(crate) supports_mute: bool,
    /// Whether hardware timestamp correlation is supported.
    pub(crate) supports_hardware_timestamps: bool,
}

/// Host stream operation hooks for backend-specific stream control.
pub(crate) trait AudioHostStreamOps: Send + Sync {
    /// Start backend stream processing.
    fn start(&self) -> RuntimeResult<()>;

    /// Pause or resume backend stream processing.
    fn pause(&self, pause: bool) -> RuntimeResult<()>;

    /// Stop backend stream processing.
    fn stop(&self) -> RuntimeResult<()>;

    /// Flush backend stream buffers.
    fn flush(&self) -> RuntimeResult<()>;
}

/// Stream synchronization payload.
#[derive(Debug)]
pub(crate) struct AudioStreamSync {
    /// Mutable stream state.
    pub(crate) state: Mutex<AudioStreamStateInner>,
    /// Wakeups for blocking read and write operations.
    pub(crate) wake: Condvar,
}

/// Mutable stream state.
#[derive(Debug)]
pub(crate) struct AudioStreamStateInner {
    /// Current stream state kind.
    pub(crate) state_kind: AudioStreamStateKind,
    /// Running flag.
    pub(crate) running: bool,
    /// Paused flag.
    pub(crate) paused: bool,
    /// Stream shutdown flag.
    pub(crate) shutdown: bool,
    /// Pending playback samples in normalized form.
    pub(crate) playback_samples: VecDeque<f32>,
    /// Pending capture samples in normalized form.
    pub(crate) capture_samples: VecDeque<f32>,
    /// Total processed frame count.
    pub(crate) stream_frames: u64,
    /// Backend xrun counter.
    pub(crate) xrun_count: u64,
    /// Input-underflow counter.
    pub(crate) input_underflow_count: u64,
    /// Input-overflow counter.
    pub(crate) input_overflow_count: u64,
    /// Output-underflow counter.
    pub(crate) output_underflow_count: u64,
    /// Output-overflow counter.
    pub(crate) output_overflow_count: u64,
    /// Current stream status flags.
    pub(crate) status_flags: AudioStreamStatusFlags,
    /// Last callback monotonic timestamp.
    pub(crate) last_callback_mono_ns: u64,
    /// First callback monotonic timestamp used for drift estimation.
    pub(crate) first_callback_mono_ns: u64,
    /// Stream frame counter at the first callback timing sample.
    pub(crate) first_callback_stream_frames: u64,
    /// Last callback-period jitter estimate in nanoseconds.
    pub(crate) last_period_jitter_ns: u64,
    /// Last input ADC timestamp in nanoseconds when available.
    pub(crate) last_input_adc_ns: u64,
    /// Last output DAC timestamp in nanoseconds when available.
    pub(crate) last_output_dac_ns: u64,
    /// Last callback CPU load estimate.
    pub(crate) last_callback_cpu_load: f64,
    /// Gain multiplier.
    pub(crate) volume: f64,
    /// Mute state.
    pub(crate) muted: bool,
    /// Last backend disconnect reason when the stream backend disconnects.
    pub(crate) last_backend_message: Option<String>,
}

/// Stream runtime payload.
pub(crate) struct AudioStreamHostState {
    /// Device metadata snapshot.
    pub(crate) device: HostDeviceDescriptor,
    /// Open-time direction lane.
    pub(crate) direction: AudioDeviceDirection,
    /// Requested stream config.
    pub(crate) requested: AudioStreamConfig,
    /// Requested stream option flags.
    pub(crate) requested_flags: AudioStreamFlags,
    /// Requested strict requirement flags.
    pub(crate) requested_requirements: AudioStreamRequirementFlags,
    /// Effective stream sample rate.
    pub(crate) sample_rate: u32,
    /// Effective stream channel count.
    pub(crate) channels: u16,
    /// Effective host period in frames.
    pub(crate) period_frames: u32,
    /// Effective per-stream queued frame budget.
    pub(crate) max_queued_frames: usize,
    /// Effective share mode.
    pub(crate) share_mode: AudioShareMode,
    /// Runtime capability snapshot for this stream instance.
    pub(crate) runtime_capabilities: AudioStreamRuntimeCapabilities,
    /// Optional backend stream operation hooks.
    pub(crate) host_ops: Mutex<Option<Arc<dyn AudioHostStreamOps>>>,
    /// Stream name used for diagnostics.
    pub(crate) name: Mutex<String>,
    /// Shared runtime state.
    pub(crate) sync: Arc<AudioStreamSync>,
    /// Bound stream handle once this host state is inserted into the resource table.
    pub(crate) stream_handle_raw: AtomicU64,
    /// Event-runtime owner used by cross-thread native event publishing.
    pub(crate) runtime_owner: Mutex<Option<Weak<AudioRuntimeState>>>,
    /// Optional backend worker thread for this stream.
    pub(crate) worker_thread: Mutex<Option<JoinHandle<()>>>,
}

impl std::fmt::Debug for AudioStreamHostState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioStreamHostState")
            .field("device_id", &self.device.id)
            .field("direction", &self.direction)
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("period_frames", &self.period_frames)
            .field("max_queued_frames", &self.max_queued_frames)
            .field("share_mode", &self.share_mode)
            .finish()
    }
}

impl AudioStreamHostState {
    /// Bind one stream handle and runtime owner to this host state.
    pub(crate) fn bind_runtime(
        self: &Arc<Self>,
        runtime_state: &Arc<AudioRuntimeState>,
        handle: resource::AudioStreamHandle,
    ) {
        self.stream_handle_raw
            .store(handle.0.local_id, Ordering::Release);

        let mut owner = self
            .runtime_owner
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *owner = Some(Arc::downgrade(runtime_state));
    }

    /// Unbind the stream handle and runtime owner from this host state.
    pub(crate) fn unbind_runtime(&self) {
        self.stream_handle_raw.store(0, Ordering::Release);

        let mut owner = self
            .runtime_owner
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *owner = None;
    }

    /// Return queued playback samples capacity.
    pub(crate) fn playback_capacity_samples(&self) -> usize {
        self.max_queued_frames
            .saturating_mul(self.channels as usize)
    }

    /// Return queued capture samples capacity.
    pub(crate) fn capture_capacity_samples(&self) -> usize {
        self.max_queued_frames
            .saturating_mul(self.channels as usize)
    }

    /// Return the bound stream handle when this host state has been registered.
    pub(crate) fn stream_handle(&self) -> Option<resource::AudioStreamHandle> {
        let raw = self.stream_handle_raw.load(Ordering::Acquire);
        if raw == 0 {
            return None;
        }

        let owner = self
            .runtime_owner
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let worker_id = owner.as_ref()?.upgrade()?.worker_id;
        let resource_id = resource::ResourceId::new(worker_id, raw);

        Some(resource::AudioStreamHandle(resource_id))
    }

    /// Return one buffered playback frame count.
    pub(crate) fn buffered_playback_frames(&self, state: &AudioStreamStateInner) -> u64 {
        (state.playback_samples.len() / self.channels as usize) as u64
    }

    /// Return one buffered capture frame count.
    pub(crate) fn buffered_capture_frames(&self, state: &AudioStreamStateInner) -> u64 {
        (state.capture_samples.len() / self.channels as usize) as u64
    }

    /// Shut down stream resources.
    pub(crate) fn shutdown(&self) {
        let mut state = self
            .sync
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.shutdown = true;
        state.running = false;
        state.state_kind = AudioStreamStateKind::Stopped;
        drop(state);
        self.sync.wake.notify_all();

        if let Some(worker) = self
            .worker_thread
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            let _ = worker.join();
        }
    }
}

impl Drop for AudioStreamHostState {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Resource finalizer that shuts down one host audio stream.
pub(crate) struct AudioStreamFinalizer {
    /// Stream owned by the resource entry.
    stream: Arc<AudioStreamHostState>,
}

impl AudioStreamFinalizer {
    /// Build one audio stream finalizer from one host stream state.
    pub(crate) fn new(stream: Arc<AudioStreamHostState>) -> Self {
        Self { stream }
    }
}

impl ResourceFinalizer for AudioStreamFinalizer {
    fn finalize(self: Box<Self>, _resource_id: ResourceId) {
        self.stream.shutdown();
    }
}

/// Opened audio event stream payload.
#[derive(Debug)]
pub(crate) struct AudioEventStream {
    /// Stable runtime stream identifier.
    pub(crate) stream_id: u64,
    /// Subscription options for this stream.
    pub(crate) options: AudioEventSubscriptionOptions,
    /// Mutable delivery frontier and overflow state.
    pub(crate) state: Mutex<AudioEventStreamState>,
}

/// Mutable audio event stream state.
#[derive(Debug)]
pub(crate) struct AudioEventStreamState {
    /// Queue capacity for unread live events.
    pub(crate) queue_capacity: usize,
    /// Overflow policy for unread live events.
    pub(crate) overflow_policy: AudioEventOverflowPolicy,
    /// Pending overflow error flag for error overflow policy.
    pub(crate) overflow_error_pending: bool,
    /// Next output sequence number for this stream.
    pub(crate) next_output_sequence: u64,
    /// Next live sequence to scan in the shared runtime event log.
    pub(crate) next_live_sequence: u64,
    /// Number of unread live records visible to this stream.
    pub(crate) unread_live_count: usize,
    /// Total dropped event count for this stream.
    pub(crate) dropped_count: u64,
    /// Registered synthetic stream poll callback.
    pub(crate) poll_callback: Option<WorkerCallbackHandle>,
}

/// Stored audio event record payload.
#[derive(Debug, Clone)]
pub(crate) struct AudioEventRecord {
    /// Event kind selector.
    pub(crate) kind: AudioEventKind,
    /// Monotonic timestamp.
    pub(crate) timestamp_ns: u64,
    /// Queue sequence number.
    pub(crate) sequence: u64,
    /// Total dropped event count before this record.
    pub(crate) dropped_count: u64,
    /// Event source.
    pub(crate) source: AudioEventSource,
    /// Backend selector for this event.
    pub(crate) backend: AudioBackend,
    /// Device identifier.
    pub(crate) device_id: String,
    /// Event flags.
    pub(crate) flags: u32,
    /// Stream status flags.
    pub(crate) status_flags: AudioStreamStatusFlags,
    /// Xrun delta for stream-related events.
    pub(crate) xrun_count_delta: u64,
    /// Stream target when available.
    pub(crate) stream: Option<resource::AudioStreamHandle>,
}

/// Runtime-owned device-monitor baseline for one backend.
#[derive(Debug)]
pub(crate) struct AudioDeviceMonitorBaseline {
    /// Subscription options for this monitor.
    pub(crate) previous_signatures: std::collections::HashMap<String, u64>,
    /// Previous default playback id.
    pub(crate) previous_default_playback: Option<String>,
    /// Previous default capture id.
    pub(crate) previous_default_capture: Option<String>,
    /// Previous default loopback id.
    pub(crate) previous_default_loopback: Option<String>,
    /// Last poll refresh timestamp in nanoseconds.
    pub(crate) last_refresh_ns: u64,
}

/// Runtime-owned stream-monitor baseline for one audio stream.
#[derive(Debug)]
pub(crate) struct AudioStreamMonitorBaseline {
    /// Previous stream state for optional stream tracking.
    pub(crate) previous_stream_state: Option<AudioStreamStateKind>,
    /// Previous stream device id for optional stream tracking.
    pub(crate) previous_stream_device_id: Option<String>,
    /// Previous stream xrun counter for optional stream tracking.
    pub(crate) previous_stream_xrun_count: u64,
    /// Last poll refresh timestamp in nanoseconds.
    pub(crate) last_refresh_ns: u64,
}

/// Build one stream event monitor state.
pub(crate) fn initial_audio_event_stream_state(
    queue_capacity: usize,
    overflow_policy: AudioEventOverflowPolicy,
    next_live_sequence: u64,
) -> AudioEventStreamState {
    AudioEventStreamState {
        queue_capacity,
        overflow_policy,
        overflow_error_pending: false,
        next_output_sequence: 1,
        next_live_sequence,
        unread_live_count: 0,
        dropped_count: 0,
        poll_callback: None,
    }
}

/// Build one backend device-monitor baseline from one snapshot.
pub(crate) fn audio_device_monitor_baseline(
    previous_signatures: std::collections::HashMap<String, u64>,
    previous_default_playback: Option<String>,
    previous_default_capture: Option<String>,
    previous_default_loopback: Option<String>,
    last_refresh_ns: u64,
) -> AudioDeviceMonitorBaseline {
    AudioDeviceMonitorBaseline {
        previous_signatures,
        previous_default_playback,
        previous_default_capture,
        previous_default_loopback,
        last_refresh_ns,
    }
}

/// Build one stream-monitor baseline from one captured stream state.
pub(crate) fn audio_stream_monitor_baseline(
    previous_stream_state: AudioStreamStateKind,
    previous_stream_device_id: String,
    previous_stream_xrun_count: u64,
    last_refresh_ns: u64,
) -> AudioStreamMonitorBaseline {
    AudioStreamMonitorBaseline {
        previous_stream_state: Some(previous_stream_state),
        previous_stream_device_id: Some(previous_stream_device_id),
        previous_stream_xrun_count,
        last_refresh_ns,
    }
}

/// Build one initial stream state payload for newly opened streams.
pub(crate) fn initial_stream_state() -> AudioStreamStateInner {
    AudioStreamStateInner {
        state_kind: AudioStreamStateKind::Stopped,
        running: false,
        paused: false,
        shutdown: false,
        playback_samples: VecDeque::new(),
        capture_samples: VecDeque::new(),
        stream_frames: 0,
        xrun_count: 0,
        input_underflow_count: 0,
        input_overflow_count: 0,
        output_underflow_count: 0,
        output_overflow_count: 0,
        status_flags: AudioStreamStatusFlags(0),
        last_callback_mono_ns: host_monotonic_nanos(),
        first_callback_mono_ns: 0,
        first_callback_stream_frames: 0,
        last_period_jitter_ns: 0,
        last_input_adc_ns: 0,
        last_output_dac_ns: 0,
        last_callback_cpu_load: 0.0,
        volume: DEFAULT_STREAM_VOLUME,
        muted: false,
        last_backend_message: None,
    }
}
