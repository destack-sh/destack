use crate::host::core::HostRuntimeId;
use crate::host::macos::unregister_location_runtime as unregister_macos_location_runtime;

/// Remove location state for one runtime id.
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    unregister_macos_location_runtime(host_runtime_id);
}
