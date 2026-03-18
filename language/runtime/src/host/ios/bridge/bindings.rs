use crate::host::HOST_STATUS_NOT_SUPPORTED;
use crate::host::ios::bridge::registry::{register_ios_bindings, resolve_ios_bindings};
use crate::host::ios::request::background::IosHostBackgroundCallbacks;
use crate::host::ios::request::calendar::IosHostCalendarCallbacks;
use crate::host::ios::request::contact::IosHostContactCallbacks;
use crate::host::ios::request::intent::IosHostIntentCallbacks;
use crate::host::ios::request::location::IosHostLocationCallbacks;
use crate::host::ios::request::media::IosHostMediaCallbacks;
use crate::host::ios::request::notification::IosHostNotificationCallbacks;

/// iOS host bindings container for callback-backed lanes.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct IosHostBindings {
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
pub unsafe extern "C" fn destack_host_ios_register_bindings(
    runtime_id: u64,
    bindings: IosHostBindings,
) -> u32 {
    register_ios_bindings(runtime_id, bindings)
}

/// Resolve one callback from one iOS host bindings lane.
pub(crate) fn resolve_ios_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostBindings) -> Option<T>,
) -> Result<T, u32> {
    let bindings = resolve_ios_bindings(runtime_id)?;

    resolve(&bindings).ok_or(HOST_STATUS_NOT_SUPPORTED)
}

/// Resolve and invoke one iOS host callback.
pub(crate) fn invoke_ios_binding_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostBindings) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    let callback = match resolve_ios_binding_callback(runtime_id, resolve) {
        Ok(callback) => callback,
        Err(status) => return status,
    };

    invoke(callback)
}
