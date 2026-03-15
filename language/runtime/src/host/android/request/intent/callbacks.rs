use crate::host::android::bridge::bindings::invoke_android_binding_callback;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// Host callback for querying one outbound Android URL route.
pub type AndroidHostIntentCanOpenUrlCallback =
    unsafe extern "C" fn(runtime_id: u64, url: NativeStringRef, is_supported: *mut bool) -> u32;

/// Host callback for opening one outbound Android URL route.
pub type AndroidHostIntentOpenUrlCallback =
    unsafe extern "C" fn(runtime_id: u64, url: NativeStringRef) -> u32;

/// Host callback for opening one outbound Android path route.
pub type AndroidHostIntentOpenPathCallback =
    unsafe extern "C" fn(runtime_id: u64, path: NativeStringRef) -> u32;

/// Host callback for sharing one outbound Android text payload.
pub type AndroidHostIntentShareTextCallback = unsafe extern "C" fn(
    runtime_id: u64,
    text: NativeStringRef,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32;

/// Host callback for sharing one outbound Android path list.
pub type AndroidHostIntentSharePathsCallback = unsafe extern "C" fn(
    runtime_id: u64,
    paths: NativeStringSlice,
    has_mime_type: bool,
    mime_type: NativeStringRef,
) -> u32;

/// Callback table for Android host intent request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AndroidHostIntentCallbacks {
    /// Callback for `intentCanOpenUrl`.
    pub can_open_url: Option<AndroidHostIntentCanOpenUrlCallback>,
    /// Callback for `intentOpenUrl`.
    pub open_url: Option<AndroidHostIntentOpenUrlCallback>,
    /// Callback for `intentOpenPath`.
    pub open_path: Option<AndroidHostIntentOpenPathCallback>,
    /// Callback for `intentShareText`.
    pub share_text: Option<AndroidHostIntentShareTextCallback>,
    /// Callback for `intentSharePaths`.
    pub share_paths: Option<AndroidHostIntentSharePathsCallback>,
}

/// Resolve and invoke one Android host intent callback.
pub(super) fn call_android_intent_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostIntentCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.intent), invoke)
}
