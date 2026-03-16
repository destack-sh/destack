use crate::host::android::abi::HOST_STATUS_INVALID_ARGUMENT;
use crate::host::android::bridge::credentials::callbacks::call_android_credentials_callback;
use crate::runtime::{NativeSlice, NativeStringRef};

/// Read one Android host credential payload.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_credentials_read(
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
    // require writable output pointers
    if output_written.is_null() || created_unix_ns.is_null() || modified_unix_ns.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // route one callback when available
    call_android_credentials_callback(
        runtime_id,
        |callbacks| callbacks.read,
        |callback| unsafe {
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
        },
    )
}

/// Write one Android host credential payload.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_credentials_write(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
    payload: NativeSlice<u8>,
    accessibility: u32,
    authentication_policy: u32,
    replace_existing: bool,
) -> u32 {
    // route one callback when available
    call_android_credentials_callback(
        runtime_id,
        |callbacks| callbacks.write,
        |callback| unsafe {
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
        },
    )
}

/// Delete one Android host credential payload.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_credentials_delete(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
) -> u32 {
    call_android_credentials_callback(
        runtime_id,
        |callbacks| callbacks.delete,
        |callback| unsafe { callback(runtime_id, service, account, access_group) },
    )
}

/// Query one Android host credential payload.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_credentials_contains(
    runtime_id: u64,
    service: NativeStringRef,
    account: NativeStringRef,
    access_group: NativeStringRef,
    is_present: *mut bool,
) -> u32 {
    // require one writable output pointer
    if is_present.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // route one callback when available
    call_android_credentials_callback(
        runtime_id,
        |callbacks| callbacks.contains,
        |callback| unsafe { callback(runtime_id, service, account, access_group, is_present) },
    )
}

/// Run one Android host credentials authentication challenge.
#[unsafe(no_mangle)]
pub(crate) unsafe extern "C" fn destack_host_android_credentials_authenticate(
    runtime_id: u64,
    title: NativeStringRef,
    subtitle: NativeStringRef,
    message: NativeStringRef,
    requirement: u32,
    authenticated: *mut bool,
    mechanism: *mut u32,
) -> u32 {
    // require writable output pointers
    if authenticated.is_null() || mechanism.is_null() {
        return HOST_STATUS_INVALID_ARGUMENT;
    }

    // route one callback when available
    call_android_credentials_callback(
        runtime_id,
        |callbacks| callbacks.authenticate,
        |callback| unsafe {
            callback(
                runtime_id,
                title,
                subtitle,
                message,
                requirement,
                authenticated,
                mechanism,
            )
        },
    )
}
