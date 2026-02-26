use std::sync::Arc;

use super::{
    MacosApplicationLifecycle, host_lifecycle_state_for_application_lifecycle,
    macos_notify_window_available,
};
use crate::host::core::{HostBridge, HostStateStore, register_host_bridge};
use crate::host::{HostEvent, HostLifecycleState, HostPlatform, HostWindowEvent};

#[test]
fn test_map_application_lifecycle_to_initializing() {
    let state = host_lifecycle_state_for_application_lifecycle(
        MacosApplicationLifecycle::DidFinishLaunching,
    );
    assert_eq!(state, HostLifecycleState::Initializing);
}

#[test]
fn test_map_application_lifecycle_to_running() {
    let state =
        host_lifecycle_state_for_application_lifecycle(MacosApplicationLifecycle::DidBecomeActive);
    assert_eq!(state, HostLifecycleState::Running);
}

#[test]
fn test_map_application_lifecycle_to_paused() {
    let state =
        host_lifecycle_state_for_application_lifecycle(MacosApplicationLifecycle::WillResignActive);
    assert_eq!(state, HostLifecycleState::Paused);
}

#[test]
fn test_map_application_lifecycle_to_destroyed() {
    let state =
        host_lifecycle_state_for_application_lifecycle(MacosApplicationLifecycle::WillTerminate);
    assert_eq!(state, HostLifecycleState::Destroyed);
}

#[test]
fn test_notify_window_available_enqueues_window_event_for_runtime_bridge() {
    let state_store = Arc::new(HostStateStore::new());
    let bridge = Arc::new(HostBridge::new(state_store));
    let registration = register_host_bridge(HostPlatform::MacOS, &bridge);
    let runtime_id = registration.runtime_id();

    macos_notify_window_available(runtime_id, 13).unwrap();

    let events = bridge.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Window(HostWindowEvent::WindowAvailable {
            window_id: 13,
        })]
    );
}

#[test]
fn test_notify_window_available_rejects_unknown_runtime_id() {
    let result = macos_notify_window_available(0, 13);
    assert!(result.is_err());
}
