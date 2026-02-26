use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::host::{HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState};

/// openbsd application lifecycle transitions from native callbacks.
pub type OpenBsdApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one openbsd application lifecycle callback.
pub fn openbsd_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: OpenBsdApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::OpenBsd, lifecycle)
}

/// Submit one openbsd window-available callback.
pub fn openbsd_notify_window_available(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::OpenBsd, window_id)
}

/// Submit one openbsd window-terminated callback.
pub fn openbsd_notify_window_terminated(runtime_id: u64, window_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::OpenBsd, window_id)
}

/// Submit one openbsd window-resized callback.
pub fn openbsd_notify_window_resized(
    runtime_id: u64,
    window_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(
        runtime_id,
        HostPlatform::OpenBsd,
        window_id,
        width_px,
        height_px,
    )
}

/// Submit one openbsd permission-result callback.
pub fn openbsd_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::OpenBsd, permission, granted)
}

/// Submit one openbsd interruption callback.
pub fn openbsd_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::OpenBsd, interrupted)
}

/// Submit one openbsd window focus callback.
pub fn openbsd_notify_window_focus_changed(
    runtime_id: u64,
    window_id: u64,
    is_focused: bool,
) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::OpenBsd, window_id, is_focused)
}

/// Submit one openbsd memory pressure callback.
pub fn openbsd_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::OpenBsd, level)
}

/// Submit one openbsd thermal state callback.
pub fn openbsd_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::OpenBsd, state)
}

/// Submit one openbsd power mode callback.
pub fn openbsd_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::OpenBsd, mode)
}

/// Submit one openbsd wall clock callback.
pub fn openbsd_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::OpenBsd)
}

/// Wake one blocked host poll operation for openbsd.
pub fn openbsd_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::OpenBsd)
}
