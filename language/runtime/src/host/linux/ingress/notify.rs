use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::unix::ingress::notify::{
    UnixApplicationLifecycle, unix_notify_application_lifecycle, unix_notify_interruption_changed,
    unix_notify_location_sample, unix_notify_memory_pressure_changed,
    unix_notify_permission_result, unix_notify_power_mode_changed,
    unix_notify_thermal_state_changed, unix_notify_wake, unix_notify_wall_clock_changed,
};
use crate::host::{HostMemoryPressureLevel, HostPowerMode, HostThermalState};
use crate::platform::os::abi_generated::LocationSampleValue;

/// Linux application lifecycle transitions from native ingress hooks.
pub type LinuxApplicationLifecycle = UnixApplicationLifecycle;

/// Route one Linux application lifecycle ingress notification.
pub(crate) fn linux_notify_application_lifecycle(
    runtime_id: u64,
    lifecycle: LinuxApplicationLifecycle,
) -> RuntimeResult<()> {
    unix_notify_application_lifecycle(runtime_id, Platform::Linux, lifecycle)
}

/// Route one Linux permission-result ingress notification.
pub(crate) fn linux_notify_permission_result(
    runtime_id: u64,
    permission: &str,
    granted: bool,
) -> RuntimeResult<()> {
    unix_notify_permission_result(runtime_id, Platform::Linux, permission, granted)
}

/// Route one Linux location sample ingress notification.
pub(crate) fn linux_notify_location_sample(
    runtime_id: u64,
    watch_id: &str,
    sample: LocationSampleValue,
) -> RuntimeResult<()> {
    unix_notify_location_sample(runtime_id, Platform::Linux, watch_id, sample)
}

/// Route one Linux interruption ingress notification.
pub(crate) fn linux_notify_interruption_changed(
    runtime_id: u64,
    interrupted: bool,
) -> RuntimeResult<()> {
    unix_notify_interruption_changed(runtime_id, Platform::Linux, interrupted)
}

/// Route one Linux memory pressure ingress notification.
pub(crate) fn linux_notify_memory_pressure_changed(
    runtime_id: u64,
    level: HostMemoryPressureLevel,
) -> RuntimeResult<()> {
    unix_notify_memory_pressure_changed(runtime_id, Platform::Linux, level)
}

/// Route one Linux thermal state ingress notification.
pub(crate) fn linux_notify_thermal_state_changed(
    runtime_id: u64,
    state: HostThermalState,
) -> RuntimeResult<()> {
    unix_notify_thermal_state_changed(runtime_id, Platform::Linux, state)
}

/// Route one Linux power mode ingress notification.
pub(crate) fn linux_notify_power_mode_changed(
    runtime_id: u64,
    mode: HostPowerMode,
) -> RuntimeResult<()> {
    unix_notify_power_mode_changed(runtime_id, Platform::Linux, mode)
}

/// Route one Linux wall clock ingress notification.
pub(crate) fn linux_notify_wall_clock_changed(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wall_clock_changed(runtime_id, Platform::Linux)
}

/// Wake one blocked host poll operation for Linux.
pub(crate) fn linux_notify_wake(runtime_id: u64) -> RuntimeResult<()> {
    unix_notify_wake(runtime_id, Platform::Linux)
}
