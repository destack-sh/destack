use crate::host::android::abi::credentials::callbacks::AndroidHostCredentialsCallbacks;
use crate::host::core::HostStatus;
use crate::runtime::{NativeSlice, NativeStringRef};

/// One test access-group string.
pub(super) const TEST_ACCESS_GROUP: &str = "destack.access-group";

/// One test biometric authentication mechanism code.
pub(super) const TEST_AUTHENTICATION_MECHANISM_BIOMETRIC: u32 = 2;

/// Return whether one requested test access-group is present.
pub(super) unsafe extern "C" fn test_contains(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    access_group: NativeStringRef,
    is_present: *mut bool,
) -> u32 {
    // validate the output pointer before writing through it
    if is_present.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // decode the incoming access-group and report presence
    let access_group = match unsafe { access_group.as_str() } {
        Ok(access_group) => access_group,
        Err(_) => return HostStatus::InvalidArgument.code(),
    };

    unsafe {
        *is_present = access_group == TEST_ACCESS_GROUP;
    }

    HostStatus::Ok.code()
}

/// Write one deterministic credentials payload into the caller buffer.
pub(super) unsafe extern "C" fn test_read(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    _access_group: NativeStringRef,
    _require_authentication: bool,
    output: NativeSlice<u8>,
    output_written: *mut u32,
    created_unix_ns: *mut u64,
    modified_unix_ns: *mut u64,
) -> u32 {
    const PAYLOAD: [u8; 4] = [1, 2, 3, 4];

    // validate the output pointers before writing through them
    if output_written.is_null() || created_unix_ns.is_null() || modified_unix_ns.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // negotiate the required payload size on short buffers
    if output.data.is_null() || output.len < PAYLOAD.len() as u32 {
        unsafe {
            *output_written = PAYLOAD.len() as u32;
        }

        return HostStatus::BufferTooSmall.code();
    }

    // write the full payload and timestamps into the caller buffers
    unsafe {
        std::ptr::copy_nonoverlapping(PAYLOAD.as_ptr(), output.data, PAYLOAD.len());
        *output_written = PAYLOAD.len() as u32;
        *created_unix_ns = 11;
        *modified_unix_ns = 22;
    }

    HostStatus::Ok.code()
}

/// Return one deterministic authentication decision for credentials tests.
pub(super) unsafe extern "C" fn test_authenticate(
    _runtime_id: u64,
    _title: NativeStringRef,
    _subtitle: NativeStringRef,
    _message: NativeStringRef,
    _requirement: u32,
    authenticated: *mut bool,
    mechanism: *mut u32,
) -> u32 {
    // validate the output pointers before writing through them
    if authenticated.is_null() || mechanism.is_null() {
        return HostStatus::InvalidArgument.code();
    }

    // report a successful biometric authentication result
    unsafe {
        *authenticated = true;
        *mechanism = TEST_AUTHENTICATION_MECHANISM_BIOMETRIC;
    }

    HostStatus::Ok.code()
}

/// Return one deterministic credentials callback table for host-bridge tests.
pub(super) fn test_callbacks() -> AndroidHostCredentialsCallbacks {
    AndroidHostCredentialsCallbacks {
        read: Some(test_read),
        write: Some(test_write),
        delete: Some(test_delete),
        contains: Some(test_contains),
        authenticate: Some(test_authenticate),
    }
}

/// Accept one credentials write during host-bridge tests.
unsafe extern "C" fn test_write(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    _access_group: NativeStringRef,
    _payload: NativeSlice<u8>,
    _accessibility: u32,
    _authentication_policy: u32,
    _replace_existing: bool,
) -> u32 {
    HostStatus::Ok.code()
}

/// Accept one credentials delete during host-bridge tests.
unsafe extern "C" fn test_delete(
    _runtime_id: u64,
    _service: NativeStringRef,
    _account: NativeStringRef,
    _access_group: NativeStringRef,
) -> u32 {
    HostStatus::Ok.code()
}
