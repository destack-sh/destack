use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostBridge, HostPlatform, HostWindowEvent, host_bridge_for_runtime};
use crate::host::{HostLifecycleState, HostMemoryPressureLevel, HostPowerMode, HostThermalState};

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

/// Return the active Windows host bridge for this process.
fn windows_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostBridge>> {
    host_bridge_for_runtime(runtime_id, HostPlatform::Windows)
}

/// Submit one Windows application lifecycle callback.
pub fn windows_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: WindowsApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_windows_application(lifecycle);
    bridge.push_lifecycle(state);

    Ok(())
}

/// Submit one Windows window-available callback.
pub fn windows_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowAvailable { window_id });

    Ok(())
}

/// Submit one Windows window-terminated callback.
pub fn windows_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowTerminated { window_id });

    Ok(())
}

/// Submit one Windows window-resized callback.
pub fn windows_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowResized {
        window_id,
        width_px,
        height_px,
    });

    Ok(())
}

/// Submit one Windows window focus callback.
pub fn windows_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_window_focus(window_id, is_focused);

    Ok(())
}

/// Submit one Windows permission-result callback.
pub fn windows_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_permission_result(permission, granted);

    Ok(())
}

/// Submit one Windows interruption callback.
pub fn windows_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_interruption(interrupted);

    Ok(())
}

/// Submit one Windows memory pressure callback.
pub fn windows_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_memory_pressure(level);

    Ok(())
}

/// Submit one Windows thermal state callback.
pub fn windows_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_thermal_state(state);

    Ok(())
}

/// Submit one Windows power mode callback.
pub fn windows_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_power_mode(mode);

    Ok(())
}

/// Submit one Windows wall clock callback.
pub fn windows_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.push_wall_clock_changed();

    Ok(())
}

/// Wake one blocked host poll operation for Windows.
pub fn windows_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = windows_host_bridge(runtime_id)?;
    bridge.wake_handle().wake()
}

/// Map one Windows application lifecycle transition to host lifecycle state.
fn host_lifecycle_state_for_windows_application(
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

#[cfg(test)]
#[path = "tests/callback.rs"]
mod tests;
