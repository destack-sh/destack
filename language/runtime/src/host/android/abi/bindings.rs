#![allow(unreachable_pub)]

use crate::host::android::abi::background::callbacks::AndroidHostBackgroundCallbacks;
use crate::host::android::abi::bluetooth::types::AndroidHostBluetoothCallbacks;
use crate::host::android::abi::calendar::callbacks::AndroidHostCalendarCallbacks;
use crate::host::android::abi::camera::types::AndroidHostCameraCallbacks;
use crate::host::android::abi::contact::callbacks::AndroidHostContactCallbacks;
use crate::host::android::abi::credentials::callbacks::AndroidHostCredentialsCallbacks;
use crate::host::android::abi::crypto::callbacks::AndroidHostCryptoCallbacks;
use crate::host::android::abi::intent::callbacks::AndroidHostIntentCallbacks;
use crate::host::android::abi::location::callbacks::AndroidHostLocationCallbacks;
use crate::host::android::abi::media::callbacks::AndroidHostMediaCallbacks;
use crate::host::android::abi::midi::types::AndroidHostMidiCallbacks;
use crate::host::android::abi::notification::callbacks::AndroidHostNotificationCallbacks;
use crate::host::android::abi::registry::{register_android_bindings, resolve_android_bindings};
use crate::host::android::abi::usb::types::AndroidHostUsbCallbacks;
use crate::host::core::{HostSessionHandle, HostStatus};

/// Android host bindings container for callback-backed lanes.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostBindings {
    /// Background host callbacks.
    pub background: AndroidHostBackgroundCallbacks,
    /// Bluetooth host callbacks.
    pub bluetooth: AndroidHostBluetoothCallbacks,
    /// Calendar host callbacks.
    pub calendar: AndroidHostCalendarCallbacks,
    /// Camera host callbacks.
    pub camera: AndroidHostCameraCallbacks,
    /// Contact host callbacks.
    pub contact: AndroidHostContactCallbacks,
    /// USB host callbacks.
    pub usb: AndroidHostUsbCallbacks,
    /// Intent host callbacks.
    pub intent: AndroidHostIntentCallbacks,
    /// Location host callbacks.
    pub location: AndroidHostLocationCallbacks,
    /// Media host callbacks.
    pub media: AndroidHostMediaCallbacks,
    /// Notification host callbacks.
    pub notification: AndroidHostNotificationCallbacks,
    /// Credentials host callbacks.
    pub credentials: AndroidHostCredentialsCallbacks,
    /// Crypto host callbacks.
    pub crypto: AndroidHostCryptoCallbacks,
    /// MIDI host callbacks.
    pub midi: AndroidHostMidiCallbacks,
}

/// Register one callback table for Android host interop through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_register_bindings(
    session_handle: HostSessionHandle,
    bindings: AndroidHostBindings,
) -> u32 {
    register_android_bindings(session_handle, bindings)
}

/// Resolve one callback from one Android host bindings lane.
pub(crate) fn resolve_android_binding_callback<T: Copy>(
    session_handle: HostSessionHandle,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
) -> Result<T, u32> {
    // resolve one runtime-scoped bindings snapshot
    let bindings = resolve_android_bindings(session_handle)?;

    // resolve one callback and report unsupported lanes explicitly
    resolve(&bindings).ok_or(HostStatus::NotSupported.code())
}

/// Resolve and invoke one Android host callback.
pub(crate) fn invoke_android_binding_callback<T: Copy>(
    session_handle: HostSessionHandle,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    // route one callback and return status for unsupported or unknown runtimes
    let callback = match resolve_android_binding_callback(session_handle, resolve) {
        Ok(callback) => callback,
        Err(status) => return status,
    };

    // invoke one resolved callback
    invoke(callback)
}
