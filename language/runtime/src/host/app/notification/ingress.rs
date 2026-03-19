use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequestContext, HostRuntimeId};

use super::backend;
use super::runtime::notification_registry;

/// Service notification ingress for one runtime.
pub(crate) fn service_notification_ingress(context: &HostRequestContext) -> RuntimeResult<()> {
    backend::service_notification_ingress(context)
}

/// Remove notification state for one runtime id.
pub(crate) fn unregister_notification_runtime(host_runtime_id: HostRuntimeId) {
    backend::unregister_runtime(host_runtime_id);

    let registry = notification_registry();
    let mut registry = registry.lock();
    registry.runtimes.remove(&host_runtime_id);
}
