use crate::host::android::bridge::bindings::invoke_android_binding_callback;
use crate::runtime::{NativeSlice, NativeStringRef};

/// Host callback for listing Android media assets.
pub(crate) type AndroidHostMediaListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for describing one Android media asset.
pub(crate) type AndroidHostMediaDescribeCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for importing one path into the Android media library.
pub(crate) type AndroidHostMediaImportPathCallback = unsafe extern "C" fn(
    runtime_id: u64,
    path: NativeStringRef,
    kind: i32,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for deleting Android media assets.
pub(crate) type AndroidHostMediaDeleteCallback =
    unsafe extern "C" fn(runtime_id: u64, ids: NativeSlice<u8>, deleted_count: *mut u32) -> u32;

/// Host callback for opening one Android media watch.
pub(crate) type AndroidHostMediaWatchOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    watch_id: NativeStringRef,
    options: NativeSlice<u8>,
) -> u32;

/// Host callback for closing one Android media watch.
pub(crate) type AndroidHostMediaWatchCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, watch_id: NativeStringRef) -> u32;

/// Callback table for Android host media request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostMediaCallbacks {
    /// Callback for `mediaList`.
    pub list: Option<AndroidHostMediaListCallback>,
    /// Callback for `mediaDescribe`.
    pub describe: Option<AndroidHostMediaDescribeCallback>,
    /// Callback for `mediaImportPath`.
    pub import_path: Option<AndroidHostMediaImportPathCallback>,
    /// Callback for `mediaDelete`.
    pub delete: Option<AndroidHostMediaDeleteCallback>,
    /// Callback for `mediaWatchOpen`.
    pub watch_open: Option<AndroidHostMediaWatchOpenCallback>,
    /// Callback for `mediaWatchClose`.
    pub watch_close: Option<AndroidHostMediaWatchCloseCallback>,
}

/// Resolve and invoke one Android host media callback.
pub(super) fn call_android_media_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostMediaCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.media), invoke)
}
