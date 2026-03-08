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

/// Windows application lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowsApplicationLifecycle {
    /// Process and windowing resources are initializing.
    Created,
    /// App entered active foreground state.
    Activated,
    /// App resumed from suspended state.
    Resumed,
    /// App moved into suspended or background state.
    Suspended,
    /// App is stopping but not fully terminated yet.
    Stopping,
    /// App process is terminating.
    Destroyed,
}

/// Return the active Windows host queue for this process.
fn windows_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostQueue>> {
    HostQueueRegistry::shared()
        .write()
        .queue_for_runtime(RuntimeId(runtime_id), Platform::Windows)
}

/// Submit one Windows application lifecycle callback.
pub fn windows_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: WindowsApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_windows_application(lifecycle);
    bridge.enqueue(HostEvent::Lifecycle(HostLifecycleEvent { state }));

    Ok(())
}

/// Submit one Windows permission-result callback.
pub fn windows_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Permission(HostPermissionEvent {
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}

/// Submit one Windows interruption callback.
pub fn windows_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one Windows memory pressure callback.
pub fn windows_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one Windows thermal state callback.
pub fn windows_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one Windows power mode callback.
pub fn windows_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one Windows wall clock callback.
pub fn windows_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for Windows.
pub fn windows_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.poll_wake_handle().wake()?;

    Ok(())
}

/// Map one Windows application lifecycle transition to host lifecycle state.
pub(super) fn host_lifecycle_state_for_windows_application(
    lifecycle: WindowsApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        WindowsApplicationLifecycle::Created => HostLifecycleState::Initializing,
        WindowsApplicationLifecycle::Activated => HostLifecycleState::Running,
        WindowsApplicationLifecycle::Resumed => HostLifecycleState::Running,
        WindowsApplicationLifecycle::Suspended => HostLifecycleState::Paused,
        WindowsApplicationLifecycle::Stopping => HostLifecycleState::Stopped,
        WindowsApplicationLifecycle::Destroyed => HostLifecycleState::Destroyed,
    }
}
