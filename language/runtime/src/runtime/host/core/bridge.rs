use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use super::{
    HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState, HostPermissionEvent,
    HostServiceState, HostWindowEvent,
};
use crate::diagnostic::RuntimeResult;
use crate::runtime::poller::HostPollerWakeHandle;

/// Shared per-runtime host bridge for adapter event ingestion.
#[derive(Debug, Clone)]
pub(crate) struct HostBridge {
    /// Shared host event queue.
    events: super::HostEventQueue,
    /// Shared mutable host service state.
    service_state: Arc<HostServiceState>,
}

impl HostBridge {
    /// Create one host bridge from one shared service state object.
    pub(crate) fn new(service_state: Arc<HostServiceState>) -> Self {
        Self {
            events: super::HostEventQueue::new(),
            service_state,
        }
    }

    /// Return the shared host service state for this bridge.
    pub(crate) fn service_state(&self) -> &Arc<HostServiceState> {
        &self.service_state
    }

    /// Return one shared wake handle for this bridge.
    pub(crate) fn wake_handle(&self) -> Arc<dyn HostPollerWakeHandle> {
        self.events.wake_handle()
    }

    /// Configure host integration options on bridge queue policy.
    pub(crate) fn configure_host_options(&self, host_options: &PlatformHostOptions) {
        let queue_capacity = host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok());
        self.events.configure(queue_capacity);
    }

    /// Take the number of dropped events observed by the bridge queue.
    pub(crate) fn take_dropped_event_count(&self) -> u64 {
        self.events.take_dropped_event_count()
    }

    /// Poll events from this bridge and apply service-state updates.
    pub(crate) fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<HostEvent>> {
        let events = self.events.poll_events(timeout_nanos)?;
        for event in &events {
            self.service_state.apply_event(event);
        }

        Ok(events)
    }

    /// Enqueue one raw host event.
    pub(crate) fn push_event(&self, event: HostEvent) {
        self.events.enqueue(event);
    }

    /// Enqueue one lifecycle event.
    pub(crate) fn push_lifecycle(&self, state: HostLifecycleState) {
        self.push_event(HostEvent::Lifecycle(HostLifecycleEvent { state }));
    }

    /// Enqueue one window event.
    pub(crate) fn push_window(&self, event: HostWindowEvent) {
        self.push_event(HostEvent::Window(event));
    }

    /// Enqueue one permission result event.
    pub(crate) fn push_permission_result(&self, permission: &str, granted: bool) {
        self.push_event(HostEvent::Permission(HostPermissionEvent {
            permission: permission.to_string(),
            granted,
        }));
    }

    /// Enqueue one interruption event.
    pub(crate) fn push_interruption(&self, interrupted: bool) {
        self.push_event(HostEvent::Interruption(HostInterruptionEvent {
            interrupted,
        }));
    }
}
