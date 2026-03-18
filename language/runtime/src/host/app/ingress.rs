use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequestContext, HostRuntimeId};

use super::background::{service_background_ingress, unregister_background_runtime};
use super::location::unregister_location_runtime;
use super::media::unregister_media_runtime;
use super::notification::{service_notification_ingress, unregister_notification_runtime};

/// Service shared app ingress for one runtime.
pub(crate) fn service_app_ingress(context: &HostRequestContext) -> RuntimeResult<()> {
    // service background ingress
    service_background_ingress(context.host_runtime_id, context.platform)?;

    // service notification ingress
    service_notification_ingress(context)?;

    Ok(())
}

/// Remove shared app host state for one runtime.
pub(crate) fn unregister_app_runtime(host_runtime_id: HostRuntimeId) {
    // clear background state
    unregister_background_runtime(host_runtime_id);

    // clear location state
    unregister_location_runtime(host_runtime_id);

    // clear media watch state
    unregister_media_runtime(host_runtime_id);

    // clear notification state
    unregister_notification_runtime(host_runtime_id);
}
