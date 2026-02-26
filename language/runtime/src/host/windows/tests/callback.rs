use std::sync::Arc;

use super::{
    WindowsApplicationLifecycle, host_lifecycle_state_for_windows_application,
    windows_notify_window_available,
};
use crate::host::core::{HostBridge, HostStateStore, register_host_bridge};
use crate::host::{HostEvent, HostLifecycleState, HostPlatform, HostWindowEvent};

#[test]
fn test_map_windows_lifecycle_to_initializing() {
    let state = host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Created);
    assert_eq!(state, HostLifecycleState::Initializing);
}

#[test]
fn test_map_windows_lifecycle_to_running() {
    let state =
        host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Activated);
    assert_eq!(state, HostLifecycleState::Running);

    let state = host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Resumed);
    assert_eq!(state, HostLifecycleState::Running);
}

#[test]
fn test_map_windows_lifecycle_to_paused() {
    let state =
        host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Suspended);
    assert_eq!(state, HostLifecycleState::Paused);
}

#[test]
fn test_map_windows_lifecycle_to_stopped() {
    let state = host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Stopping);
    assert_eq!(state, HostLifecycleState::Stopped);
}

#[test]
fn test_map_windows_lifecycle_to_destroyed() {
    let state =
        host_lifecycle_state_for_windows_application(WindowsApplicationLifecycle::Destroyed);
    assert_eq!(state, HostLifecycleState::Destroyed);
}

#[test]
fn test_notify_window_available_enqueues_window_event_for_runtime_bridge() {
    let state_store = Arc::new(HostStateStore::new());
    let bridge = Arc::new(HostBridge::new(state_store));
    let registration = register_host_bridge(HostPlatform::Windows, &bridge);
    let runtime_id = registration.runtime_id();

    windows_notify_window_available(runtime_id, 9).unwrap();

    let events = bridge.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Window(HostWindowEvent::WindowAvailable {
            window_id: 9,
        })]
    );
}

#[test]
fn test_notify_window_available_rejects_platform_mismatch_for_runtime_bridge() {
    let state_store = Arc::new(HostStateStore::new());
    let bridge = Arc::new(HostBridge::new(state_store));
    let registration = register_host_bridge(HostPlatform::MacOS, &bridge);
    let runtime_id = registration.runtime_id();

    let result = windows_notify_window_available(runtime_id, 9);
    assert!(result.is_err());
}

#[test]
fn test_notify_window_available_rejects_unknown_runtime_id() {
    let result = windows_notify_window_available(0, 9);
    assert!(result.is_err());
}
