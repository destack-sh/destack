use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::runtime::host::HostLifecycleState;
use crate::runtime::host::core::{
    HostBridge, HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState,
    HostWindowEvent, host_bridge_for_runtime,
};

/// Unix application lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnixApplicationLifecycle {
    /// Process and windowing resources are initializing.
    Created,
    /// App entered active foreground state.
    Running,
    /// App moved into paused or background state.
    Paused,
    /// App is stopping but not fully terminated yet.
    Stopped,
    /// App process is terminating.
    Destroyed,
}

/// Return the active Unix host bridge for this process and platform.
fn unix_host_bridge(runtime_id: u64, platform: HostPlatform) -> RuntimeResult<Arc<HostBridge>> {
    host_bridge_for_runtime(runtime_id, platform)
}

/// Submit one Unix application lifecycle callback.
pub fn unix_notify_application_lifecycle(
    runtime_id: u64,
    platform: HostPlatform,
    lifecycle: UnixApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    let state = host_lifecycle_state_for_unix_application(lifecycle);
    bridge.push_lifecycle(state);

    Ok(())
}

/// Submit one Unix window-available callback.
pub fn unix_notify_window_available(runtime_id: u64, platform: HostPlatform) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_window(HostWindowEvent::WindowAvailable);

    Ok(())
}

/// Submit one Unix window-terminated callback.
pub fn unix_notify_window_terminated(runtime_id: u64, platform: HostPlatform) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_window(HostWindowEvent::WindowTerminated);

    Ok(())
}

/// Submit one Unix window-resized callback.
pub fn unix_notify_window_resized(
    runtime_id: u64,
    platform: HostPlatform,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_window(HostWindowEvent::WindowResized {
        width_px,
        height_px,
    });

    Ok(())
}

/// Submit one Unix window focus callback.
pub fn unix_notify_window_focus_changed(
    runtime_id: u64,
    platform: HostPlatform,
    is_focused: bool,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_window_focus(is_focused);

    Ok(())
}

/// Submit one Unix permission-result callback.
pub fn unix_notify_permission_result(
    runtime_id: u64,
    platform: HostPlatform,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_permission_result(permission, granted);

    Ok(())
}

/// Submit one Unix interruption callback.
pub fn unix_notify_interruption_changed(
    runtime_id: u64,
    platform: HostPlatform,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_interruption(interrupted);

    Ok(())
}

/// Submit one Unix memory pressure callback.
pub fn unix_notify_memory_pressure_changed(
    runtime_id: u64,
    platform: HostPlatform,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_memory_pressure(level);

    Ok(())
}

/// Submit one Unix thermal state callback.
pub fn unix_notify_thermal_state_changed(
    runtime_id: u64,
    platform: HostPlatform,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_thermal_state(state);

    Ok(())
}

/// Submit one Unix power mode callback.
pub fn unix_notify_power_mode_changed(
    runtime_id: u64,
    platform: HostPlatform,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_power_mode(mode);

    Ok(())
}

/// Submit one Unix wall clock callback.
pub fn unix_notify_wall_clock_changed(
    runtime_id: u64,
    platform: HostPlatform,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.push_wall_clock_changed();

    Ok(())
}

/// Wake one blocked host poll operation for Unix platforms.
pub fn unix_notify_wake(runtime_id: u64, platform: HostPlatform) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.wake_handle().wake()
}

/// Map one Unix application lifecycle transition to host lifecycle state.
fn host_lifecycle_state_for_unix_application(
    lifecycle: UnixApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        UnixApplicationLifecycle::Created => HostLifecycleState::Initializing,
        UnixApplicationLifecycle::Running => HostLifecycleState::Running,
        UnixApplicationLifecycle::Paused => HostLifecycleState::Paused,
        UnixApplicationLifecycle::Stopped => HostLifecycleState::Stopped,
        UnixApplicationLifecycle::Destroyed => HostLifecycleState::Destroyed,
    }
}

#[cfg(test)]
mod tests {
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
}
