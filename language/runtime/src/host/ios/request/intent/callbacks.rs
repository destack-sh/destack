use crate::host::ios::bridge::bindings::invoke_ios_binding_callback;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// Host callback for querying one outbound iOS URL route.
pub(crate) type IosHostIntentCanOpenUrlCallback =
    unsafe extern "C" fn(runtime_id: u64, url: NativeStringRef, is_supported: *mut bool) -> u32;

/// Host callback for opening one outbound iOS URL route.
pub(crate) type IosHostIntentOpenUrlCallback =
    unsafe extern "C" fn(runtime_id: u64, url: NativeStringRef) -> u32;

/// Host callback for opening one outbound iOS path route.
pub(crate) type IosHostIntentOpenPathCallback =
    unsafe extern "C" fn(runtime_id: u64, path: NativeStringRef) -> u32;

/// Host callback for sharing one outbound iOS text payload.
pub(crate) type IosHostIntentShareTextCallback = unsafe extern "C" fn(
    runtime_id: u64,
    text: NativeStringRef,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32;

/// Host callback for sharing one outbound iOS path list.
pub(crate) type IosHostIntentSharePathsCallback = unsafe extern "C" fn(
    runtime_id: u64,
    paths: NativeStringSlice,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32;

/// Callback table for iOS host intent request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostIntentCallbacks {
    /// Callback for `intentCanOpenUrl`.
    pub can_open_url: Option<IosHostIntentCanOpenUrlCallback>,
    /// Callback for `intentOpenUrl`.
    pub open_url: Option<IosHostIntentOpenUrlCallback>,
    /// Callback for `intentOpenPath`.
    pub open_path: Option<IosHostIntentOpenPathCallback>,
    /// Callback for `intentShareText`.
    pub share_text: Option<IosHostIntentShareTextCallback>,
    /// Callback for `intentSharePaths`.
    pub share_paths: Option<IosHostIntentSharePathsCallback>,
}

/// Resolve and invoke one iOS host intent callback.
pub(super) fn call_ios_intent_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostIntentCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.intent), invoke)
}
