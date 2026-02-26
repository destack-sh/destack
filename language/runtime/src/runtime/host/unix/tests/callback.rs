use std::sync::Arc;

use super::{
    UnixApplicationLifecycle, host_lifecycle_state_for_unix_application,
    unix_notify_window_available,
};
use crate::runtime::host::core::{
    HostBridge, HostStateStore, host_bridge_for_runtime, register_host_bridge,
};
use crate::runtime::host::{HostEvent, HostLifecycleState, HostPlatform, HostWindowEvent};

#[test]
fn test_map_unix_lifecycle_to_host_states() {
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Created),
        HostLifecycleState::Initializing
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Running),
        HostLifecycleState::Running
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Paused),
        HostLifecycleState::Paused
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Stopped),
        HostLifecycleState::Stopped
    );
    assert_eq!(
        host_lifecycle_state_for_unix_application(UnixApplicationLifecycle::Destroyed),
        HostLifecycleState::Destroyed
    );
}

#[test]
fn test_notify_window_available_enqueues_window_event_for_runtime_bridge() {
    let state_store = Arc::new(HostStateStore::new());
    let bridge = Arc::new(HostBridge::new(state_store));
    let registration = register_host_bridge(HostPlatform::Linux, &bridge);
    let runtime_id = registration.runtime_id();

    unix_notify_window_available(runtime_id, HostPlatform::Linux).unwrap();

    let events = bridge.poll_events(Some(0)).unwrap();
    assert_eq!(
        events.as_slice(),
        [HostEvent::Window(HostWindowEvent::WindowAvailable)]
    );
}

#[test]
fn test_notify_window_available_rejects_platform_mismatch_for_runtime_bridge() {
    let state_store = Arc::new(HostStateStore::new());
    let bridge = Arc::new(HostBridge::new(state_store));
    let registration = register_host_bridge(HostPlatform::Linux, &bridge);
    let runtime_id = registration.runtime_id();

    let result = unix_notify_window_available(runtime_id, HostPlatform::FreeBsd);
    assert!(result.is_err());

    let resolved_bridge = host_bridge_for_runtime(runtime_id, HostPlatform::Linux).unwrap();
    assert!(Arc::ptr_eq(&bridge, &resolved_bridge));
}
