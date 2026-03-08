use std::sync::Arc;

use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostQueue, HostQueueRegistry};
use crate::host::{
    HostEvent, HostInterruptionEvent, HostLifecycleEvent, HostLifecycleState,
    HostMemoryPressureEvent, HostMemoryPressureLevel, HostPermissionEvent, HostPowerMode,
    HostPowerModeEvent, HostThermalEvent, HostThermalState, HostWallClockEvent,
};
use crate::runtime::world::RuntimeId;

/// Android activity lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AndroidActivityLifecycle {
    /// Activity was created (`onCreate`).
    Created,
    /// Activity moved to started state (`onStart`).
    Started,
    /// Activity moved to resumed state (`onResume`).
    Resumed,
    /// Activity moved to paused state (`onPause`).
    Paused,
    /// Activity moved to stopped state (`onStop`).
    Stopped,
    /// Activity was destroyed (`onDestroy`).
    Destroyed,
}

/// Return the active Android host queue for this process.
fn android_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostQueue>> {
    HostQueueRegistry::shared()
        .write()
        .queue_for_runtime(RuntimeId(runtime_id), Platform::Android)
}

/// Submit one Android activity lifecycle callback.
pub fn android_notify_activity_lifecycle(
    runtime_id: u64,
    lifecycle: AndroidActivityLifecycle,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_android_activity(lifecycle);
    bridge.enqueue(HostEvent::Lifecycle(HostLifecycleEvent { state }));

    Ok(())
}

/// Submit one Android permission-result callback.
pub fn android_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Permission(HostPermissionEvent {
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}

/// Submit one Android interruption callback.
pub fn android_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one Android memory pressure callback.
pub fn android_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one Android thermal state callback.
pub fn android_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one Android power mode callback.
pub fn android_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one Android wall clock callback.
pub fn android_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for Android.
pub fn android_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.poll_wake_handle().wake()?;

    Ok(())
}

/// Map one Android activity lifecycle transition to host lifecycle state.
pub(super) fn host_lifecycle_state_for_android_activity(
    lifecycle: AndroidActivityLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        AndroidActivityLifecycle::Created => HostLifecycleState::Initializing,
        AndroidActivityLifecycle::Started => HostLifecycleState::Running,
        AndroidActivityLifecycle::Resumed => HostLifecycleState::Running,
        AndroidActivityLifecycle::Paused => HostLifecycleState::Paused,
        AndroidActivityLifecycle::Stopped => HostLifecycleState::Stopped,
        AndroidActivityLifecycle::Destroyed => HostLifecycleState::Destroyed,
    }
}
