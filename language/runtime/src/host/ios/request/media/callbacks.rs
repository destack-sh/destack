use crate::host::ios::bridge::bindings::invoke_ios_binding_callback;
use crate::runtime::{NativeSlice, NativeStringRef};

/// Host callback for listing iOS media assets.
pub(crate) type IosHostMediaListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    payload: NativeSlice<u8>,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for describing one iOS media asset.
pub(crate) type IosHostMediaDescribeCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeStringRef,
    output: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for importing one path into the iOS media library.
pub(crate) type IosHostMediaImportPathCallback = unsafe extern "C" fn(
    runtime_id: u64,
    path: NativeStringRef,
    kind: i32,
    output_id: NativeSlice<u8>,
    output_written: *mut u32,
) -> u32;

/// Host callback for deleting iOS media assets.
pub(crate) type IosHostMediaDeleteCallback =
    unsafe extern "C" fn(runtime_id: u64, ids: NativeSlice<u8>, deleted_count: *mut u32) -> u32;

/// Host callback for opening one iOS media watch.
pub(crate) type IosHostMediaWatchOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    watch_id: NativeStringRef,
    options: NativeSlice<u8>,
) -> u32;

/// Host callback for closing one iOS media watch.
pub(crate) type IosHostMediaWatchCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, watch_id: NativeStringRef) -> u32;

/// Callback table for iOS host media request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostMediaCallbacks {
    /// Callback for `mediaList`.
    pub list: Option<IosHostMediaListCallback>,
    /// Callback for `mediaDescribe`.
    pub describe: Option<IosHostMediaDescribeCallback>,
    /// Callback for `mediaImportPath`.
    pub import_path: Option<IosHostMediaImportPathCallback>,
    /// Callback for `mediaDelete`.
    pub delete: Option<IosHostMediaDeleteCallback>,
    /// Callback for `mediaWatchOpen`.
    pub watch_open: Option<IosHostMediaWatchOpenCallback>,
    /// Callback for `mediaWatchClose`.
    pub watch_close: Option<IosHostMediaWatchCloseCallback>,
}

/// Resolve and invoke one iOS host media callback.
pub(super) fn call_ios_media_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostMediaCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.media), invoke)
}
