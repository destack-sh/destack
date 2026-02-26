use crate::diagnostic::RuntimeResult;
use crate::host::unix::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_memory_pressure_changed, unix_notify_permission_result,
    unix_notify_power_mode_changed, unix_notify_thermal_state_changed, unix_notify_wake,
    unix_notify_wall_clock_changed, unix_notify_window_available, unix_notify_window_focus_changed,
    unix_notify_window_resized, unix_notify_window_terminated,
};
use crate::host::{HostMemoryPressureLevel, HostPlatform, HostPowerMode, HostThermalState};

/// haiku application lifecycle transitions from native callbacks.
pub type HaikuApplicationLifecycle = UnixApplicationLifecycle;

/// Submit one haiku application lifecycle callback.
pub fn haiku_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: HaikuApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, HostPlatform::Haiku, lifecycle)
}

/// Submit one haiku window-available callback.
pub fn haiku_notify_window_available(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_window_available(runtime_id, HostPlatform::Haiku)
}

/// Submit one haiku window-terminated callback.
pub fn haiku_notify_window_terminated(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_window_terminated(runtime_id, HostPlatform::Haiku)
}

/// Submit one haiku window-resized callback.
pub fn haiku_notify_window_resized(
    runtime_id: u64,
    width_px: u32,
    height_px: u32,
) -> RuntimeResult<()> {
    unix_notify_window_resized(runtime_id, HostPlatform::Haiku, width_px, height_px)
}

/// Submit one haiku permission-result callback.
pub fn haiku_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, HostPlatform::Haiku, permission, granted)
}

/// Submit one haiku interruption callback.
pub fn haiku_notify_interruption_changed(runtime_id: u64, interrupted: bool) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, HostPlatform::Haiku, interrupted)
}

/// Submit one haiku window focus callback.
pub fn haiku_notify_window_focus_changed(runtime_id: u64, is_focused: bool) -> RuntimeResult<()> {
    unix_notify_window_focus_changed(runtime_id, HostPlatform::Haiku, is_focused)
}

/// Submit one haiku memory pressure callback.
pub fn haiku_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, HostPlatform::Haiku, level)
}

/// Submit one haiku thermal state callback.
pub fn haiku_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, HostPlatform::Haiku, state)
}

/// Submit one haiku power mode callback.
pub fn haiku_notify_power_mode_changed(runtime_id: u64, mode: HostPowerMode) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, HostPlatform::Haiku, mode)
}

/// Submit one haiku wall clock callback.
pub fn haiku_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, HostPlatform::Haiku)
}

/// Wake one blocked host poll operation for haiku.
pub fn haiku_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, HostPlatform::Haiku)
}
