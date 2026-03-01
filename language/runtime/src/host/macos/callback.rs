use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostPlatform, HostState, HostWindowEvent, host_state_for_runtime};
use crate::host::{HostLifecycleState, HostMemoryPressureLevel, HostPowerMode, HostThermalState};

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

/// Return the active macOS host state for this process.
fn macos_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostState>> {
    host_state_for_runtime(runtime_id, HostPlatform::MacOS)
}

/// Submit one macOS application lifecycle callback.
pub fn macos_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: MacosApplicationLifecycle,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_application_lifecycle(lifecycle);
    bridge.push_lifecycle(state);

    Ok(())
}

/// Submit one macOS window-available callback.
pub fn macos_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowAvailable { window_id });

    Ok(())
}

/// Submit one macOS window-terminated callback.
pub fn macos_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowTerminated { window_id });

    Ok(())
}

/// Submit one macOS window-resized callback.
pub fn macos_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowResized {
        window_id,
        width_px,
        height_px,
    });

    Ok(())
}

/// Submit one macOS window focus callback.
pub fn macos_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_window_focus(window_id, is_focused);

    Ok(())
}

/// Submit one macOS permission-result callback.
pub fn macos_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_permission_result(permission, granted);

    Ok(())
}

/// Submit one macOS interruption callback.
pub fn macos_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_interruption(interrupted);

    Ok(())
}

/// Submit one macOS memory pressure callback.
pub fn macos_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_memory_pressure(level);

    Ok(())
}

/// Submit one macOS thermal state callback.
pub fn macos_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_thermal_state(state);

    Ok(())
}

/// Submit one macOS power mode callback.
pub fn macos_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_power_mode(mode);

    Ok(())
}

/// Submit one macOS wall clock callback.
pub fn macos_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.push_wall_clock_changed();

    Ok(())
}

/// Wake one blocked host poll operation for macOS.
pub fn macos_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = macos_host_bridge(runtime_id)?;
    bridge.wake_handle().wake()
}

/// Map one macOS application lifecycle transition to host lifecycle state.
fn host_lifecycle_state_for_application_lifecycle(
    lifecycle: MacosApplicationLifecycle,
) -> HostLifecycleState {
    match lifecycle {
        MacosApplicationLifecycle::DidFinishLaunching => HostLifecycleState::Initializing,
        MacosApplicationLifecycle::DidBecomeActive => HostLifecycleState::Running,
        MacosApplicationLifecycle::WillResignActive => HostLifecycleState::Paused,
        MacosApplicationLifecycle::WillTerminate => HostLifecycleState::Destroyed,
    }
}

#[cfg(test)]
#[path = "tests/callback.rs"]
mod tests;
