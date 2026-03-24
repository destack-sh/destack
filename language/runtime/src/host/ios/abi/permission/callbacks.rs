use crate::host::abi::permission::HostPermissionRequest;
use crate::host::ios::abi::bindings::invoke_ios_binding_callback;

/// Host callback for opening iOS permission settings.
pub(crate) type IosHostPermissionOpenSettingsCallback =
    unsafe extern "C" fn(runtime_id: u64) -> u32;

/// Host callback for one iOS permission request.
pub(crate) type IosHostPermissionRequestCallback =
    unsafe extern "C" fn(runtime_id: u64, request: HostPermissionRequest) -> u32;

/// Callback table for iOS host permission request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostPermissionCallbacks {
    /// Callback for `permissionOpenSettings`.
    pub open_settings: Option<IosHostPermissionOpenSettingsCallback>,
    /// Callback for `permissionRequest`.
    pub request: Option<IosHostPermissionRequestCallback>,
}

/// Resolve and invoke one iOS host permission callback.
pub(super) fn call_ios_permission_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostPermissionCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.permission), invoke)
}
