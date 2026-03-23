pub(crate) mod notify;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostSessionContext, HostSessionId};
use crate::host::macos::request::location::unregister_location_runtime;
use crate::platform::os::background::{service_background_ingress, unregister_background_runtime};
use crate::platform::os::notification::{
    service_notification_ingress, unregister_notification_runtime,
};

/// Service runtime-owned macOS ingress.
pub(crate) fn service_macos_ingress(context: &HostSessionContext) -> RuntimeResult<()> {
    service_background_ingress(context.host_session_id, context.platform)?;
    service_notification_ingress(context)
}

/// Remove one macOS runtime from shared host state.
pub(crate) fn unregister_macos_runtime(host_session_id: HostSessionId) {
    unregister_background_runtime(host_session_id);
    unregister_location_runtime(host_session_id);
    unregister_notification_runtime(host_session_id);
}
