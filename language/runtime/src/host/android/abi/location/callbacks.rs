use crate::host::android::abi::bindings::invoke_android_binding_callback;
use crate::platform::os::{LocationSample, LocationWatchOptions};
use crate::runtime::NativeStringRef;

/// Host callback for reading Android location services state.
pub(crate) type AndroidHostLocationServicesEnabledCallback =
    unsafe extern "C" fn(runtime_id: u64, is_enabled: *mut bool) -> u32;

/// Host callback for reading one Android last-known location sample.
pub(crate) type AndroidHostLocationLastKnownCallback =
    unsafe extern "C" fn(runtime_id: u64, sample: *mut LocationSample) -> u32;

/// Host callback for opening one Android location watch.
pub(crate) type AndroidHostLocationWatchOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    watch_id: NativeStringRef,
    options: LocationWatchOptions,
) -> u32;

/// Host callback for closing one Android location watch.
pub(crate) type AndroidHostLocationWatchCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, watch_id: NativeStringRef) -> u32;

/// Callback table for Android host location request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct AndroidHostLocationCallbacks {
    /// Callback for `locationServicesEnabled`.
    pub services_enabled: Option<AndroidHostLocationServicesEnabledCallback>,
    /// Callback for `locationLastKnown`.
    pub last_known: Option<AndroidHostLocationLastKnownCallback>,
    /// Callback for `locationWatchOpen`.
    pub watch_open: Option<AndroidHostLocationWatchOpenCallback>,
    /// Callback for `locationWatchClose`.
    pub watch_close: Option<AndroidHostLocationWatchCloseCallback>,
}

/// Resolve and invoke one Android host location callback.
pub(super) fn call_android_location_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostLocationCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(runtime_id, |bindings| resolve(&bindings.location), invoke)
}
