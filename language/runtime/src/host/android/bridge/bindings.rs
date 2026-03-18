#![allow(unreachable_pub)]

use crate::host::abi::HostStatus;
use crate::host::android::bluetooth::types::AndroidHostBluetoothCallbacks;
use crate::host::android::bridge::credentials::AndroidHostCredentialsCallbacks;
use crate::host::android::bridge::crypto::AndroidHostCryptoCallbacks;
use crate::host::android::bridge::midi::types::AndroidHostMidiCallbacks;
use crate::host::android::bridge::registry::{register_android_bindings, resolve_android_bindings};
use crate::host::android::camera::types::AndroidHostCameraCallbacks;
use crate::host::android::request::background::AndroidHostBackgroundCallbacks;
use crate::host::android::request::calendar::AndroidHostCalendarCallbacks;
use crate::host::android::request::contact::AndroidHostContactCallbacks;
use crate::host::android::request::intent::AndroidHostIntentCallbacks;
use crate::host::android::request::location::AndroidHostLocationCallbacks;
use crate::host::android::request::media::AndroidHostMediaCallbacks;
use crate::host::android::request::notification::AndroidHostNotificationCallbacks;
use crate::host::android::usb::types::AndroidHostUsbCallbacks;

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
    runtime_id: u64,
    bindings: AndroidHostBindings,
) -> u32 {
    register_android_bindings(runtime_id, bindings)
}

/// Resolve one callback from one Android host bindings lane.
pub(crate) fn resolve_android_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
) -> Result<T, u32> {
    // resolve one runtime-scoped bindings snapshot
    let bindings = resolve_android_bindings(runtime_id)?;

    // resolve one callback and report unsupported lanes explicitly
    resolve(&bindings).ok_or(HostStatus::NotSupported.code())
}

/// Resolve and invoke one Android host callback.
pub(crate) fn invoke_android_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostBindings) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    // route one callback and return status for unsupported or unknown runtimes
    let callback = match resolve_android_binding_callback(runtime_id, resolve) {
        Ok(callback) => callback,
        Err(status) => return status,
    };

    // invoke one resolved callback
    invoke(callback)
}
