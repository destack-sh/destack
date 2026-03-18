use crate::host::core::HostRuntimeId;
use crate::host::unix::unregister_unix_location_runtime;

/// Remove location state for one runtime id.
pub(crate) fn unregister_location_runtime(host_runtime_id: HostRuntimeId) {
    unregister_unix_location_runtime(host_runtime_id);
}
