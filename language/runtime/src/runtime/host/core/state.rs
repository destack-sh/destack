use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use parking_lot::RwLock;
use rustc_hash::FxHashSet;

use super::{
    HostEvent, HostInterruptionService, HostLifecycleService, HostLifecycleState,
    HostPermissionService, HostWindowEvent, HostWindowService,
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

/// Mutable host service state shared by adapter service surfaces.
#[derive(Debug)]
pub(crate) struct HostServiceState {
    /// Current lifecycle state.
    lifecycle_state: AtomicU8,
    /// Whether one host window is currently available.
    has_window: AtomicBool,
    /// Permission requests currently in-flight by permission tag.
    permissions_in_flight: RwLock<FxHashSet<String>>,
    /// Whether host interruption is currently active.
    is_interrupted: AtomicBool,
}

impl Default for HostServiceState {
    fn default() -> Self {
        Self::new()
    }
}

impl HostServiceState {
    /// Create one host service state with neutral defaults.
    pub(crate) fn new() -> Self {
        Self {
            lifecycle_state: AtomicU8::new(LIFECYCLE_INITIALIZING),
            has_window: AtomicBool::new(false),
            permissions_in_flight: RwLock::new(FxHashSet::default()),
            is_interrupted: AtomicBool::new(false),
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
            HostEvent::Permission(event) => {
                let mut permissions_in_flight = self.permissions_in_flight.write();
                permissions_in_flight.remove(event.permission.as_str());
            }
            HostEvent::Interruption(event) => {
                self.is_interrupted
                    .store(event.interrupted, Ordering::Relaxed);
            }
        }
    }
}

impl HostLifecycleService for HostServiceState {
    fn state(&self) -> HostLifecycleState {
        let lifecycle_state = self.lifecycle_state.load(Ordering::Relaxed);
        decode_lifecycle_state(lifecycle_state)
    }
}

impl HostWindowService for HostServiceState {
    fn has_window(&self) -> bool {
        self.has_window.load(Ordering::Relaxed)
    }
}

impl HostPermissionService for HostServiceState {
    fn is_request_in_flight(&self, permission: &str) -> bool {
        let permissions_in_flight = self.permissions_in_flight.read();
        permissions_in_flight.contains(permission)
    }
}

impl HostInterruptionService for HostServiceState {
    fn is_interrupted(&self) -> bool {
        self.is_interrupted.load(Ordering::Relaxed)
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

#[cfg(test)]
mod tests {
    use super::HostServiceState;
    use crate::runtime::host::{
        HostEvent, HostInterruptionEvent, HostInterruptionService, HostLifecycleEvent,
        HostLifecycleService, HostLifecycleState, HostWindowEvent, HostWindowService,
    };

    #[test]
    fn test_apply_event_updates_lifecycle_state() {
        let state = HostServiceState::new();
        let event = HostEvent::Lifecycle(HostLifecycleEvent {
            state: HostLifecycleState::Running,
        });

        state.apply_event(&event);

        assert_eq!(state.state(), HostLifecycleState::Running);
    }

    #[test]
    fn test_apply_event_updates_window_state() {
        let state = HostServiceState::new();

        state.apply_event(&HostEvent::Window(HostWindowEvent::WindowAvailable));
        assert!(state.has_window());

        state.apply_event(&HostEvent::Window(HostWindowEvent::WindowTerminated));
        assert!(!state.has_window());
    }

    #[test]
    fn test_apply_event_updates_interruption_state() {
        let state = HostServiceState::new();
        let event = HostEvent::Interruption(HostInterruptionEvent { interrupted: true });

        state.apply_event(&event);

        assert!(state.is_interrupted());
    }
}
