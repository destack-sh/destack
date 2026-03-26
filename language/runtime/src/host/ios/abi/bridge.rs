use crate::host::core::{HostSessionHandle, HostSessionId};
use crate::host::ios::abi::background::callbacks::IosHostBackgroundCallbacks;
use crate::host::ios::abi::bindings::IosHostBindings;
use crate::host::ios::abi::calendar::callbacks::IosHostCalendarCallbacks;
use crate::host::ios::abi::contact::callbacks::IosHostContactCallbacks;
use crate::host::ios::abi::document::callbacks::IosHostDocumentCallbacks;
use crate::host::ios::abi::intent::callbacks::IosHostIntentCallbacks;
use crate::host::ios::abi::location::callbacks::IosHostLocationCallbacks;
use crate::host::ios::abi::media::callbacks::IosHostMediaCallbacks;
use crate::host::ios::abi::notification::callbacks::IosHostNotificationCallbacks;
use crate::host::ios::abi::permission::callbacks::IosHostPermissionCallbacks;
use crate::host::ios::abi::registry::{register_ios_bindings, unregister_ios_bindings};
use crate::host::ios::abi::text::callbacks::IosHostTextCallbacks;

/// iOS runtime-bridge bindings for the mobile host lanes.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosRuntimeBridgeBindings {
    /// The document host callbacks.
    pub document: IosHostDocumentCallbacks,
    /// The permission host callbacks.
    pub permission: IosHostPermissionCallbacks,
    /// The calendar host callbacks.
    pub calendar: IosHostCalendarCallbacks,
    /// The contact host callbacks.
    pub contact: IosHostContactCallbacks,
    /// The intent host callbacks.
    pub intent: IosHostIntentCallbacks,
    /// The location host callbacks.
    pub location: IosHostLocationCallbacks,
    /// The media host callbacks.
    pub media: IosHostMediaCallbacks,
    /// The notification host callbacks.
    pub notification: IosHostNotificationCallbacks,
    /// The text host callbacks.
    pub text: IosHostTextCallbacks,
    /// The background host callbacks.
    pub background: IosHostBackgroundCallbacks,
}

/// Register one iOS runtime-bridge callback table through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_register_runtime_bridge_bindings(
    session_handle: HostSessionHandle,
    bindings: IosRuntimeBridgeBindings,
) -> u32 {
    let bindings = IosHostBindings {
        document: bindings.document,
        permission: bindings.permission,
        text: bindings.text,
        background: bindings.background,
        calendar: bindings.calendar,
        contact: bindings.contact,
        intent: bindings.intent,
        location: bindings.location,
        media: bindings.media,
        notification: bindings.notification,
        ..IosHostBindings::default()
    };

    register_ios_bindings(session_handle, bindings)
}

/// Remove one iOS runtime-bridge callback table through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_ios_unregister_runtime_bridge_bindings(
    session_handle: HostSessionHandle,
) {
    let session_id = HostSessionId::from_handle(session_handle);

    unregister_ios_bindings(session_id);
}
