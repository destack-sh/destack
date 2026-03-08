use std::sync::Arc;

use destack_workspace::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::core::HostState;
use crate::host::core::registry::host_state_for_runtime;
use crate::host::{HostLifecycleState, HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::runtime::world::RuntimeId;

/// iOS application lifecycle transitions from native callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IosApplicationLifecycle {
    /// App launch finished and normal processing can begin.
    DidFinishLaunching,
    /// App became active in the foreground.
    DidBecomeActive,
    /// App is resigning active foreground state.
    WillResignActive,
    /// App entered background execution state.
    DidEnterBackground,
    /// App is returning to the foreground.
    WillEnterForeground,
    /// App is terminating.
    WillTerminate,
}

/// Return the active iOS host state for this process.
fn ios_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostState>> {
    host_state_for_runtime(RuntimeId(runtime_id), Platform::IOS)
}

/// Submit one iOS application lifecycle callback.
pub fn ios_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: IosApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_application_lifecycle(lifecycle);
    bridge.push_lifecycle(state);

    Ok(())
}

/// Submit one iOS permission-result callback.
pub fn ios_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.push_permission_result(permission, granted);

    Ok(())
}

/// Submit one iOS interruption callback.
pub fn ios_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.push_interruption(interrupted);

    Ok(())
}

/// Submit one iOS memory pressure callback.
pub fn ios_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.push_memory_pressure(level);

    Ok(())
}

/// Submit one iOS thermal state callback.
pub fn ios_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.push_thermal_state(state);

    Ok(())
}

/// Submit one iOS power mode callback.
pub fn ios_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.push_power_mode(mode);

    Ok(())
}

/// Submit one iOS wall clock callback.
pub fn ios_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.push_wall_clock_changed();

    Ok(())
}

/// Wake one blocked host poll operation for iOS.
pub fn ios_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = ios_host_bridge(runtime_id)?;
    bridge.poll_wake_handle().wake()?;

    Ok(())
}

/// Map one iOS application lifecycle transition to host lifecycle state.
pub(super) fn host_lifecycle_state_for_application_lifecycle(
    lifecycle: IosApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        IosApplicationLifecycle::DidFinishLaunching => HostLifecycleState::Initializing,
        IosApplicationLifecycle::DidBecomeActive => HostLifecycleState::Running,
        IosApplicationLifecycle::WillResignActive => HostLifecycleState::Paused,
        IosApplicationLifecycle::DidEnterBackground => HostLifecycleState::Paused,
        IosApplicationLifecycle::WillEnterForeground => HostLifecycleState::Running,
        IosApplicationLifecycle::WillTerminate => HostLifecycleState::Destroyed,
    }
}
