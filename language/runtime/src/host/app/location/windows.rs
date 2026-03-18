use crate::host::core::HostRuntimeId;
use crate::host::windows::unregister_location_runtime as unregister_windows_location_runtime;

/// Remove location state for one runtime id.
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    unregister_windows_location_runtime(host_runtime_id);
}
