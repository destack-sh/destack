#[cfg(windows)]
pub(crate) mod message;
pub(crate) mod notify;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostSessionContext, HostSessionId};
#[cfg(windows)]
use crate::host::windows::request::location::unregister_location_runtime;
use crate::platform::os::background::{service_background_ingress, unregister_background_runtime};
#[cfg(any(target_os = "linux", target_os = "windows"))]
use crate::platform::os::credentials::unregister_credential_runtime;
use crate::platform::os::notification::{
    service_notification_ingress, unregister_notification_runtime,
};

#[cfg(all(test, not(windows)))]
fn unregister_location_runtime(_host_runtime_id: HostSessionId) {}

#[cfg(windows)]
pub(crate) use message::process_ingress_loop;

/// Service runtime-owned Windows ingress.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn service_windows_ingress(context: &HostSessionContext) -> RuntimeResult<()> {
    service_background_ingress(context.host_session_id, context.platform)?;
    service_notification_ingress(context)
}

/// Remove one Windows runtime from shared host state.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn unregister_windows_runtime(host_session_id: HostSessionId) {
    unregister_background_runtime(host_session_id);
    unregister_location_runtime(host_session_id);
    unregister_notification_runtime(host_session_id);

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    unregister_credential_runtime(host_session_id.0);
}
