use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostIngressHandle, HostRuntimeId, HostRuntimeRegistry};
use crate::host::{
    HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState, HostLocationEvent,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent, HostPowerMode,
    HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
use crate::platform::os::abi_generated::LocationSampleValue;
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

/// Return the active Unix host queue for this process and platform.
fn unix_host_bridge(runtime_id: u64, platform: Platform) -> RuntimeResult<HostIngressHandle> {
    HostRuntimeRegistry::ingress_handle_for_runtime(HostRuntimeId(runtime_id), platform)
}

/// Submit one Unix application lifecycle callback.
pub fn unix_notify_application_lifecycle(
    runtime_id: u64,
    platform: Platform,
    lifecycle: UnixApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    let state = host_lifecycle_state_for_unix_application(lifecycle);
    bridge.publish_event(HostEvent::Lifecycle(HostLifecycleEvent { state }));

    Ok(())
}

/// Submit one Unix permission-result callback.
pub fn unix_notify_permission_result(
    runtime_id: u64,
    platform: Platform,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.publish_event(HostEvent::Permission(HostPermissionEvent {
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}

/// Submit one Unix location sample callback.
pub fn unix_notify_location_sample(
    runtime_id: u64,
    platform: Platform,
    watch_id: &str,
    sample: LocationSampleValue,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.publish_event(HostEvent::Location(Box::new(HostLocationEvent {
        watch_id: watch_id.to_string(),
        sample,
    })));

    Ok(())
}

/// Submit one Unix interruption callback.
pub fn unix_notify_interruption_changed(
    runtime_id: u64,
    platform: Platform,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.publish_event(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one Unix memory pressure callback.
pub fn unix_notify_memory_pressure_changed(
    runtime_id: u64,
    platform: Platform,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.publish_event(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one Unix thermal state callback.
pub fn unix_notify_thermal_state_changed(
    runtime_id: u64,
    platform: Platform,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.publish_event(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one Unix power mode callback.
pub fn unix_notify_power_mode_changed(
    runtime_id: u64,
    platform: Platform,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.publish_event(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one Unix wall clock callback.
pub fn unix_notify_wall_clock_changed(runtime_id: u64, platform: Platform) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.publish_event(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for Unix platforms.
pub fn unix_notify_wake(runtime_id: u64, platform: Platform) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.wake()?;

    Ok(())
}

/// Map one Unix application lifecycle transition to host lifecycle state.
pub(crate) fn host_lifecycle_state_for_unix_application(
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
