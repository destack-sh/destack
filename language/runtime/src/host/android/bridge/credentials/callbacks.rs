use super::super::bindings::invoke_android_binding_callback;
use crate::runtime::{NativeSlice, NativeStringRef};

/// Host callback for reading one credential payload.
pub(crate) type AndroidHostCredentialsReadCallback = unsafe extern "C" fn(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
    require_authentication: bool,
    output: NativeSlice<u8>,
    output_written: *mut u32,
    created_unix_ns: *mut u64,
    modified_unix_ns: *mut u64,
) -> u32;

/// Host callback for writing one credential payload.
pub(crate) type AndroidHostCredentialsWriteCallback = unsafe extern "C" fn(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
    payload: NativeSlice<u8>,
    accessibility: u32,
    authentication_policy: u32,
    replace_existing: bool,
) -> u32;

/// Host callback for deleting one credential payload.
pub(crate) type AndroidHostCredentialsDeleteCallback = unsafe extern "C" fn(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
) -> u32;

/// Host callback for checking one credential payload.
pub(crate) type AndroidHostCredentialsContainsCallback = unsafe extern "C" fn(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
    is_present: *mut bool,
) -> u32;

/// Host callback for running one credentials authentication challenge.
pub(crate) type AndroidHostCredentialsAuthenticateCallback = unsafe extern "C" fn(
    runtime_id: u64,
    title: NativeStringRef,
    subtitle: NativeStringRef,
    message: NativeStringRef,
    requirement: u32,
    authenticated: *mut bool,
    mechanism: *mut u32,
) -> u32;

/// Callback table for Android host credentials interop.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct AndroidHostCredentialsCallbacks {
    /// Read callback for one credential payload.
    pub read: Option<AndroidHostCredentialsReadCallback>,
    /// Write callback for one credential payload.
    pub write: Option<AndroidHostCredentialsWriteCallback>,
    /// Delete callback for one credential payload.
    pub delete: Option<AndroidHostCredentialsDeleteCallback>,
    /// Contains callback for one credential payload.
    pub contains: Option<AndroidHostCredentialsContainsCallback>,
    /// Authenticate callback for one host challenge.
    pub authenticate: Option<AndroidHostCredentialsAuthenticateCallback>,
}

impl Default for AndroidHostCredentialsCallbacks {
    /// Build one callback table with no handlers.
    fn default() -> Self {
        Self {
            read: None,
            write: None,
            delete: None,
            contains: None,
            authenticate: None,
        }
    }
}

/// Resolve and invoke one Android host credentials callback.
pub(crate) fn call_android_credentials_callback<T: Copy>(
    runtime_id: u64,
    resolve: impl FnOnce(&AndroidHostCredentialsCallbacks) -> Option<T>,
    invoke: impl FnOnce(T) -> u32,
) -> u32 {
    invoke_android_binding_callback(
        runtime_id,
        |bindings| resolve(&bindings.credentials),
        invoke,
    )
}
