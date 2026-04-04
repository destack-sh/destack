pub(crate) mod notify;

use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::request::unregister_location_runtime;
use crate::host::{HostSessionId, SessionContext};
use crate::platform::os::background::{service_background_ingress, unregister_background_runtime};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::platform::os::credentials::unregister_credential_runtime;
use crate::platform::os::notification::{
    service_notification_ingress, unregister_notification_runtime,
};

/// Service runtime-owned Linux ingress.
pub(crate) fn service_linux_ingress(context: &SessionContext) -> RuntimeResult<()> {
    service_background_ingress(context.host_session_id, context.platform)?;
    service_notification_ingress(context)
}

/// Remove one Linux runtime from shared host state.
pub(crate) fn unregister_linux_runtime(host_session_id: HostSessionId) {
    unregister_background_runtime(host_session_id);
    unregister_location_runtime(host_session_id);
    unregister_notification_runtime(host_session_id);

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    unregister_credential_runtime(host_session_id.0);
}
