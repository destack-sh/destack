use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};

use parking_lot::RwLock;
use rustc_hash::FxHashSet;

use super::{
    HostEvent, HostLifecycleState, HostMemoryPressureLevel, HostPermissionService, HostPowerMode,
    HostStateReader, HostThermalState, HostWindowEvent,
};

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
pub(crate) struct HostStateStore {
    /// Current lifecycle state.
    lifecycle_state: AtomicU8,
    /// Whether one host window is currently available.
    has_window: AtomicBool,
    /// Whether one host window is currently focused.
    is_window_focused: AtomicBool,
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

impl Default for HostStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl HostStateStore {
    /// Create one host service state with neutral defaults.
    pub(crate) fn new() -> Self {
        Self {
            lifecycle_state: AtomicU8::new(LIFECYCLE_INITIALIZING),
            has_window: AtomicBool::new(false),
            is_window_focused: AtomicBool::new(true),
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
            HostEvent::Poller(_) => {}
            HostEvent::Lifecycle(event) => {
                let lifecycle_state = encode_lifecycle_state(event.state);
                self.lifecycle_state
                    .store(lifecycle_state, Ordering::Relaxed);
            }
            HostEvent::Window(event) => match event {
                HostWindowEvent::WindowAvailable | HostWindowEvent::WindowResized { .. } => {
                    self.has_window.store(true, Ordering::Relaxed);
                }
                HostWindowEvent::WindowTerminated => {
                    self.has_window.store(false, Ordering::Relaxed);
                }
            },
            HostEvent::WindowFocus(event) => {
                self.is_window_focused
                    .store(event.is_focused, Ordering::Relaxed);
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
}

impl HostStateReader for HostStateStore {
    fn lifecycle_state(&self) -> HostLifecycleState {
        let lifecycle_state = self.lifecycle_state.load(Ordering::Relaxed);
        decode_lifecycle_state(lifecycle_state)
    }

    fn has_window(&self) -> bool {
        self.has_window.load(Ordering::Relaxed)
    }

    fn is_window_focused(&self) -> bool {
        self.is_window_focused.load(Ordering::Relaxed)
    }

    fn is_interrupted(&self) -> bool {
        self.is_interrupted.load(Ordering::Relaxed)
    }

    fn memory_pressure_level(&self) -> HostMemoryPressureLevel {
        let encoded_level = self.memory_pressure_level.load(Ordering::Relaxed);
        decode_memory_pressure_level(encoded_level)
    }

    fn thermal_state(&self) -> HostThermalState {
        let encoded_state = self.thermal_state.load(Ordering::Relaxed);
        decode_thermal_state(encoded_state)
    }

    fn power_mode(&self) -> HostPowerMode {
        let encoded_mode = self.power_mode.load(Ordering::Relaxed);
        decode_power_mode(encoded_mode)
    }

    fn wall_clock_change_count(&self) -> u64 {
        self.wall_clock_change_count.load(Ordering::Relaxed)
    }
}

impl HostPermissionService for HostStateStore {
    fn is_request_in_flight(&self, permission: &str) -> bool {
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
    use super::HostStateStore;
    use crate::host::{
        HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
        HostMemoryPressureEvent, HostMemoryPressureLevel, HostPowerMode, HostPowerModeEvent,
        HostStateReader, HostThermalEvent, HostThermalState, HostWallClockEvent, HostWindowEvent,
        HostWindowFocusEvent,
    };

    #[test]
    fn test_apply_event_updates_lifecycle_state() {
        let state = HostStateStore::new();
        let event = HostEvent::Lifecycle(HostLifecycleEvent {
            state: HostLifecycleState::Running,
        });

        state.apply_event(&event);

        assert_eq!(state.lifecycle_state(), HostLifecycleState::Running);
    }

    #[test]
    fn test_apply_event_updates_window_state() {
        let state = HostStateStore::new();

        state.apply_event(&HostEvent::Window(HostWindowEvent::WindowAvailable));
        assert!(state.has_window());

        state.apply_event(&HostEvent::Window(HostWindowEvent::WindowTerminated));
        assert!(!state.has_window());
    }

    #[test]
    fn test_apply_event_updates_interruption_state() {
        let state = HostStateStore::new();
        let event = HostEvent::Interruption(HostInterruptionEvent { interrupted: true });

        state.apply_event(&event);

        assert!(state.is_interrupted());
    }

    #[test]
    fn test_apply_event_updates_window_focus_state() {
        let state = HostStateStore::new();
        let event = HostEvent::WindowFocus(HostWindowFocusEvent { is_focused: false });

        state.apply_event(&event);

        assert!(!state.is_window_focused());
    }

    #[test]
    fn test_apply_event_updates_memory_pressure_state() {
        let state = HostStateStore::new();
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
        let state = HostStateStore::new();
        let event = HostEvent::ThermalState(HostThermalEvent {
            state: HostThermalState::Serious,
        });

        state.apply_event(&event);

        assert_eq!(state.thermal_state(), HostThermalState::Serious);
    }

    #[test]
    fn test_apply_event_updates_power_mode_state() {
        let state = HostStateStore::new();
        let event = HostEvent::PowerMode(HostPowerModeEvent {
            mode: HostPowerMode::LowPower,
        });

        state.apply_event(&event);

        assert_eq!(state.power_mode(), HostPowerMode::LowPower);
    }

    #[test]
    fn test_apply_event_updates_wall_clock_change_count() {
        let state = HostStateStore::new();
        let event = HostEvent::WallClock(HostWallClockEvent);

        state.apply_event(&event);

        assert_eq!(state.wall_clock_change_count(), 1);
    }
}
