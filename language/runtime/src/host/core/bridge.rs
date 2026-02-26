use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use super::{
    HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent, HostPollOutcome,
    HostPowerMode, HostPowerModeEvent, HostState, HostThermalEvent, HostThermalState,
    HostWallClockEvent, HostWindowEvent, HostWindowFocusEvent,
};
use crate::diagnostic::RuntimeResult;
use crate::host::core::HostEventQueue;
use crate::runtime::poller::HostPollerWakeHandle;

/// Shared per-runtime host bridge for adapter event ingestion.
#[derive(Debug, Clone)]
pub(crate) struct HostBridge {
    /// Shared host event queue.
    events: HostEventQueue,
    /// Shared mutable host service state.
    state: Arc<HostState>,
}

impl HostBridge {
    /// Create one host bridge from one shared service state object.
    pub(crate) fn new(state: Arc<HostState>) -> Self {
        // initialize host bridge shared state
        Self {
            events: HostEventQueue::new(),
            state,
        }
    }

    /// Return the shared host service state for this bridge.
    pub(crate) fn state(&self) -> &Arc<HostState> {
        &self.state
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

    /// Poll events from this bridge and apply service-state updates.
    pub(crate) fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        let events = self.events.poll_events(timeout_nanos)?;
        for event in &events {
            self.state.apply_event(event);
        }

        let dropped_event_count = self.events.take_dropped_event_count();

        Ok(HostPollOutcome {
            events,
            dropped_event_count,
        })
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

    /// Enqueue one window focus event.
    pub(crate) fn push_window_focus(&self, window_id: u64, is_focused: bool) {
        self.push_event(HostEvent::WindowFocus(HostWindowFocusEvent {
            window_id,
            is_focused,
        }));
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

    /// Enqueue one memory pressure event.
    pub(crate) fn push_memory_pressure(&self, level: HostMemoryPressureLevel) {
        self.push_event(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));
    }

    /// Enqueue one thermal state event.
    pub(crate) fn push_thermal_state(&self, state: HostThermalState) {
        self.push_event(HostEvent::ThermalState(HostThermalEvent { state }));
    }

    /// Enqueue one power mode event.
    pub(crate) fn push_power_mode(&self, mode: HostPowerMode) {
        self.push_event(HostEvent::PowerMode(HostPowerModeEvent { mode }));
    }

    /// Enqueue one wall clock change event.
    pub(crate) fn push_wall_clock_changed(&self) {
        self.push_event(HostEvent::WallClock(HostWallClockEvent));
    }
}
