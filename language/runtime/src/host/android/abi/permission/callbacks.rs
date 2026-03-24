use crate::host::abi::permission::HostPermissionRequest;
use crate::host::android::abi::bindings::invoke_android_binding_callback;

/// Host callback for opening Android permission settings.
pub(crate) type AndroidHostPermissionOpenSettingsCallback =
    unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for one Android permission request.
pub(crate) type AndroidHostPermissionRequestCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostPermissionRequest) -> u32;

/// Callback table for Android host permission request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostPermissionCallbacks {
    /// Callback for `permissionOpenSettings`.
    pub open_settings: Option<AndroidHostPermissionOpenSettingsCallback>,
    /// Callback for `permissionRequest`.
    pub request: Option<AndroidHostPermissionRequestCallback>,
}

/// Resolve and invoke one Android host permission callback.
pub(super) fn call_android_permission_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostPermissionCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.permission), invoke)
}
