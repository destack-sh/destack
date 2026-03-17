use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, Weak};
use std::time::Duration;

use parking_lot::{Condvar, Mutex, RwLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::{HostEventObserver, HostQueue, HostRuntimeRegistry};
use crate::host::{
    HostEvent, HostIntentEvent, HostIntentPayload, HostLifecycleState, HostMemoryPressureLevel,
    HostPowerMode, HostRequest,
};
use crate::platform::core::{invalid_argument, io_would_block, monotonic_now_ns};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{
    IntentOpenOptions, LifecycleBackgroundEventValue, LifecycleEventMetadata, LifecycleEventValue,
    LifecycleForegroundEventValue, LifecycleLaunchEventValue, LifecycleLowMemoryEventValue,
    LifecycleLowMemoryPayload, LifecycleLowPowerModeChangedEventValue, LifecycleLowPowerPayload,
    LifecyclePauseEventValue, LifecycleResumeEventValue, LifecycleState,
    LifecycleTerminateEventValue, NetworkEvent, NetworkState, NotificationPermissionState,
    Permission, PermissionEntry, PermissionState,
};
use crate::platform::resource::{self, ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, fs};
use crate::runtime::BindingCallContext;
use crate::runtime::process::RuntimeScheduledCallbackHandle;

/// Maximum wait slice used while a lifecycle read services runtime ingress.
const LIFECYCLE_WAIT_SLICE_NS: u64 = 10_000_000;
/// Maximum wait slice used while an intent read services runtime ingress.
const INTENT_WAIT_SLICE_NS: u64 = 10_000_000;

/// One-time successful host queue bootstrap for OS runtime state.
#[derive(Debug, Default)]
struct HostQueueBootstrap {
    /// Successful bootstrap marker.
    is_complete: OnceLock<()>,
    /// Bootstrap serialization for one runtime state.
    lock: Mutex<()>,
}

/// Runtime-owned OS event state.
#[derive(Debug)]
pub(crate) struct OsRuntimeState {
    /// Current lifecycle state for this runtime.
    lifecycle_state: RwLock<LifecycleState>,
    /// Last host lifecycle state observed for this runtime.
    host_lifecycle_state: RwLock<HostLifecycleState>,
    /// One-time host queue bootstrap state.
    host_queue_bootstrap: HostQueueBootstrap,
    /// Last observed permission states for this runtime.
    permission_states: RwLock<HashMap<Permission, PermissionState>>,
    /// Open intent event streams.
    intent_streams: Mutex<Vec<Weak<IntentEventStream>>>,
    /// Open lifecycle event streams.
    lifecycle_streams: Mutex<Vec<Weak<LifecycleEventStream>>>,
    /// Open network watch streams.
    network_watch_streams: Mutex<Vec<Weak<NetworkWatchStream>>>,
    /// Active network poll callback handle when one is registered.
    network_watch_callback: Mutex<Option<RuntimeScheduledCallbackHandle>>,
}

/// One queued intent event inside runtime-owned stream state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntentQueuedEvent {
    /// Host source package, bundle, or process identifier when available.
    pub(crate) source: Option<String>,
    /// Intent payload delivered by the host.
    pub(crate) payload: IntentQueuedPayload,
}

/// One queued intent payload inside runtime-owned stream state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IntentQueuedPayload {
    /// Open-url payload.
    OpenUrl {
        /// URL payload from the host.
        url: String,
    },
    /// Open-file payload.
    OpenFile {
        /// Path payload from the host.
        path: String,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
    /// Share-text payload.
    ShareText {
        /// Shared text payload from the host.
        text: String,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
    /// Share-files payload.
    ShareFiles {
        /// Shared file path payloads from the host.
        paths: Vec<String>,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
    /// Custom-action payload.
    CustomAction {
        /// Action identifier from the host.
        action: String,
        /// URL payload when provided by the host.
        url: Option<String>,
        /// File path payloads when provided by the host.
        paths: Vec<String>,
        /// Shared text payload when provided by the host.
        text: Option<String>,
        /// MIME type when provided by the host.
        mime_type: Option<String>,
    },
}

/// Runtime-owned intent event stream.
#[derive(Debug)]
pub(crate) struct IntentEventStream {
    /// Open options that filter which events are visible to this stream.
    options: IntentOpenOptions,
    /// Pending intent events for this stream.
    events: Mutex<VecDeque<IntentQueuedEvent>>,
    /// Wake primitive for blocking reads.
    wake: Condvar,
    /// Whether this stream has been closed.
    is_closed: AtomicBool,
    /// Sequence number for the next event in this stream.
    next_sequence: AtomicU64,
}

/// Runtime-owned lifecycle event stream.
#[derive(Debug, Default)]
pub(crate) struct LifecycleEventStream {
    /// Pending lifecycle events for this stream.
    events: Mutex<VecDeque<LifecycleEventValue>>,
    /// Wake primitive for blocking reads.
    wake: Condvar,
    /// Whether this stream has been closed.
    is_closed: AtomicBool,
    /// Sequence number for the next event in this stream.
    next_sequence: AtomicU64,
}

/// Runtime-owned network watch stream.
#[derive(Debug)]
pub(crate) struct NetworkWatchStream {
    /// Last emitted network snapshot.
    last_state: Mutex<NetworkState>,
    /// Pending network events for this stream.
    events: Mutex<VecDeque<NetworkEvent>>,
    /// Wake primitive for blocking reads.
    wake: Condvar,
    /// Whether this stream has been closed.
    is_closed: AtomicBool,
    /// Sequence number for the next event in this stream.
    next_sequence: AtomicU64,
}

impl OsRuntimeState {
    /// Create one empty OS runtime state.
    pub(crate) fn new() -> Self {
        Self {
            lifecycle_state: RwLock::new(LifecycleState::Inactive),
            host_lifecycle_state: RwLock::new(HostLifecycleState::Initializing),
            host_queue_bootstrap: HostQueueBootstrap::default(),
            permission_states: RwLock::new(HashMap::new()),
            intent_streams: Mutex::new(Vec::new()),
            lifecycle_streams: Mutex::new(Vec::new()),
            network_watch_streams: Mutex::new(Vec::new()),
            network_watch_callback: Mutex::new(None),
        }
    }

    /// Return the current lifecycle state.
    pub(crate) fn lifecycle_state(&self) -> LifecycleState {
        *self.lifecycle_state.read()
    }

    /// Register one lifecycle stream.
    pub(crate) fn register_lifecycle_stream(&self, stream: Arc<LifecycleEventStream>) {
        let mut lifecycle_streams = self.lifecycle_streams.lock();
        lifecycle_streams.push(Arc::downgrade(&stream));
    }

    /// Register one intent stream.
    pub(crate) fn register_intent_stream(&self, stream: Arc<IntentEventStream>) {
        let mut intent_streams = self.intent_streams.lock();
        intent_streams.push(Arc::downgrade(&stream));
    }

    /// Register one network watch stream.
    pub(crate) fn register_network_watch_stream(&self, stream: Arc<NetworkWatchStream>) {
        let mut network_watch_streams = self.network_watch_streams.lock();
        network_watch_streams.push(Arc::downgrade(&stream));
    }

    /// Resolve every live network watch stream and prune dead entries.
    pub(crate) fn network_watch_streams(&self) -> Vec<Arc<NetworkWatchStream>> {
        let mut network_watch_streams = self.network_watch_streams.lock();
        let mut resolved_streams = Vec::new();

        network_watch_streams.retain(|stream| {
            let Some(stream) = stream.upgrade() else {
                return false;
            };

            resolved_streams.push(stream);
            true
        });

        resolved_streams
    }

    /// Return the active network watch callback handle when one exists.
    pub(crate) fn network_watch_callback(&self) -> Option<RuntimeScheduledCallbackHandle> {
        *self.network_watch_callback.lock()
    }

    /// Store one network watch callback handle.
    pub(crate) fn set_network_watch_callback(&self, handle: RuntimeScheduledCallbackHandle) {
        *self.network_watch_callback.lock() = Some(handle);
    }

    /// Clear one registered network watch callback handle.
    pub(crate) fn clear_network_watch_callback(&self, handle: RuntimeScheduledCallbackHandle) {
        let mut callback = self.network_watch_callback.lock();
        if callback.is_some_and(|current| current == handle) {
            *callback = None;
        }
    }

    /// Reconcile current lifecycle state from already queued host events.
    pub(crate) fn reconcile_host_queue(
        &self,
        queue: &HostQueue,
        observer: &Arc<dyn HostEventObserver>,
    ) -> RuntimeResult<()> {
        self.host_queue_bootstrap.ensure_complete(|| {
            // register and snapshot under one queue lock so live host delivery cannot
            // race one bootstrap replay
            for event in queue.register_observer_and_snapshot(observer) {
                self.observe_host_event(&event)?;
            }

            Ok(())
        })
    }

    /// Push one lifecycle event to every live stream.
    fn publish_lifecycle_event(
        &self,
        build: impl Fn(&LifecycleEventStream) -> LifecycleEventValue,
    ) {
        let lifecycle_streams = {
            let mut lifecycle_streams = self.lifecycle_streams.lock();
            let mut resolved_streams = Vec::new();

            lifecycle_streams.retain(|stream| {
                let Some(stream) = stream.upgrade() else {
                    return false;
                };

                resolved_streams.push(stream);
                true
            });

            resolved_streams
        };

        for stream in lifecycle_streams {
            stream.push_event(build(&stream));
        }
    }

    /// Push one intent event to every interested live stream.
    fn publish_intent_event(&self, event: &IntentQueuedEvent) {
        let intent_streams = {
            let mut intent_streams = self.intent_streams.lock();
            let mut resolved_streams = Vec::new();

            intent_streams.retain(|stream| {
                let Some(stream) = stream.upgrade() else {
                    return false;
                };

                resolved_streams.push(stream);
                true
            });

            resolved_streams
        };

        for stream in intent_streams {
            if !stream_accepts_payload(stream.options(), &event.payload) {
                continue;
            }

            stream.push_event(event.clone());
        }
    }

    /// Apply one host lifecycle transition.
    fn observe_lifecycle_transition(&self, state: HostLifecycleState) {
        let previous_host_state = *self.host_lifecycle_state.read();
        let next_state = lifecycle_state_from_host(state);

        if previous_host_state == state {
            return;
        }

        {
            let mut lifecycle_state = self.lifecycle_state.write();
            *lifecycle_state = next_state;
        }

        {
            let mut host_lifecycle_state = self.host_lifecycle_state.write();
            *host_lifecycle_state = state;
        }

        let Some(kind) = lifecycle_event_kind_for_transition(previous_host_state, state) else {
            return;
        };

        self.publish_lifecycle_event(|stream| {
            lifecycle_event_for_kind(kind, stream.next_metadata())
        });
    }

    /// Apply one host memory-pressure event.
    fn observe_memory_pressure(&self, level: HostMemoryPressureLevel) {
        let severity = match level {
            HostMemoryPressureLevel::Normal => return,
            HostMemoryPressureLevel::Warning => 1,
            HostMemoryPressureLevel::Critical => 2,
        };

        self.publish_lifecycle_event(|stream| {
            LifecycleEventValue::LifecycleLowMemoryEvent(LifecycleLowMemoryEventValue {
                kind: "lowMemory".to_string(),
                metadata: stream.next_metadata(),
                payload: LifecycleLowMemoryPayload { severity },
            })
        });
    }

    /// Apply one host power-mode event.
    fn observe_power_mode(&self, mode: HostPowerMode) {
        let enabled = matches!(mode, HostPowerMode::LowPower);

        self.publish_lifecycle_event(|stream| {
            LifecycleEventValue::LifecycleLowPowerModeChangedEvent(
                LifecycleLowPowerModeChangedEventValue {
                    kind: "lowPowerModeChanged".to_string(),
                    metadata: stream.next_metadata(),
                    payload: LifecycleLowPowerPayload { enabled },
                },
            )
        });
    }

    /// Apply one host permission result.
    fn observe_permission(&self, permission: &str, granted: bool) {
        let Some(permission) = parse_host_permission_name(permission) else {
            return;
        };

        let state = if granted {
            PermissionState::Granted
        } else {
            PermissionState::Denied
        };

        let mut permission_states = self.permission_states.write();
        permission_states.insert(permission, state);
    }

    /// Read one cached permission state.
    fn permission_state(&self, permission: Permission) -> RuntimeResult<PermissionState> {
        let permission_states = self.permission_states.read();
        let Some(state) = permission_states.get(&permission).copied() else {
            return Err(invalid_data(
                "destack.os.permission.state",
                "permission state is not yet known for this runtime",
            ));
        };

        Ok(state)
    }

    /// Read multiple cached permission states.
    fn permission_state_many(
        &self,
        permissions: &[Permission],
    ) -> RuntimeResult<Vec<PermissionEntry>> {
        let mut entries = Vec::with_capacity(permissions.len());

        for permission in permissions {
            let state = self.permission_state(*permission)?;
            entries.push(PermissionEntry {
                permission: *permission,
                state,
            });
        }

        Ok(entries)
    }

    /// Apply one host intent event.
    fn observe_intent_event(&self, event: &HostIntentEvent) {
        let queued_event = IntentQueuedEvent {
            source: event.source.clone(),
            payload: intent_payload_from_host(&event.payload),
        };

        self.publish_intent_event(&queued_event);
    }
}

impl HostQueueBootstrap {
    /// Run one fallible bootstrap exactly once after successful completion.
    fn ensure_complete(&self, bootstrap: impl FnOnce() -> RuntimeResult<()>) -> RuntimeResult<()> {
        if self.is_complete.get().is_some() {
            return Ok(());
        }

        // serialize retries before the first successful completion
        let _lock = self.lock.lock();
        if self.is_complete.get().is_some() {
            return Ok(());
        }

        bootstrap()?;
        let _ = self.is_complete.set(());

        Ok(())
    }
}

impl HostEventObserver for OsRuntimeState {
    /// Observe one host event and update OS runtime state.
    fn observe_host_event(&self, event: &HostEvent) -> RuntimeResult<()> {
        // lifecycle streams only depend on lifecycle, memory, and power signals
        match event {
            HostEvent::Lifecycle(event) => self.observe_lifecycle_transition(event.state),
            HostEvent::Intent(event) => self.observe_intent_event(event),
            HostEvent::Permission(event) => {
                self.observe_permission(event.permission.as_str(), event.granted)
            }
            HostEvent::MemoryPressure(event) => self.observe_memory_pressure(event.level),
            HostEvent::PowerMode(event) => self.observe_power_mode(event.mode),
            _ => {}
        }

        Ok(())
    }
}

impl IntentEventStream {
    /// Create one empty intent event stream.
    fn new(options: IntentOpenOptions) -> Self {
        Self {
            options,
            events: Mutex::new(VecDeque::new()),
            wake: Condvar::new(),
            is_closed: AtomicBool::new(false),
            next_sequence: AtomicU64::new(0),
        }
    }

    /// Return the open options for this stream.
    fn options(&self) -> IntentOpenOptions {
        self.options
    }

    /// Push one event and wake blocked readers.
    fn push_event(&self, event: IntentQueuedEvent) {
        if self.is_closed.load(Ordering::Relaxed) {
            return;
        }

        let mut events = self.events.lock();

        events.push_back(event);
        self.wake.notify_all();
    }

    /// Try to take one queued event.
    fn try_take(&self) -> Option<IntentQueuedEvent> {
        let mut events = self.events.lock();

        events.pop_front()
    }

    /// Return whether this stream is closed.
    fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Close this stream and wake blocked readers.
    fn close(&self) {
        self.is_closed.store(true, Ordering::Relaxed);
        self.wake.notify_all();
    }

    /// Wait once for queued events or timeout.
    fn wait_once(&self, duration: Duration) {
        let mut events = self.events.lock();

        if events.is_empty() {
            self.wake.wait_for(&mut events, duration);
        }
    }

    /// Return the next sequence number for this stream.
    pub(crate) fn next_sequence(&self) -> u64 {
        self.next_sequence.fetch_add(1, Ordering::Relaxed)
    }
}

impl LifecycleEventStream {
    /// Return the next metadata payload for this stream.
    fn next_metadata(&self) -> LifecycleEventMetadata {
        let timestamp_ns = monotonic_now_ns();
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);

        LifecycleEventMetadata {
            timestamp_ns,
            sequence,
        }
    }

    /// Push one event and wake blocked readers.
    fn push_event(&self, event: LifecycleEventValue) {
        if self.is_closed.load(Ordering::Relaxed) {
            return;
        }

        let mut events = self.events.lock();

        events.push_back(event);
        self.wake.notify_all();
    }

    /// Try to take one queued event.
    fn try_take(&self) -> Option<LifecycleEventValue> {
        let mut events = self.events.lock();

        events.pop_front()
    }

    /// Return whether this stream is closed.
    fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Close this stream and wake blocked readers.
    fn close(&self) {
        self.is_closed.store(true, Ordering::Relaxed);
        self.wake.notify_all();
    }

    /// Wait once for queued events or timeout.
    fn wait_once(&self, duration: Duration) {
        let mut events = self.events.lock();

        if events.is_empty() {
            self.wake.wait_for(&mut events, duration);
        }
    }
}

impl NetworkWatchStream {
    /// Create one network watch stream seeded from the current snapshot.
    pub(crate) fn new(initial_state: NetworkState) -> Self {
        Self {
            last_state: Mutex::new(initial_state),
            events: Mutex::new(VecDeque::new()),
            wake: Condvar::new(),
            is_closed: AtomicBool::new(false),
            next_sequence: AtomicU64::new(0),
        }
    }

    /// Publish one changed network snapshot to this stream.
    pub(crate) fn publish_state(&self, next_state: NetworkState) {
        if self.is_closed() {
            return;
        }

        let mut last_state = self.last_state.lock();
        if *last_state == next_state {
            return;
        }

        let timestamp_ns = monotonic_now_ns();
        let sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        *last_state = next_state;

        let mut events = self.events.lock();
        events.push_back(NetworkEvent {
            timestamp_ns,
            sequence,
            state: next_state,
        });
        self.wake.notify_all();
    }

    /// Try to take one queued network event.
    pub(crate) fn try_take(&self) -> Option<NetworkEvent> {
        let mut events = self.events.lock();

        events.pop_front()
    }

    /// Return whether this stream is closed.
    pub(crate) fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Relaxed)
    }

    /// Close this stream and wake blocked readers.
    pub(crate) fn close(&self) {
        self.is_closed.store(true, Ordering::Relaxed);
        self.wake.notify_all();
    }

    /// Wait once for queued events or timeout.
    pub(crate) fn wait_once(&self, duration: Duration) {
        let mut events = self.events.lock();

        if events.is_empty() {
            self.wake.wait_for(&mut events, duration);
        }
    }
}

/// Return the runtime-owned OS state for this binding.
pub(crate) fn runtime_state(binding: &BindingCallContext) -> RuntimeResult<Arc<OsRuntimeState>> {
    let runtime_state = binding
        .agent()
        .platform_state
        .os
        .runtime_state(OsRuntimeState::new);
    let observer: Arc<dyn HostEventObserver> = runtime_state.clone();
    let runtime_id = binding.host().runtime_id();
    let platform = binding.host().platform();
    let queue = HostRuntimeRegistry::queue_for_runtime(runtime_id, platform);

    // lifecycle state can still function without queue catchup if callback routing
    // is not registered yet, or if tests are dispatching host events directly
    match queue {
        Ok(queue) => {
            runtime_state.reconcile_host_queue(&queue, &observer)?;
        }
        Err(error) => {
            let is_missing_queue = matches!(
                error.as_ref(),
                RuntimeError::Platform(platform_error)
                    if platform_error.code == PlatformErrorCode::NotSupported
                        && platform_error
                            .context
                            .as_ref()
                            .and_then(|context| context.feature.as_deref())
                            .is_some_and(|feature| feature.starts_with("runtime.host.queue."))
            );

            if !is_missing_queue {
                return Err(error);
            }

            HostRuntimeRegistry::register_host_event_observer(runtime_id, &observer);
        }
    }

    Ok(runtime_state)
}

/// Read the current lifecycle state from runtime-owned OS state.
pub(crate) fn lifecycle_state(binding: &BindingCallContext) -> RuntimeResult<LifecycleState> {
    // last observed lifecycle state
    //
    // state reads should be side-effect free and return the runtime-owned view that
    // has already been observed through the host callback bridge or event stream
    let runtime_state = runtime_state(binding)?;

    Ok(runtime_state.lifecycle_state())
}

/// Open one lifecycle event stream.
pub(crate) fn lifecycle_open(
    binding: &BindingCallContext,
) -> RuntimeResult<resource::LifecycleEventHandle> {
    let runtime_state = runtime_state(binding)?;
    let stream = Arc::new(LifecycleEventStream::default());

    // stream registration comes before handle publication
    runtime_state.register_lifecycle_stream(Arc::clone(&stream));

    let entry = ResourceEntry::new(ResourceKind::LifecycleEvent)
        .with_label("os.lifecycle.event")
        .with_payload(stream);

    let handle = binding
        .agent()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::LifecycleEventHandle(handle))
}

/// Open one intent event stream.
pub(crate) fn intent_open(
    binding: &BindingCallContext,
    options: IntentOpenOptions,
) -> RuntimeResult<resource::IntentHandle> {
    let runtime_state = runtime_state(binding)?;
    let stream = Arc::new(IntentEventStream::new(options));

    // stream registration comes before handle publication
    runtime_state.register_intent_stream(Arc::clone(&stream));

    let entry = ResourceEntry::new(ResourceKind::Intent)
        .with_label("os.intent")
        .with_payload(stream);

    let handle = binding
        .agent()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::IntentHandle(handle))
}

/// Close one intent event stream.
pub(crate) fn intent_close(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    let removed =
        binding
            .agent()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_intent_handle());
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_intent_handle());
    };

    let Ok(stream) = payload.downcast::<Arc<IntentEventStream>>() else {
        return Err(invalid_intent_handle());
    };

    stream.close();

    Ok(())
}

/// Poll one intent event without blocking.
pub(crate) fn intent_try_read(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<(Arc<IntentEventStream>, IntentQueuedEvent)> {
    binding.service_runtime_ingress()?;

    let stream = resolve_intent_stream(binding, handle)?;
    let Some(event) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.intent.tryRead",
            "no intent event is currently queued",
        ));
    };

    Ok((stream, event))
}

/// Wait for one intent event.
pub(crate) fn intent_read(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
    timeout_ns: u64,
) -> RuntimeResult<(Arc<IntentEventStream>, IntentQueuedEvent)> {
    let stream = resolve_intent_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    let event = binding.wait_for_binding_result(
        "destack.os.intent.read",
        "timed out waiting for intent event",
        deadline_ns,
        INTENT_WAIT_SLICE_NS,
        || {
            if stream.is_closed() {
                return Err(invalid_intent_handle());
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )?;

    Ok((stream, event))
}

/// Close one lifecycle event stream.
pub(crate) fn lifecycle_close(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    let removed =
        binding
            .agent()
            .resources
            .remove(binding.world(), handle.0, Some(binding.engine()));

    let Some(entry) = removed else {
        return Err(invalid_lifecycle_handle());
    };

    let Some(payload) = entry.payload else {
        return Err(invalid_lifecycle_handle());
    };

    let Ok(stream) = payload.downcast::<Arc<LifecycleEventStream>>() else {
        return Err(invalid_lifecycle_handle());
    };

    stream.close();

    Ok(())
}

/// Poll one lifecycle event without blocking.
pub(crate) fn lifecycle_try_read(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<LifecycleEventValue> {
    binding.service_runtime_ingress()?;

    let stream = resolve_lifecycle_stream(binding, handle)?;
    let Some(event) = stream.try_take() else {
        return Err(io_would_block(
            "destack.os.lifecycle.tryRead",
            "no lifecycle event is currently queued",
        ));
    };

    Ok(event)
}

/// Wait for one lifecycle event.
pub(crate) fn lifecycle_read(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<LifecycleEventValue> {
    let stream = resolve_lifecycle_stream(binding, handle)?;
    let now = monotonic_now_ns();
    let deadline_ns = now.saturating_add(timeout_ns);

    binding.wait_for_binding_result(
        "destack.os.lifecycle.read",
        "timed out waiting for lifecycle event",
        deadline_ns,
        LIFECYCLE_WAIT_SLICE_NS,
        || {
            if stream.is_closed() {
                return Err(invalid_lifecycle_handle());
            }

            Ok(stream.try_take())
        },
        |duration| stream.wait_once(duration),
    )
}

/// Read one runtime-owned permission state.
pub(crate) fn permission_state(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    let runtime_state = runtime_state(binding)?;

    runtime_state.permission_state(permission)
}

/// Read multiple runtime-owned permission states.
pub(crate) fn permission_state_many(
    binding: &BindingCallContext,
    permissions: &[Permission],
) -> RuntimeResult<Vec<PermissionEntry>> {
    let runtime_state = runtime_state(binding)?;

    runtime_state.permission_state_many(permissions)
}

/// Read the runtime-owned notification permission state.
pub(crate) fn notification_permission_state(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    let permission_state = permission_state(binding, Permission::Notifications)?;

    let notification_state = match permission_state {
        PermissionState::Granted => NotificationPermissionState::Granted,
        PermissionState::Denied | PermissionState::Restricted => {
            NotificationPermissionState::Denied
        }
        PermissionState::Prompt | PermissionState::Limited => NotificationPermissionState::Prompt,
    };

    Ok(notification_state)
}

/// Request host notification permission.
pub(crate) fn notification_request_permission(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    binding
        .host()
        .submit_request(HostRequest::OsNotificationRequestPermission)?
        .into_notification_permission_state("destack.os.notification.requestPermission")
}

/// Open host settings for runtime permissions.
pub(crate) fn permission_open_settings(binding: &BindingCallContext) -> RuntimeResult<()> {
    binding
        .host()
        .submit_request(HostRequest::OsPermissionOpenSettings)?;

    Ok(())
}

/// Request host authorization for one permission selector.
pub(crate) fn permission_request(
    binding: &BindingCallContext,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    binding
        .host()
        .submit_request(HostRequest::OsPermissionRequest { permission })?
        .into_permission_state("destack.os.permission.request")
}

/// Request host authorization for one permission selector list.
pub(crate) fn permission_request_many(
    binding: &BindingCallContext,
    permissions: Vec<Permission>,
) -> RuntimeResult<Vec<PermissionEntry>> {
    binding
        .host()
        .submit_request(HostRequest::OsPermissionRequestMany { permissions })?
        .into_permission_entries("destack.os.permission.requestMany")
}

/// Resolve one lifecycle stream handle into its payload.
fn resolve_lifecycle_stream(
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<Arc<LifecycleEventStream>> {
    let resolved = binding.agent().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<LifecycleEventStream>>())
            .map(Arc::clone)
    });

    resolved.flatten().ok_or_else(invalid_lifecycle_handle)
}

/// Resolve one intent stream handle into its payload.
fn resolve_intent_stream(
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<Arc<IntentEventStream>> {
    let resolved = binding.agent().resources.with_entry(handle.0, |entry| {
        entry
            .payload
            .as_ref()
            .and_then(|payload| payload.downcast_ref::<Arc<IntentEventStream>>())
            .map(Arc::clone)
    });

    resolved.flatten().ok_or_else(invalid_intent_handle)
}

/// Build one invalid lifecycle handle error.
fn invalid_lifecycle_handle() -> Box<RuntimeError> {
    invalid_argument("handle", "unknown lifecycle event stream handle")
}

/// Build one invalid intent handle error.
fn invalid_intent_handle() -> Box<RuntimeError> {
    invalid_argument("handle", "unknown intent event stream handle")
}

/// Build one invalid-data runtime error.
fn invalid_data(operation: &'static str, detail: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        detail,
    ))
    .boxed()
}

/// Map one host lifecycle state to the public runtime lifecycle state.
fn lifecycle_state_from_host(state: HostLifecycleState) -> LifecycleState {
    match state {
        HostLifecycleState::Initializing => LifecycleState::Inactive,
        HostLifecycleState::Running => LifecycleState::Active,
        HostLifecycleState::Paused => LifecycleState::Inactive,
        HostLifecycleState::Stopped => LifecycleState::Background,
        HostLifecycleState::Destroyed => LifecycleState::Terminating,
    }
}

/// Return the lifecycle event kind for one host lifecycle transition.
fn lifecycle_event_kind_for_transition(
    previous: HostLifecycleState,
    next: HostLifecycleState,
) -> Option<&'static str> {
    match (previous, next) {
        (HostLifecycleState::Initializing, HostLifecycleState::Running) => Some("launch"),
        (HostLifecycleState::Paused, HostLifecycleState::Running) => Some("resume"),
        (HostLifecycleState::Stopped, HostLifecycleState::Running) => Some("foreground"),
        (HostLifecycleState::Running, HostLifecycleState::Paused) => Some("pause"),
        (HostLifecycleState::Running, HostLifecycleState::Stopped)
        | (HostLifecycleState::Paused, HostLifecycleState::Stopped) => Some("background"),
        (_, HostLifecycleState::Destroyed) => Some("terminate"),
        _ => None,
    }
}

/// Build one lifecycle event payload for one kind string.
fn lifecycle_event_for_kind(
    kind: &'static str,
    metadata: LifecycleEventMetadata,
) -> LifecycleEventValue {
    match kind {
        "launch" => LifecycleEventValue::LifecycleLaunchEvent(LifecycleLaunchEventValue {
            kind: "launch".to_string(),
            metadata,
        }),
        "resume" => LifecycleEventValue::LifecycleResumeEvent(LifecycleResumeEventValue {
            kind: "resume".to_string(),
            metadata,
        }),
        "pause" => LifecycleEventValue::LifecyclePauseEvent(LifecyclePauseEventValue {
            kind: "pause".to_string(),
            metadata,
        }),
        "background" => {
            LifecycleEventValue::LifecycleBackgroundEvent(LifecycleBackgroundEventValue {
                kind: "background".to_string(),
                metadata,
            })
        }
        "foreground" => {
            LifecycleEventValue::LifecycleForegroundEvent(LifecycleForegroundEventValue {
                kind: "foreground".to_string(),
                metadata,
            })
        }
        "terminate" => LifecycleEventValue::LifecycleTerminateEvent(LifecycleTerminateEventValue {
            kind: "terminate".to_string(),
            metadata,
        }),
        _ => unreachable!("unknown lifecycle event kind"),
    }
}

/// Return whether one stream accepts one queued intent payload.
fn stream_accepts_payload(options: IntentOpenOptions, payload: &IntentQueuedPayload) -> bool {
    match payload {
        IntentQueuedPayload::OpenUrl { .. } => options.include_open_url,
        IntentQueuedPayload::OpenFile { .. } => options.include_open_file,
        IntentQueuedPayload::ShareText { .. } | IntentQueuedPayload::ShareFiles { .. } => {
            options.include_share
        }
        IntentQueuedPayload::CustomAction { .. } => options.include_custom_action,
    }
}

/// Convert one host intent payload into runtime-owned queue data.
fn intent_payload_from_host(payload: &HostIntentPayload) -> IntentQueuedPayload {
    match payload {
        HostIntentPayload::OpenUrl { url } => IntentQueuedPayload::OpenUrl { url: url.clone() },
        HostIntentPayload::OpenFile { path, mime_type } => IntentQueuedPayload::OpenFile {
            path: path.clone(),
            mime_type: mime_type.clone(),
        },
        HostIntentPayload::ShareText { text, mime_type } => IntentQueuedPayload::ShareText {
            text: text.clone(),
            mime_type: mime_type.clone(),
        },
        HostIntentPayload::ShareFiles { paths, mime_type } => IntentQueuedPayload::ShareFiles {
            paths: paths.clone(),
            mime_type: mime_type.clone(),
        },
        HostIntentPayload::CustomAction {
            action,
            url,
            paths,
            text,
            mime_type,
        } => IntentQueuedPayload::CustomAction {
            action: action.clone(),
            url: url.clone(),
            paths: paths.clone(),
            text: text.clone(),
            mime_type: mime_type.clone(),
        },
    }
}

/// Encode one owned string path into a native os-path payload.
pub(crate) fn intent_path_from_utf8(binding: &BindingCallContext, value: String) -> fs::OsPath {
    fs::core::os_path_from_utf8_string(binding, value)
}

/// Normalize one host permission token into the public permission selector.
fn parse_host_permission_name(permission: &str) -> Option<Permission> {
    let canonical = permission
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect::<String>();

    match canonical.as_str() {
        "location" => Some(Permission::Location),
        "locationbackground" => Some(Permission::LocationBackground),
        "camera" => Some(Permission::Camera),
        "microphone" => Some(Permission::Microphone),
        "bluetooth" => Some(Permission::Bluetooth),
        "notifications" | "notification" => Some(Permission::Notifications),
        "contactsread" => Some(Permission::ContactsRead),
        "contactswrite" => Some(Permission::ContactsWrite),
        "mediaread" => Some(Permission::MediaRead),
        "mediawrite" => Some(Permission::MediaWrite),
        "motion" => Some(Permission::Motion),
        "clipboardread" => Some(Permission::ClipboardRead),
        "calendarread" => Some(Permission::CalendarRead),
        "calendarwrite" => Some(Permission::CalendarWrite),
        _ => None,
    }
}
