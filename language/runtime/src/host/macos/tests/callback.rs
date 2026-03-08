use super::super::callback::{
    MacosApplicationLifecycle, host_lifecycle_state_for_application_lifecycle,
    macos_notify_permission_result,
};
use crate::host::core::{HostQueue, HostQueueRegistry};
use crate::host::{HostEvent, HostLifecycleState, HostPermissionEvent, Platform};
use crate::runtime::world::RuntimeId;
use std::sync::Arc;

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
fn test_notify_permission_result_enqueues_permission_event_for_runtime_bridge() {
    let queue = Arc::new(HostQueue::new());
    let registration = HostQueueRegistry::shared().write().register(
        Platform::MacOS,
        RuntimeId(1),
        Arc::downgrade(&queue),
        None,
    );
    let runtime_id = registration.runtime_id();

    macos_notify_permission_result(runtime_id.0, "camera", true).unwrap();

    let events = queue.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Permission(HostPermissionEvent {
            permission: "camera".to_string(),
            granted: true,
        })],
    );
}
