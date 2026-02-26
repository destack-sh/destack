use parking_lot::RwLock;
use std::sync::OnceLock;

use super::abi::{
    HOST_STATUS_INVALID_ARGUMENT, HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
};
use crate::platform::{NativeSlice, NativeStringRef};
use crate::runtime::host::HostPlatform;
use crate::runtime::host::core::host_bridge_for_runtime;

/// ABI version for the Android host-credentials callback table.
pub(super) const ANDROID_HOST_CREDENTIALS_CALLBACKS_ABI_VERSION: u32 = 1;

/// Host callback for reading one credential payload.
pub type AndroidHostCredentialsReadCallback = unsafe extern "C" fn(
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
pub type AndroidHostCredentialsWriteCallback = unsafe extern "C" fn(
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
pub type AndroidHostCredentialsDeleteCallback = unsafe extern "C" fn(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
) -> u32;
/// Host callback for checking one credential payload.
pub type AndroidHostCredentialsContainsCallback = unsafe extern "C" fn(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    is_present: *mut bool,
) -> u32;
/// Host callback for running one credentials authentication challenge.
pub type AndroidHostCredentialsAuthenticateCallback = unsafe extern "C" fn(
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
pub struct AndroidHostCredentialsCallbacks {
    /// ABI version for this callback table.
    pub abi_version: u32,
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
            abi_version: ANDROID_HOST_CREDENTIALS_CALLBACKS_ABI_VERSION,
            read: None,
            write: None,
            delete: None,
            contains: None,
            authenticate: None,
        }
    }
}

/// Return the shared Android host-credentials callback registry.
fn android_host_credentials_callbacks() -> &'static RwLock<AndroidHostCredentialsCallbacks> {
    static CALLBACKS: OnceLock<RwLock<AndroidHostCredentialsCallbacks>> = OnceLock::new();

    CALLBACKS.get_or_init(|| RwLock::new(AndroidHostCredentialsCallbacks::default()))
}

/// Return whether one callback table uses the expected ABI version.
fn callbacks_abi_is_supported(callbacks: &AndroidHostCredentialsCallbacks) -> bool {
    callbacks.abi_version == ANDROID_HOST_CREDENTIALS_CALLBACKS_ABI_VERSION
}

/// Return whether one runtime identifier resolves to one live Android host bridge.
fn runtime_id_is_registered(runtime_id: u64) -> bool {
    host_bridge_for_runtime(runtime_id, HostPlatform::Android).is_ok()
}

/// Return the callback-table ABI version for Android host credentials interop.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_credentials_callbacks_abi_version() -> u32 {
    ANDROID_HOST_CREDENTIALS_CALLBACKS_ABI_VERSION
}

/// Set one callback table for Android host credentials interop.
pub fn set_android_host_credentials_callbacks(callbacks: AndroidHostCredentialsCallbacks) {
    // replace the entire callback table atomically
    let mut stored_callbacks = android_host_credentials_callbacks().write();
    *stored_callbacks = callbacks;
}

/// Clear Android host credentials callbacks for one test reset.
#[cfg(test)]
fn clear_android_host_credentials_callbacks() {
    set_android_host_credentials_callbacks(AndroidHostCredentialsCallbacks::default());
}

/// Set one callback table for Android host credentials interop through one C ABI entrypoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_credentials_set_callbacks(
    callbacks: AndroidHostCredentialsCallbacks,
) -> u32 {
    // reject callback tables built against one incompatible ABI version
    if !callbacks_abi_is_supported(&callbacks) {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    set_android_host_credentials_callbacks(callbacks);

    HOST_STATUS_OK
}

/// Read one Android host credential payload.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_credentials_read(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
    require_authentication: bool,
    output: NativeSlice<u8>,
    output_written: *mut u32,
    created_unix_ns: *mut u64,
    modified_unix_ns: *mut u64,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // require writable output pointers
    if output_written.is_null() || created_unix_ns.is_null() || modified_unix_ns.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // route one callback when available
    let callbacks = android_host_credentials_callbacks().read();
    let Some(callback) = callbacks.read else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe {
        callback(
            runtime_id,
            service,
            account,
            access_group,
            require_authentication,
            output,
            output_written,
            created_unix_ns,
            modified_unix_ns,
        )
    }
}

/// Write one Android host credential payload.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_credentials_write(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
    payload: NativeSlice<u8>,
    accessibility: u32,
    authentication_policy: u32,
    replace_existing: bool,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_credentials_callbacks().read();
    let Some(callback) = callbacks.write else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe {
        callback(
            runtime_id,
            service,
            account,
            access_group,
            payload,
            accessibility,
            authentication_policy,
            replace_existing,
        )
    }
}

/// Delete one Android host credential payload.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_credentials_delete(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // route one callback when available
    let callbacks = android_host_credentials_callbacks().read();
    let Some(callback) = callbacks.delete else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe { callback(runtime_id, service, account) }
}

/// Query one Android host credential payload.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_credentials_contains(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    is_present: *mut bool,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // require one writable output pointer
    if is_present.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // route one callback when available
    let callbacks = android_host_credentials_callbacks().read();
    let Some(callback) = callbacks.contains else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe { callback(runtime_id, service, account, is_present) }
}

/// Run one Android host credentials authentication challenge.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_runtime_host_android_credentials_authenticate(
    runtime_id: u64,
    title: NativeStringRef,
    subtitle: NativeStringRef,
    message: NativeStringRef,
    requirement: u32,
    authenticated: *mut bool,
    mechanism: *mut u32,
) -> u32 {
    // reject unknown runtime identifiers
    if !runtime_id_is_registered(runtime_id) {
        return HOST_STATUS_NOT_FOUND;
    }

    // require writable output pointers
    if authenticated.is_null() || mechanism.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // route one callback when available
    let callbacks = android_host_credentials_callbacks().read();
    let Some(callback) = callbacks.authenticate else {
        return HOST_STATUS_NOT_SUPPORTED;
    };

    unsafe {
        callback(
            runtime_id,
            title,
            subtitle,
            message,
            requirement,
            authenticated,
            mechanism,
        )
    }
}

#[cfg(test)]
#[path = "tests/credentials.rs"]
mod tests;
