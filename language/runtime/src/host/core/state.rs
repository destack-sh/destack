use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use destack_workspace::PlatformHostOptions;
use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    HostEvent, HostEventQueue, HostLifecycleState, HostMemoryPressureLevel, HostPollOutcome,
    HostPowerMode, HostThermalState, HostWindowEvent,
};
use crate::diagnostic::RuntimeResult;
use crate::runtime::poller::HostPollerWakeHandle;

/// Encoded lifecycle value for initializing.
const LIFECYCLE_INITIALIZING: u8 = 0;
/// Encoded lifecycle value for running.
const LIFECYCLE_RUNNING: u8 = 1;
/// Encoded lifecycle value for paused.
const LIFECYCLE_PAUSED: u8 = 2;
/// Encoded lifecycle value for stopped.
const LIFECYCLE_STOPPED: u8 = 3;
/// Encoded lifecycle value for destroyed.
const LIFECYCLE_DESTROYED: u8 = 4;
/// Encoded memory pressure value for normal.
const MEMORY_PRESSURE_NORMAL: u8 = 0;
/// Encoded memory pressure value for warning.
const MEMORY_PRESSURE_WARNING: u8 = 1;
/// Encoded memory pressure value for critical.
const MEMORY_PRESSURE_CRITICAL: u8 = 2;
/// Encoded thermal value for nominal.
const THERMAL_NOMINAL: u8 = 0;
/// Encoded thermal value for fair.
const THERMAL_FAIR: u8 = 1;
/// Encoded thermal value for serious.
const THERMAL_SERIOUS: u8 = 2;
/// Encoded thermal value for critical.
const THERMAL_CRITICAL: u8 = 3;
/// Encoded power mode value for normal.
const POWER_MODE_NORMAL: u8 = 0;
/// Encoded power mode value for low power.
const POWER_MODE_LOW_POWER: u8 = 1;

/// Mutable host service state shared by adapter service surfaces.
#[derive(Debug)]
pub struct HostState {
    /// Shared host event queue for adapter event ingestion.
    queue: HostEventQueue,
    /// Current lifecycle state.
    lifecycle_state: AtomicU8,
    /// State tracked for each known host window.
    windows: RwLock<FxHashMap<u64, HostWindowState>>,
    /// Permission requests currently in-flight by permission tag.
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

/// One tracked host window state entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct HostWindowState {
    /// Whether this window is currently focused.
    is_focused: bool,
    /// Last known width in physical pixels.
    width_px: u32,
    /// Last known height in physical pixels.
    height_px: u32,
}

impl Default for HostState {
    fn default() -> Self {
        Self::new()
    }
}

impl HostState {
    /// Create one host service state with neutral defaults.
    pub fn new() -> Self {
        Self {
            queue: HostEventQueue::new(),
            lifecycle_state: AtomicU8::new(LIFECYCLE_INITIALIZING),
            windows: RwLock::new(FxHashMap::default()),
            permissions_in_flight: RwLock::new(FxHashSet::default()),
            is_interrupted: AtomicBool::new(false),
            memory_pressure_level: AtomicU8::new(MEMORY_PRESSURE_NORMAL),
            thermal_state: AtomicU8::new(THERMAL_NOMINAL),
            power_mode: AtomicU8::new(POWER_MODE_NORMAL),
            wall_clock_change_count: AtomicU64::new(0),
        }
    }

    /// Configure whether one permission request is currently in flight.
    #[cfg(any(test, target_os = "android"))]
    pub(crate) fn set_permission_request_in_flight(&self, permission: &str, is_in_flight: bool) {
        let mut permissions_in_flight = self.permissions_in_flight.write();

        if is_in_flight {
            permissions_in_flight.insert(permission.to_string());
        } else {
            permissions_in_flight.remove(permission);
        }
    }

    /// Apply one host event to mutable service state.
    pub(crate) fn apply_event(&self, event: &HostEvent) {
        match event {
            HostEvent::Lifecycle(event) => {
                let lifecycle_state = encode_lifecycle_state(event.state);
                self.lifecycle_state
                    .store(lifecycle_state, Ordering::Relaxed);
            }
            HostEvent::Window(event) => {
                let mut windows = self.windows.write();

                match event {
                    HostWindowEvent::WindowAvailable { window_id } => {
                        windows.entry(*window_id).or_default();
                    }
                    HostWindowEvent::WindowTerminated { window_id } => {
                        windows.remove(window_id);
                    }
                    HostWindowEvent::WindowResized {
                        window_id,
                        width_px,
                        height_px,
                    } => {
                        let window_state = windows.entry(*window_id).or_default();
                        window_state.width_px = *width_px;
                        window_state.height_px = *height_px;
                    }
                }
            }
            HostEvent::WindowFocus(event) => {
                let mut windows = self.windows.write();
                let window_state = windows.entry(event.window_id).or_default();
                window_state.is_focused = event.is_focused;
            }
            HostEvent::Permission(event) => {
                let mut permissions_in_flight = self.permissions_in_flight.write();
                permissions_in_flight.remove(event.permission.as_str());
            }
            HostEvent::Interruption(event) => {
                self.is_interrupted
                    .store(event.interrupted, Ordering::Relaxed);
            }
            HostEvent::MemoryPressure(event) => {
                let level = encode_memory_pressure_level(event.level);
                self.memory_pressure_level.store(level, Ordering::Relaxed);
            }
            HostEvent::ThermalState(event) => {
                let state = encode_thermal_state(event.state);
                self.thermal_state.store(state, Ordering::Relaxed);
            }
            HostEvent::PowerMode(event) => {
                let mode = encode_power_mode(event.mode);
                self.power_mode.store(mode, Ordering::Relaxed);
            }
            HostEvent::WallClock(_) => {
                self.wall_clock_change_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Return one shared wake handle for host event polling.
    pub(crate) fn wake_handle(&self) -> Arc<dyn HostPollerWakeHandle> {
        self.queue.wake_handle()
    }

    /// Configure host integration options on queue policy.
    pub(crate) fn configure_host_options(&self, host_options: &PlatformHostOptions) {
        let queue_capacity = host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok());
        self.queue.configure(queue_capacity);
    }

    /// Poll events from this host and apply service-state updates.
    pub(crate) fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        let events = self.queue.poll_events(timeout_nanos)?;
        for event in &events {
            self.apply_event(event);
        }

        let dropped_event_count = self.queue.take_dropped_event_count();

        Ok(HostPollOutcome {
            events,
            dropped_event_count,
        })
    }

    /// Enqueue one raw host event.
    pub(crate) fn push_event(&self, event: HostEvent) {
        self.queue.enqueue(event);
    }

    /// Enqueue one lifecycle event.
    pub(crate) fn push_lifecycle(&self, state: HostLifecycleState) {
        self.push_event(HostEvent::Lifecycle(super::HostLifecycleEvent { state }));
    }

    /// Enqueue one window event.
    pub(crate) fn push_window(&self, event: HostWindowEvent) {
        self.push_event(HostEvent::Window(event));
    }

    /// Enqueue one window focus event.
    pub(crate) fn push_window_focus(&self, window_id: u64, is_focused: bool) {
        self.push_event(HostEvent::WindowFocus(super::HostWindowFocusEvent {
            window_id,
            is_focused,
        }));
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
}

impl HostState {
    /// Return the current lifecycle state.
    pub fn lifecycle_state(&self) -> HostLifecycleState {
        let lifecycle_state = self.lifecycle_state.load(Ordering::Relaxed);
        decode_lifecycle_state(lifecycle_state)
    }

    /// Return whether one native window is currently available.
    pub fn has_window(&self) -> bool {
        !self.windows.read().is_empty()
    }

    /// Return whether one native window is currently focused.
    pub fn is_window_focused(&self) -> bool {
        self.windows
            .read()
            .values()
            .any(|window_state| window_state.is_focused)
    }

    /// Return whether host interruption is currently active.
    pub fn is_interrupted(&self) -> bool {
        self.is_interrupted.load(Ordering::Relaxed)
    }

    /// Return the current host memory pressure level.
    pub fn memory_pressure_level(&self) -> HostMemoryPressureLevel {
        let encoded_level = self.memory_pressure_level.load(Ordering::Relaxed);
        decode_memory_pressure_level(encoded_level)
    }

    /// Return the current host thermal state.
    pub fn thermal_state(&self) -> HostThermalState {
        let encoded_state = self.thermal_state.load(Ordering::Relaxed);
        decode_thermal_state(encoded_state)
    }

    /// Return the current host power mode.
    pub fn power_mode(&self) -> HostPowerMode {
        let encoded_mode = self.power_mode.load(Ordering::Relaxed);
        decode_power_mode(encoded_mode)
    }

    /// Return the number of host wall clock change events observed.
    pub fn wall_clock_change_count(&self) -> u64 {
        self.wall_clock_change_count.load(Ordering::Relaxed)
    }

    /// Return whether the named permission currently has a pending request.
    pub fn is_request_in_flight(&self, permission: &str) -> bool {
        let permissions_in_flight = self.permissions_in_flight.read();
        permissions_in_flight.contains(permission)
    }
}

/// Encode one lifecycle state into one compact atomic representation.
fn encode_lifecycle_state(state: HostLifecycleState) -> u8 {
    match state {
        HostLifecycleState::Initializing => LIFECYCLE_INITIALIZING,
        HostLifecycleState::Running => LIFECYCLE_RUNNING,
        HostLifecycleState::Paused => LIFECYCLE_PAUSED,
        HostLifecycleState::Stopped => LIFECYCLE_STOPPED,
        HostLifecycleState::Destroyed => LIFECYCLE_DESTROYED,
    }
}

/// Decode one compact atomic lifecycle representation.
fn decode_lifecycle_state(encoded_state: u8) -> HostLifecycleState {
    match encoded_state {
        LIFECYCLE_RUNNING => HostLifecycleState::Running,
        LIFECYCLE_PAUSED => HostLifecycleState::Paused,
        LIFECYCLE_STOPPED => HostLifecycleState::Stopped,
        LIFECYCLE_DESTROYED => HostLifecycleState::Destroyed,
        _ => HostLifecycleState::Initializing,
    }
}

/// Encode one memory pressure level into one compact atomic representation.
fn encode_memory_pressure_level(level: HostMemoryPressureLevel) -> u8 {
    match level {
        HostMemoryPressureLevel::Normal => MEMORY_PRESSURE_NORMAL,
        HostMemoryPressureLevel::Warning => MEMORY_PRESSURE_WARNING,
        HostMemoryPressureLevel::Critical => MEMORY_PRESSURE_CRITICAL,
    }
}

/// Decode one compact atomic memory pressure representation.
fn decode_memory_pressure_level(encoded_level: u8) -> HostMemoryPressureLevel {
    match encoded_level {
        MEMORY_PRESSURE_WARNING => HostMemoryPressureLevel::Warning,
        MEMORY_PRESSURE_CRITICAL => HostMemoryPressureLevel::Critical,
        _ => HostMemoryPressureLevel::Normal,
    }
}

/// Encode one thermal state into one compact atomic representation.
fn encode_thermal_state(state: HostThermalState) -> u8 {
    match state {
        HostThermalState::Nominal => THERMAL_NOMINAL,
        HostThermalState::Fair => THERMAL_FAIR,
        HostThermalState::Serious => THERMAL_SERIOUS,
        HostThermalState::Critical => THERMAL_CRITICAL,
    }
}

/// Decode one compact atomic thermal representation.
fn decode_thermal_state(encoded_state: u8) -> HostThermalState {
    match encoded_state {
        THERMAL_FAIR => HostThermalState::Fair,
        THERMAL_SERIOUS => HostThermalState::Serious,
        THERMAL_CRITICAL => HostThermalState::Critical,
        _ => HostThermalState::Nominal,
    }
}

/// Encode one power mode into one compact atomic representation.
fn encode_power_mode(mode: HostPowerMode) -> u8 {
    match mode {
        HostPowerMode::Normal => POWER_MODE_NORMAL,
        HostPowerMode::LowPower => POWER_MODE_LOW_POWER,
    }
}

/// Decode one compact atomic power mode representation.
fn decode_power_mode(encoded_mode: u8) -> HostPowerMode {
    match encoded_mode {
        POWER_MODE_LOW_POWER => HostPowerMode::LowPower,
        _ => HostPowerMode::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::HostState;
    use crate::host::{
        HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
        HostMemoryPressureEvent, HostMemoryPressureLevel, HostPowerMode, HostPowerModeEvent,
        HostThermalEvent, HostThermalState, HostWallClockEvent, HostWindowEvent,
        HostWindowFocusEvent,
    };

    #[test]
    fn test_apply_event_updates_lifecycle_state() {
        let state = HostState::new();
        let event = HostEvent::Lifecycle(HostLifecycleEvent {
            state: HostLifecycleState::Running,
        });

        state.apply_event(&event);

        assert_eq!(state.lifecycle_state(), HostLifecycleState::Running);
    }

    #[test]
    fn test_apply_event_updates_window_state() {
        let state = HostState::new();

        state.apply_event(&HostEvent::Window(HostWindowEvent::WindowAvailable {
            window_id: 11,
        }));
        assert!(state.has_window());

        state.apply_event(&HostEvent::Window(HostWindowEvent::WindowTerminated {
            window_id: 11,
        }));
        assert!(!state.has_window());
    }

    #[test]
    fn test_apply_event_updates_interruption_state() {
        let state = HostState::new();
        let event = HostEvent::Interruption(HostInterruptionEvent { interrupted: true });

        state.apply_event(&event);

        assert!(state.is_interrupted());
    }

    #[test]
    fn test_apply_event_updates_window_focus_state() {
        let state = HostState::new();
        let event = HostEvent::WindowFocus(HostWindowFocusEvent {
            window_id: 42,
            is_focused: false,
        });

        state.apply_event(&event);

        assert!(!state.is_window_focused());
    }

    #[test]
    fn test_apply_event_updates_memory_pressure_state() {
        let state = HostState::new();
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
        let state = HostState::new();
        let event = HostEvent::ThermalState(HostThermalEvent {
            state: HostThermalState::Serious,
        });

        state.apply_event(&event);

        assert_eq!(state.thermal_state(), HostThermalState::Serious);
    }

    #[test]
    fn test_apply_event_updates_power_mode_state() {
        let state = HostState::new();
        let event = HostEvent::PowerMode(HostPowerModeEvent {
            mode: HostPowerMode::LowPower,
        });

        state.apply_event(&event);

        assert_eq!(state.power_mode(), HostPowerMode::LowPower);
    }

    #[test]
    fn test_apply_event_updates_wall_clock_change_count() {
        let state = HostState::new();
        let event = HostEvent::WallClock(HostWallClockEvent);

        state.apply_event(&event);

        assert_eq!(state.wall_clock_change_count(), 1);
    }
}
