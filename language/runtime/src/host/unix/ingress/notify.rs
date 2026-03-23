use std::sync::Arc;

use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostQueue, HostSessionId, HostSessionRegistry};
use crate::host::{
    HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleSourceKind,
    HostLifecycleState, HostLocationEvent, HostMemoryPressureEvent, HostMemoryPressureLevel,
    HostPermissionEvent, HostPowerMode, HostPowerModeEvent, HostThermalEvent, HostThermalState,
    HostWallClockEvent,
};
use crate::platform::os::abi_generated::LocationSampleValue;
/// Unix application lifecycle transitions from native ingress hooks.
#[cfg_attr(test, allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum UnixApplicationLifecycle {
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
fn unix_host_bridge(runtime_id: u64, platform: Platform) -> RuntimeResult<Arc<HostQueue>> {
    HostSessionRegistry::queue_for_session(HostSessionId(runtime_id), platform)
}

/// Route one Unix application lifecycle ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_application_lifecycle(
    runtime_id: u64,
    platform: Platform,
    lifecycle: UnixApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    let state = host_lifecycle_state_for_unix_application(lifecycle);
    bridge.enqueue(HostEvent::Lifecycle(HostLifecycleEvent {
        source_kind: HostLifecycleSourceKind::Application,
        state,
    }));

    Ok(())
}

/// Route one Unix permission-result ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_permission_result(
    runtime_id: u64,
    platform: Platform,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.enqueue(HostEvent::Permission(HostPermissionEvent {
        request_id: None,
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}

/// Route one Unix location sample ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_location_sample(
    runtime_id: u64,
    platform: Platform,
    watch_id: &str,
    sample: LocationSampleValue,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.enqueue(HostEvent::Location(Box::new(HostLocationEvent {
        watch_id: watch_id.to_string(),
        sample,
    })));

    Ok(())
}

/// Route one Unix interruption ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_interruption_changed(
    runtime_id: u64,
    platform: Platform,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.enqueue(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Route one Unix memory pressure ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_memory_pressure_changed(
    runtime_id: u64,
    platform: Platform,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.enqueue(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Route one Unix thermal state ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_thermal_state_changed(
    runtime_id: u64,
    platform: Platform,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.enqueue(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Route one Unix power mode ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_power_mode_changed(
    runtime_id: u64,
    platform: Platform,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.enqueue(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Route one Unix wall clock ingress notification.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_wall_clock_changed(
    runtime_id: u64,
    platform: Platform,
) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.enqueue(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for Unix platforms.
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn unix_notify_wake(runtime_id: u64, platform: Platform) -> RuntimeResult<()> {
    let bridge = unix_host_bridge(runtime_id, platform)?;
    bridge.poll_wake_handle().wake()?;

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
