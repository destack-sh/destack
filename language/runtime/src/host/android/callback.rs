use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostBridge, HostPlatform, HostWindowEvent, host_bridge_for_runtime};
use crate::host::{HostLifecycleState, HostMemoryPressureLevel, HostPowerMode, HostThermalState};

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

/// Return the active Android host bridge for this process.
fn android_host_bridge(runtime_id: u64) -> RuntimeResult<Arc<HostBridge>> {
    host_bridge_for_runtime(runtime_id, HostPlatform::Android)
}

/// Submit one Android activity lifecycle callback.
pub fn android_notify_activity_lifecycle(
    runtime_id: u64,
    lifecycle: AndroidActivityLifecycle,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    let state = host_lifecycle_state_for_android_activity(lifecycle);
    bridge.push_lifecycle(state);

    Ok(())
}

/// Submit one Android surface-available callback.
pub fn android_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowAvailable { window_id });

    Ok(())
}

/// Submit one Android surface-terminated callback.
pub fn android_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowTerminated { window_id });

    Ok(())
}

/// Submit one Android surface-resized callback.
pub fn android_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_window(HostWindowEvent::WindowResized {
        window_id,
        width_px,
        height_px,
    });

    Ok(())
}

/// Submit one Android window focus callback.
pub fn android_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_window_focus(window_id, is_focused);

    Ok(())
}

/// Submit one Android permission-request lifecycle callback.
pub fn android_notify_permission_request_in_flight(
    runtime_id: u64,
    permission: &str,
    is_in_flight: bool,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge
        .state()
        .set_permission_request_in_flight(permission, is_in_flight);

    Ok(())
}

/// Submit one Android permission-result callback.
pub fn android_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_permission_result(permission, granted);

    Ok(())
}

/// Submit one Android interruption callback.
pub fn android_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_interruption(interrupted);

    Ok(())
}

/// Submit one Android memory pressure callback.
pub fn android_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_memory_pressure(level);

    Ok(())
}

/// Submit one Android thermal state callback.
pub fn android_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_thermal_state(state);

    Ok(())
}

/// Submit one Android power mode callback.
pub fn android_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_power_mode(mode);

    Ok(())
}

/// Submit one Android wall clock callback.
pub fn android_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.push_wall_clock_changed();

    Ok(())
}

/// Wake one blocked host poll operation for Android.
pub fn android_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    let bridge = android_host_bridge(runtime_id)?;
    bridge.wake_handle().wake()
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
