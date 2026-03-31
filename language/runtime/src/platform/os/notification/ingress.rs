use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostSessionContext, HostSessionId};

#[cfg(target_os = "macos")]
use crate::host::macos::request::notification::backend as notification_backend;
#[cfg(all(
    unix,
    not(any(target_os = "android", target_os = "ios", target_os = "macos"))
))]
use crate::host::unix::request::notification::backend as notification_backend;
#[cfg(windows)]
use crate::host::windows::request::notification::core as notification_backend;
use crate::platform::os::notification::runtime::notification_runtime_service;
#[cfg(not(any(
    windows,
    target_os = "macos",
    all(
        unix,
        not(any(target_os = "android", target_os = "ios", target_os = "macos"))
    )
)))]
use crate::platform::os::notification::unsupported as notification_backend;

/// Service notification ingress for one runtime.
pub(crate) fn service_notification_ingress(context: &HostSessionContext) -> RuntimeResult<()> {
    notification_backend::service_notification_ingress(context)
}

/// Remove notification state for one runtime id.
pub(crate) fn unregister_notification_runtime(host_session_id: HostSessionId) {
    notification_backend::unregister_runtime(host_session_id);

    let service = notification_runtime_service();
    let mut registry = service.registry.lock();
    registry.runtimes.remove(&host_session_id);
}
