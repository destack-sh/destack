use crate::host::ios::abi::bindings::invoke_ios_binding_callback;
use crate::platform::os::{LocationSample, LocationWatchOptions};
use crate::runtime::NativeStringRef;

/// Host callback for reading iOS location services state.
pub(crate) type IosHostLocationServicesEnabledCallback =
    unsafe extern "C" fn(runtime_id: u64, is_enabled: *mut bool) -> u32;

/// Host callback for reading one iOS last-known location sample.
pub(crate) type IosHostLocationLastKnownCallback =
    unsafe extern "C" fn(runtime_id: u64, sample: *mut LocationSample) -> u32;

/// Host callback for opening one iOS location watch.
pub(crate) type IosHostLocationWatchOpenCallback = unsafe extern "C" fn(
    runtime_id: u64,
    watch_id: NativeStringRef,
    options: LocationWatchOptions,
) -> u32;

/// Host callback for closing one iOS location watch.
pub(crate) type IosHostLocationWatchCloseCallback =
    unsafe extern "C" fn(runtime_id: u64, watch_id: NativeStringRef) -> u32;

/// Callback table for iOS host location request interop.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct IosHostLocationCallbacks {
    /// Callback for `locationServicesEnabled`.
    pub services_enabled: Option<IosHostLocationServicesEnabledCallback>,
    /// Callback for `locationLastKnown`.
    pub last_known: Option<IosHostLocationLastKnownCallback>,
    /// Callback for `locationWatchOpen`.
    pub watch_open: Option<IosHostLocationWatchOpenCallback>,
    /// Callback for `locationWatchClose`.
    pub watch_close: Option<IosHostLocationWatchCloseCallback>,
}

/// Resolve and invoke one iOS host location callback.
pub(super) fn call_ios_location_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&IosHostLocationCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_ios_binding_callback(runtime_id, |bindings| resolve(&bindings.location), invoke)
}
