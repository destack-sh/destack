use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::{
    AudioBackend, AudioEventDeliveryMode, AudioEventSource, backend as audio_backend,
};
use crate::runtime::process::service::executor::periodic::{
    PeriodicTaskHandle, open_periodic_task,
};
use crate::runtime::process::{ExecutionMode, ExecutionPolicy, Service};
use crate::runtime::{ProcessSubscriberRegistry, WorkerId};

use super::constants::host_monotonic_nanos;
use super::event::publish::publish_device_events_from_snapshot;
use super::event::queue::event_streams_snapshot;
use super::event::snapshot::monitor_snapshot;
use super::runtime::{AudioRuntimeState, tracks_device_events};

/// Shared monitor handle owned by one process-global audio backend monitor.
pub(crate) trait AudioMonitorHandle: Send + Sync {
    /// Stop the backend monitor.
    fn stop(self: Box<Self>);
}

/// One synthetic monitor worker owned by the shared backend monitor.
struct SyntheticAudioMonitorHandle {
    /// Registered periodic task.
    task: PeriodicTaskHandle,
}

impl AudioMonitorHandle for SyntheticAudioMonitorHandle {
    fn stop(self: Box<Self>) {
        drop(self.task);
    }
}

/// One effective monitor demand summary for one backend.
#[derive(Clone, Copy, Debug, Default)]
struct AudioMonitorDemand {
    /// Whether at least one runtime still subscribes to this backend.
    has_subscribers: bool,
    /// Whether a native backend monitor should be enabled.
    wants_native_monitor: bool,
    /// Whether a synthetic polling worker should be enabled.
    wants_synthetic_worker: bool,
    /// The fastest synthetic polling interval requested by subscribers.
    synthetic_interval_ns: u64,
}

/// One process-global backend monitor state for one backend.
struct AudioBackendMonitorState {
    /// Registered worker runtimes keyed by owning worker id.
    subscribers: ProcessSubscriberRegistry<WorkerId, AudioRuntimeState>,
    /// Shared native backend monitor handle when the backend supports one.
    native_handle: Option<Box<dyn AudioMonitorHandle>>,
    /// Shared synthetic polling worker when one is required.
    synthetic_handle: Option<Box<dyn AudioMonitorHandle>>,
    /// The currently active synthetic polling interval.
    synthetic_interval_ns: u64,
}

/// One deferred backend monitor action.
enum AudioMonitorServiceAction {
    /// Start one native backend monitor.
    StartNative,
    /// Stop one running native backend monitor.
    StopNative(Box<dyn AudioMonitorHandle>),
    /// Start one synthetic polling worker.
    StartSynthetic(u64),
    /// Stop one running synthetic polling worker.
    StopSynthetic(Box<dyn AudioMonitorHandle>),
}

impl AudioBackendMonitorState {
    /// Build one empty backend monitor service.
    fn new() -> Self {
        Self {
            subscribers: ProcessSubscriberRegistry::default(),
            native_handle: None,
            synthetic_handle: None,
            synthetic_interval_ns: 0,
        }
    }

    /// Return one snapshot of the live runtime subscribers.
    fn live_runtime_states(&mut self) -> Vec<Arc<AudioRuntimeState>> {
        self.subscribers.snapshot()
    }
}

/// One process-global audio monitor service.
pub(crate) struct AudioMonitorService {
    /// Shared backend monitor services keyed by backend.
    backends: Mutex<HashMap<AudioBackend, AudioBackendMonitorState>>,
}

impl AudioMonitorService {
    /// Build one empty process-global audio monitor service.
    fn new() -> Self {
        Self {
            backends: Mutex::new(HashMap::new()),
        }
    }

    /// Refresh one runtime subscription for one backend.
    pub(crate) fn refresh_runtime(
        self: &Arc<Self>,
        runtime_state: &Arc<AudioRuntimeState>,
        backend: AudioBackend,
    ) -> RuntimeResult<()> {
        let mut pending_native_start = false;
        let mut pending_native_stop = None;
        let mut pending_synthetic_start = None;
        let mut pending_synthetic_stop = None;

        {
            let mut backends = self
                .backends
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let actions = self.refresh_runtime_locked(&mut backends, runtime_state, backend);

            for action in actions {
                match action {
                    // start outside the registry lock
                    AudioMonitorServiceAction::StartNative => {
                        pending_native_start = true;
                    }

                    // stop outside the registry lock
                    AudioMonitorServiceAction::StopNative(handle) => {
                        pending_native_stop = Some(handle);
                    }

                    // start outside the registry lock
                    AudioMonitorServiceAction::StartSynthetic(interval_ns) => {
                        pending_synthetic_start = Some(interval_ns);
                    }

                    // stop outside the registry lock
                    AudioMonitorServiceAction::StopSynthetic(handle) => {
                        pending_synthetic_stop = Some(handle);
                    }
                }
            }
        }

        // stop stale native monitor
        if let Some(handle) = pending_native_stop {
            handle.stop();
        }

        // stop stale synthetic worker
        if let Some(handle) = pending_synthetic_stop {
            handle.stop();
        }

        // start requested native monitor
        if pending_native_start {
            let handle = audio_backend::start_backend_native_device_events(backend)?;
            let mut backends = self
                .backends
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let Some(monitor) = backends.get_mut(&backend) else {
                handle.stop();
                return Ok(());
            };

            if monitor.native_handle.is_none() {
                monitor.native_handle = Some(handle);
            } else {
                handle.stop();
            }
        }

        // start requested synthetic worker
        if let Some(interval_ns) = pending_synthetic_start {
            let handle = start_synthetic_monitor_worker(Arc::clone(self), backend, interval_ns)?;
            let mut backends = self
                .backends
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let Some(monitor) = backends.get_mut(&backend) else {
                handle.stop();
                return Ok(());
            };

            if monitor.synthetic_handle.is_none() {
                monitor.synthetic_interval_ns = interval_ns;
                monitor.synthetic_handle = Some(handle);
            } else {
                handle.stop();
            }
        }

        Ok(())
    }

    /// Remove one runtime from all backend monitors.
    pub(crate) fn unregister_runtime(&self, runtime_state: &AudioRuntimeState) {
        let mut handles_to_stop = Vec::new();

        {
            let mut backends = self
                .backends
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let mut empty_backends = Vec::new();

            for (backend, monitor) in &mut *backends {
                monitor.subscribers.unregister(runtime_state.worker_id);
                let runtimes = monitor.live_runtime_states();
                let demand = aggregate_backend_monitor_demand(&runtimes, *backend);

                // detach stale native monitor
                if !demand.wants_native_monitor
                    && let Some(handle) = monitor.native_handle.take()
                {
                    handles_to_stop.push(handle);
                }

                // detach stale synthetic worker
                if !demand.wants_synthetic_worker
                    && let Some(handle) = monitor.synthetic_handle.take()
                {
                    monitor.synthetic_interval_ns = 0;
                    handles_to_stop.push(handle);
                }

                if monitor.subscribers.is_empty()
                    && monitor.native_handle.is_none()
                    && monitor.synthetic_handle.is_none()
                {
                    empty_backends.push(*backend);
                }
            }

            for backend in empty_backends {
                backends.remove(&backend);
            }
        }

        // stop detached monitors after the registry lock is released
        for handle in handles_to_stop {
            handle.stop();
        }
    }

    /// Publish one native backend snapshot to subscribed runtimes.
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    pub(crate) fn publish_native_snapshot(&self, backend: AudioBackend) {
        let now = host_monotonic_nanos();
        let snapshot = match monitor_snapshot(backend) {
            Ok(snapshot) => snapshot,
            Err(_) => return,
        };
        let runtimes = {
            let mut backends = self
                .backends
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let Some(monitor) = backends.get_mut(&backend) else {
                return;
            };

            monitor.live_runtime_states()
        };

        for runtime_state in &runtimes {
            publish_device_events_from_snapshot(
                runtime_state,
                backend,
                &snapshot,
                now,
                AudioEventSource::Native,
            );
        }
    }

    /// Refresh one runtime subscription while holding the registry lock.
    fn refresh_runtime_locked(
        &self,
        backends: &mut HashMap<AudioBackend, AudioBackendMonitorState>,
        runtime_state: &Arc<AudioRuntimeState>,
        backend: AudioBackend,
    ) -> Vec<AudioMonitorServiceAction> {
        let wants_backend = runtime_backend_monitor_demand(runtime_state, backend).has_subscribers;
        let monitor = backends
            .entry(backend)
            .or_insert_with(AudioBackendMonitorState::new);
        let mut actions = Vec::new();

        // keep only runtimes that still have matching subscriptions
        if wants_backend {
            monitor
                .subscribers
                .register(runtime_state.worker_id, runtime_state);
        } else {
            monitor.subscribers.unregister(runtime_state.worker_id);
        }

        let runtimes = monitor.live_runtime_states();
        let demand = aggregate_backend_monitor_demand(&runtimes, backend);

        // native backend monitor
        if demand.wants_native_monitor && monitor.native_handle.is_none() {
            actions.push(AudioMonitorServiceAction::StartNative);
        }

        // drop native backend monitor when no runtime needs it
        if !demand.wants_native_monitor
            && let Some(handle) = monitor.native_handle.take()
        {
            actions.push(AudioMonitorServiceAction::StopNative(handle));
        }

        // restart the synthetic worker when the required interval changes
        let restart_synthetic_worker = demand.wants_synthetic_worker
            && monitor.synthetic_handle.is_some()
            && monitor.synthetic_interval_ns != demand.synthetic_interval_ns;
        if restart_synthetic_worker && let Some(handle) = monitor.synthetic_handle.take() {
            actions.push(AudioMonitorServiceAction::StopSynthetic(handle));
        }

        // synthetic native-only polling worker
        if demand.wants_synthetic_worker && monitor.synthetic_handle.is_none() {
            actions.push(AudioMonitorServiceAction::StartSynthetic(
                demand.synthetic_interval_ns,
            ));
        }

        // stop synthetic polling when no longer needed
        if !demand.wants_synthetic_worker
            && let Some(handle) = monitor.synthetic_handle.take()
        {
            monitor.synthetic_interval_ns = 0;
            actions.push(AudioMonitorServiceAction::StopSynthetic(handle));
        }

        if monitor.subscribers.is_empty()
            && monitor.native_handle.is_none()
            && monitor.synthetic_handle.is_none()
        {
            backends.remove(&backend);
        }

        actions
    }
}

impl Service for AudioMonitorService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::global(ExecutionMode::Inline);
}

/// Return one shared process-global audio monitor service.
pub(crate) fn audio_monitor_service() -> Arc<AudioMonitorService> {
    AudioMonitorService::global(|| Ok(AudioMonitorService::new()))
        .expect("audio monitor service initialization should not fail")
}

/// Return the shared process-global audio monitor service when it is already live.
///
/// This is for backend callback ingress that may arrive after the monitor service
/// has not been initialized yet or has already been torn down.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) fn active_audio_monitor_service() -> Option<Arc<AudioMonitorService>> {
    AudioMonitorService::active()
}

/// Return the effective monitor demand for one runtime and backend.
fn runtime_backend_monitor_demand(
    runtime_state: &AudioRuntimeState,
    backend: AudioBackend,
) -> AudioMonitorDemand {
    let streams = event_streams_snapshot(runtime_state);
    let supports_native_monitor = audio_backend::backend_supports_native_device_monitor(backend);
    let mut wants_native_events = false;
    let mut wants_synthetic_worker = false;
    let mut synthetic_interval_ns = u64::MAX;

    for stream in &streams {
        if stream.options.backend != backend || !tracks_device_events(stream.options) {
            continue;
        }

        // use native backend monitors when the backend can provide them and the
        // subscription accepts native delivery
        if supports_native_monitor
            && stream.options.delivery_mode != AudioEventDeliveryMode::PollOnly
        {
            wants_native_events = true;
        }

        // fall back to shared synthetic polling when native device ingress is
        // unavailable or when the subscription explicitly requests poll-only delivery
        if stream.options.delivery_mode == AudioEventDeliveryMode::PollOnly
            || (!supports_native_monitor
                && stream.options.delivery_mode != AudioEventDeliveryMode::NativeOnly)
        {
            wants_synthetic_worker = true;
            synthetic_interval_ns = synthetic_interval_ns.min(stream.options.poll_interval_ns);
        }
    }

    AudioMonitorDemand {
        has_subscribers: wants_native_events || wants_synthetic_worker,
        wants_native_monitor: wants_native_events,
        wants_synthetic_worker,
        synthetic_interval_ns: if wants_synthetic_worker {
            synthetic_interval_ns
        } else {
            0
        },
    }
}

/// Aggregate one backend demand across all subscribed runtimes.
fn aggregate_backend_monitor_demand(
    runtimes: &[Arc<AudioRuntimeState>],
    backend: AudioBackend,
) -> AudioMonitorDemand {
    let mut aggregate = AudioMonitorDemand::default();

    for runtime_state in runtimes {
        let demand = runtime_backend_monitor_demand(runtime_state, backend);
        if !demand.has_subscribers {
            continue;
        }

        aggregate.has_subscribers = true;
        aggregate.wants_native_monitor |= demand.wants_native_monitor;
        aggregate.wants_synthetic_worker |= demand.wants_synthetic_worker;
        if demand.wants_synthetic_worker {
            aggregate.synthetic_interval_ns = if aggregate.synthetic_interval_ns == 0 {
                demand.synthetic_interval_ns
            } else {
                aggregate
                    .synthetic_interval_ns
                    .min(demand.synthetic_interval_ns)
            };
        }
    }

    aggregate
}

/// Start one shared synthetic monitor worker for one backend.
fn start_synthetic_monitor_worker(
    service: Arc<AudioMonitorService>,
    backend: AudioBackend,
    poll_interval_ns: u64,
) -> RuntimeResult<Box<dyn AudioMonitorHandle>> {
    let interval = std::time::Duration::from_nanos(poll_interval_ns.max(1));

    let task = open_periodic_task(
        "destack-audio-monitor",
        AudioMonitorService::POLICY,
        interval,
        move || {
            // publish one fresh synthetic snapshot pass
            let now = host_monotonic_nanos();
            let snapshot = monitor_snapshot(backend);
            if let Ok(snapshot) = snapshot {
                let runtimes = {
                    let mut backends = service
                        .backends
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    let Some(monitor) = backends.get_mut(&backend) else {
                        return Ok(());
                    };

                    monitor.live_runtime_states()
                };

                for runtime_state in &runtimes {
                    publish_device_events_from_snapshot(
                        runtime_state,
                        backend,
                        &snapshot,
                        now,
                        AudioEventSource::SyntheticPoll,
                    );
                }
            }

            Ok(())
        },
    )?;

    Ok(Box::new(SyntheticAudioMonitorHandle { task }))
}
