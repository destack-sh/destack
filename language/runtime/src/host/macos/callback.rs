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

/// macOS application lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacosApplicationLifecycle {
    /// App launch finished and normal processing can begin.
    DidFinishLaunching,
    /// App became active in the foreground.
    DidBecomeActive,
    /// App is resigning active foreground state.
    WillResignActive,
    /// App is terminating.
    WillTerminate,
}

/// Return the active macOS host queue for this process.
fn macos_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostQueue>> {
    HostQueueRegistry::shared()
        .write()
        .queue_for_runtime(RuntimeId(runtime_id), Platform::MacOS)
}

/// Submit one macOS application lifecycle callback.
pub fn macos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: MacosApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_application_lifecycle(lifecycle);
    bridge.enqueue(HostEvent::Lifecycle(HostLifecycleEvent { state }));

    Ok(())
}

/// Submit one macOS permission-result callback.
pub fn macos_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Permission(HostPermissionEvent {
        permission: permission.to_string(),
        granted,
    }));

    Ok(())
}

/// Submit one macOS interruption callback.
pub fn macos_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::Interruption(HostInterruptionEvent {
        interrupted,
    }));

    Ok(())
}

/// Submit one macOS memory pressure callback.
pub fn macos_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::MemoryPressure(HostMemoryPressureEvent { level }));

    Ok(())
}

/// Submit one macOS thermal state callback.
pub fn macos_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::ThermalState(HostThermalEvent { state }));

    Ok(())
}

/// Submit one macOS power mode callback.
pub fn macos_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::PowerMode(HostPowerModeEvent { mode }));

    Ok(())
}

/// Submit one macOS wall clock callback.
pub fn macos_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.enqueue(HostEvent::WallClock(HostWallClockEvent));

    Ok(())
}

/// Wake one blocked host poll operation for macOS.
pub fn macos_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.poll_wake_handle().wake()?;

    Ok(())
}

/// Map one macOS application lifecycle transition to host lifecycle state.
pub(super) fn host_lifecycle_state_for_application_lifecycle(
    lifecycle: MacosApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        MacosApplicationLifecycle::DidFinishLaunching => HostLifecycleState::Initializing,
        MacosApplicationLifecycle::DidBecomeActive => HostLifecycleState::Running,
        MacosApplicationLifecycle::WillResignActive => HostLifecycleState::Paused,
        MacosApplicationLifecycle::WillTerminate => HostLifecycleState::Destroyed,
    }
}
