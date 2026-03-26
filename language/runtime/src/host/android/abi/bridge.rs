use crate::host::android::abi::background::callbacks::AndroidHostBackgroundCallbacks;
use crate::host::android::abi::bindings::AndroidHostBindings;
use crate::host::android::abi::calendar::callbacks::AndroidHostCalendarCallbacks;
use crate::host::android::abi::contact::callbacks::AndroidHostContactCallbacks;
use crate::host::android::abi::document::callbacks::AndroidHostDocumentCallbacks;
use crate::host::android::abi::intent::callbacks::AndroidHostIntentCallbacks;
use crate::host::android::abi::location::callbacks::AndroidHostLocationCallbacks;
use crate::host::android::abi::media::callbacks::AndroidHostMediaCallbacks;
use crate::host::android::abi::notification::callbacks::AndroidHostNotificationCallbacks;
use crate::host::android::abi::permission::callbacks::AndroidHostPermissionCallbacks;
use crate::host::android::abi::registry::{register_android_bindings, unregister_android_bindings};
use crate::host::android::abi::text::callbacks::AndroidHostTextCallbacks;
use crate::host::core::HostSessionHandle;

/// Android runtime-bridge bindings for the mobile host lanes.
///
/// This layout must stay in lockstep with `AndroidRuntimeBridgeBindings` in `bridge/types.h`.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidRuntimeBridgeBindings {
    /// The document host callbacks.
    pub document: AndroidHostDocumentCallbacks,
    /// The permission host callbacks.
    pub permission: AndroidHostPermissionCallbacks,
    /// The calendar host callbacks.
    pub calendar: AndroidHostCalendarCallbacks,
    /// The contact host callbacks.
    pub contact: AndroidHostContactCallbacks,
    /// The intent host callbacks.
    pub intent: AndroidHostIntentCallbacks,
    /// The location host callbacks.
    pub location: AndroidHostLocationCallbacks,
    /// The media host callbacks.
    pub media: AndroidHostMediaCallbacks,
    /// The notification host callbacks.
    pub notification: AndroidHostNotificationCallbacks,
    /// The text host callbacks.
    pub text: AndroidHostTextCallbacks,
    /// The background host callbacks.
    pub background: AndroidHostBackgroundCallbacks,
}

/// Register one Android runtime-bridge callback table through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_register_runtime_bridge_bindings(
    session_handle: HostSessionHandle,
    bindings: AndroidRuntimeBridgeBindings,
) -> u32 {
    let bindings = AndroidHostBindings {
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
        ..AndroidHostBindings::default()
    };

    register_android_bindings(session_handle, bindings)
}

/// Remove one Android runtime-bridge callback table through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_unregister_runtime_bridge_bindings(
    session_handle: HostSessionHandle,
) {
    unregister_android_bindings(session_handle);
}
