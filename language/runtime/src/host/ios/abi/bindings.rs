use crate::host::HOST_STATUS_NOT_SUPPORTED;
use crate::host::core::{HostSessionHandle, HostSessionId};
use crate::host::ios::abi::background::callbacks::IosHostBackgroundCallbacks;
use crate::host::ios::abi::calendar::callbacks::IosHostCalendarCallbacks;
use crate::host::ios::abi::contact::callbacks::IosHostContactCallbacks;
use crate::host::ios::abi::document::callbacks::IosHostDocumentCallbacks;
use crate::host::ios::abi::intent::callbacks::IosHostIntentCallbacks;
use crate::host::ios::abi::location::callbacks::IosHostLocationCallbacks;
use crate::host::ios::abi::media::callbacks::IosHostMediaCallbacks;
use crate::host::ios::abi::notification::callbacks::IosHostNotificationCallbacks;
use crate::host::ios::abi::permission::callbacks::IosHostPermissionCallbacks;
use crate::host::ios::abi::registry::{
    register_ios_bindings, resolve_ios_bindings, unregister_ios_bindings,
};

/// iOS host bindings container for callback-backed lanes.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostBindings {
    /// Document host callbacks.
    pub document: IosHostDocumentCallbacks,
    /// Permission host callbacks.
    pub permission: IosHostPermissionCallbacks,
    /// Background host callbacks.
    pub background: IosHostBackgroundCallbacks,
    /// Calendar host callbacks.
    pub calendar: IosHostCalendarCallbacks,
    /// Contact host callbacks.
    pub contact: IosHostContactCallbacks,
    /// Intent host callbacks.
    pub intent: IosHostIntentCallbacks,
    /// Location host callbacks.
    pub location: IosHostLocationCallbacks,
    /// Media host callbacks.
    pub media: IosHostMediaCallbacks,
    /// Notification host callbacks.
    pub notification: IosHostNotificationCallbacks,
}

/// Register one callback table for iOS host interop through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_register_bindings(
    session_handle: HostSessionHandle,
    bindings: IosHostBindings,
) -> u32 {
    register_ios_bindings(session_handle, bindings)
}

/// Remove one callback table for iOS host interop through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_unregister_bindings(
    session_handle: HostSessionHandle,
) {
    let session_id = HostSessionId::from_handle(session_handle);

    unregister_ios_bindings(session_id);
}

/// Resolve one callback from one iOS host bindings lane.
pub(crate) fn resolve_ios_binding_callback<T: Copy>(
    session_handle: HostSessionHandle,
    resolve: impl FnOnce(&IosHostBindings) -> Option<T>,
) -> Result<T, u32> {
    let bindings = resolve_ios_bindings(session_handle)?;

    resolve(&bindings).ok_or(HOST_STATUS_NOT_SUPPORTED)
}

/// Resolve and invoke one iOS host callback.
pub(crate) fn invoke_ios_binding_callback<T: Copy>(
    session_handle: HostSessionHandle,
    resolve: impl FnOnce(&IosHostBindings) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    let callback = match resolve_ios_binding_callback(session_handle, resolve) {
        Ok(callback) => callback,
        Err(status) => return status,
    };

    invoke(callback)
}
