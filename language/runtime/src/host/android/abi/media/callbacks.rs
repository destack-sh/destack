use crate::host::abi::media::{HostMediaAssetDescriptor, HostMediaPage, HostMediaQuery};
use crate::host::android::abi::bindings::invoke_android_binding_callback;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// Host callback for listing Android media assets.
pub(crate) type AndroidHostMediaListCallback = unsafe extern "C" fn(
    runtime_id: u64,
    query: HostMediaQuery,
    output_page: *mut HostMediaPage,
) -> u32;

/// Host callback for describing one Android media asset.
pub(crate) type AndroidHostMediaDescribeCallback = unsafe extern "C" fn(
    runtime_id: u64,
    id: NativeStringRef,
    output_descriptor: *mut HostMediaAssetDescriptor,
) -> u32;

/// Host callback for importing one path into the Android media library.
pub(crate) type AndroidHostMediaImportPathCallback = unsafe extern "C" fn(
    runtime_id: u64,
    path: NativeStringRef,
    kind: i32,
    output_id: *mut NativeStringRef,
) -> u32;

/// Host callback for deleting Android media assets.
pub(crate) type AndroidHostMediaDeleteCallback =
    unsafe extern "C" fn(runtime_id: u64, ids: NativeStringSlice, deleted_count: *mut u32) -> u32;

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
}

/// Resolve and invoke one Android host media callback.
pub(super) fn call_android_media_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostMediaCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.media), invoke)
}
