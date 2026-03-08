use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use destack_workspace::PlatformHostOptions;
use parking_lot::RwLock;
use rustc_hash::FxHashSet;

use super::{
    HostEvent, HostEventQueue, HostLifecycleState, HostMemoryPressureLevel, HostPollOutcome,
    HostPowerMode, HostThermalState,
};
use crate::diagnostic::RuntimeResult;
use crate::runtime::poller::PollerWakeHandle;

/// Shared host runtime state for one host instance.
#[derive(Debug)]
pub(crate) struct HostState {
    /// Shared host event queue for adapter event ingestion.
    queue: HostEventQueue,
    /// Current lifecycle state.
    lifecycle_state: AtomicU8,
    /// Permission requests currently in flight by permission tag.
    permissions_in_flight: RwLock<FxHashSet<String>>,
    /// Whether host interruption is currently active.
    is_interrupted: AtomicBool,
    /// Current memory pressure state.
    memory_pressure_level: AtomicU8,
    /// Current thermal state.
    thermal_state: AtomicU8,
    /// Current power mode state.
    power_mode: AtomicU8,
    /// Number of wall clock changes observed.
    wall_clock_change_count: AtomicU64,
}

impl HostState {
    /// Create one shared host state.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self::new_inner())
    }

    /// Create one shared host state for tests without runtime registration.
    #[cfg(test)]
    pub(crate) fn new_for_test() -> Arc<Self> {
        Arc::new(Self::new_inner())
    }

    /// Create one unregistered host state value.
    fn new_inner() -> Self {
        Self {
            queue: HostEventQueue::new(),
            lifecycle_state: AtomicU8::new(HostLifecycleState::Initializing.encode()),
            permissions_in_flight: RwLock::new(FxHashSet::default()),
            is_interrupted: AtomicBool::new(false),
            memory_pressure_level: AtomicU8::new(HostMemoryPressureLevel::Normal.encode()),
            thermal_state: AtomicU8::new(HostThermalState::Nominal.encode()),
            power_mode: AtomicU8::new(HostPowerMode::Normal.encode()),
            wall_clock_change_count: AtomicU64::new(0),
        }
    }

    /// Return one shared wake handle for host event polling.
    pub(crate) fn poll_wake_handle(&self) -> Arc<dyn PollerWakeHandle> {
        self.queue.poll_wake_handle()
    }

    /// Configure host integration options on queue policy.
    pub(crate) fn apply_host_options(&self, host_options: &PlatformHostOptions) {
        let queue_capacity = host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok());

        self.queue.configure(queue_capacity);
    }

    /// Poll events from this host and apply service-state updates.
    pub(crate) fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        let events = self.queue.poll_events(timeout_nanos)?;

        // update the semantic host state from the drained events
        for event in &events {
            self.apply_event(event);
        }

        let dropped_event_count = self.queue.take_dropped_event_count();

        Ok(HostPollOutcome {
            events,
            dropped_event_count,
        })
    }

    /// Configure whether one permission request is currently in flight.
    #[cfg(any(test, target_os = "android"))]
    pub(crate) fn set_permission_request_in_flight(&self, permission: &str, is_in_flight: bool) {
        let mut permissions_in_flight = self.permissions_in_flight.write();

        // track this permission request in the shared runtime state
        if is_in_flight {
            permissions_in_flight.insert(permission.to_string());
        }
        // otherwise remove the completed request
        else {
            permissions_in_flight.remove(permission);
        }
    }

    /// Apply one host event to mutable semantic host state.
    pub(crate) fn apply_event(&self, event: &HostEvent) {
        match event {
            // lifecycle
            HostEvent::Lifecycle(event) => {
                self.lifecycle_state
                    .store(event.state.encode(), Ordering::Relaxed);
            }

            // permissions
            HostEvent::Permission(event) => {
                let mut permissions_in_flight = self.permissions_in_flight.write();
                permissions_in_flight.remove(event.permission.as_str());
            }

            // interruption
            HostEvent::Interruption(event) => {
                self.is_interrupted
                    .store(event.interrupted, Ordering::Relaxed);
            }

            // memory pressure
            HostEvent::MemoryPressure(event) => {
                self.memory_pressure_level
                    .store(event.level.encode(), Ordering::Relaxed);
            }

            // thermal
            HostEvent::ThermalState(event) => {
                self.thermal_state
                    .store(event.state.encode(), Ordering::Relaxed);
            }

            // power mode
            HostEvent::PowerMode(event) => {
                self.power_mode
                    .store(event.mode.encode(), Ordering::Relaxed);
            }

            // wall clock
            HostEvent::WallClock(_) => {
                self.wall_clock_change_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Enqueue one raw host event.
    pub(crate) fn push_event(&self, event: HostEvent) {
        self.queue.enqueue(event);
    }

    /// Enqueue one lifecycle event.
    pub(crate) fn push_lifecycle(&self, state: HostLifecycleState) {
        self.push_event(HostEvent::Lifecycle(super::HostLifecycleEvent { state }));
    }

    /// Enqueue one permission result event.
    pub(crate) fn push_permission_result(&self, permission: &str, granted: bool) {
        self.push_event(HostEvent::Permission(super::HostPermissionEvent {
            permission: permission.to_string(),
            granted,
        }));
    }

    /// Enqueue one interruption event.
    pub(crate) fn push_interruption(&self, interrupted: bool) {
        self.push_event(HostEvent::Interruption(super::HostInterruptionEvent {
            interrupted,
        }));
    }

    /// Enqueue one memory pressure event.
    pub(crate) fn push_memory_pressure(&self, level: HostMemoryPressureLevel) {
        self.push_event(HostEvent::MemoryPressure(super::HostMemoryPressureEvent {
            level,
        }));
    }

    /// Enqueue one thermal state event.
    pub(crate) fn push_thermal_state(&self, state: HostThermalState) {
        self.push_event(HostEvent::ThermalState(super::HostThermalEvent { state }));
    }

    /// Enqueue one power mode event.
    pub(crate) fn push_power_mode(&self, mode: HostPowerMode) {
        self.push_event(HostEvent::PowerMode(super::HostPowerModeEvent { mode }));
    }

    /// Enqueue one wall clock change event.
    pub(crate) fn push_wall_clock_changed(&self) {
        self.push_event(HostEvent::WallClock(super::HostWallClockEvent));
    }

    /// Return the current lifecycle state.
    #[cfg(test)]
    pub(crate) fn lifecycle_state(&self) -> HostLifecycleState {
        let lifecycle_state = self.lifecycle_state.load(Ordering::Relaxed);
        HostLifecycleState::decode(lifecycle_state)
    }

    /// Return whether host interruption is currently active.
    #[cfg(test)]
    pub(crate) fn is_interrupted(&self) -> bool {
        self.is_interrupted.load(Ordering::Relaxed)
    }

    /// Return the current host memory pressure level.
    #[cfg(test)]
    pub(crate) fn memory_pressure_level(&self) -> HostMemoryPressureLevel {
        let encoded_level = self.memory_pressure_level.load(Ordering::Relaxed);
        HostMemoryPressureLevel::decode(encoded_level)
    }

    /// Return the current host thermal state.
    #[cfg(test)]
    pub(crate) fn thermal_state(&self) -> HostThermalState {
        let encoded_state = self.thermal_state.load(Ordering::Relaxed);
        HostThermalState::decode(encoded_state)
    }

    /// Return the current host power mode.
    #[cfg(test)]
    pub(crate) fn power_mode(&self) -> HostPowerMode {
        let encoded_mode = self.power_mode.load(Ordering::Relaxed);
        HostPowerMode::decode(encoded_mode)
    }

    /// Return the number of host wall clock change events observed.
    #[cfg(test)]
    pub(crate) fn wall_clock_change_count(&self) -> u64 {
        self.wall_clock_change_count.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::HostState;
    use crate::host::{
        HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
        HostMemoryPressureEvent, HostMemoryPressureLevel, HostPowerMode, HostPowerModeEvent,
        HostThermalEvent, HostThermalState, HostWallClockEvent,
    };

    #[test]
    fn test_apply_event_updates_lifecycle_state() {
        let state = HostState::new_for_test();
        let event = HostEvent::Lifecycle(HostLifecycleEvent {
            state: HostLifecycleState::Running,
        });

        state.apply_event(&event);

        assert_eq!(state.lifecycle_state(), HostLifecycleState::Running);
    }

    #[test]
    fn test_apply_event_updates_interruption_state() {
        let state = HostState::new_for_test();
        let event = HostEvent::Interruption(HostInterruptionEvent { interrupted: true });

        state.apply_event(&event);

        assert!(state.is_interrupted());
    }

    #[test]
    fn test_apply_event_updates_memory_pressure_state() {
        let state = HostState::new_for_test();
        let event = HostEvent::MemoryPressure(HostMemoryPressureEvent {
            level: HostMemoryPressureLevel::Critical,
        });

        state.apply_event(&event);

        assert_eq!(
            state.memory_pressure_level(),
            HostMemoryPressureLevel::Critical
        );
    }

    #[test]
    fn test_apply_event_updates_thermal_state() {
        let state = HostState::new_for_test();
        let event = HostEvent::ThermalState(HostThermalEvent {
            state: HostThermalState::Serious,
        });

        state.apply_event(&event);

        assert_eq!(state.thermal_state(), HostThermalState::Serious);
    }

    #[test]
    fn test_apply_event_updates_power_mode_state() {
        let state = HostState::new_for_test();
        let event = HostEvent::PowerMode(HostPowerModeEvent {
            mode: HostPowerMode::LowPower,
        });

        state.apply_event(&event);

        assert_eq!(state.power_mode(), HostPowerMode::LowPower);
    }

    #[test]
    fn test_apply_event_updates_wall_clock_change_count() {
        let state = HostState::new_for_test();
        let event = HostEvent::WallClock(HostWallClockEvent);

        state.apply_event(&event);

        assert_eq!(state.wall_clock_change_count(), 1);
    }
}
