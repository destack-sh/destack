use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::host::core::{
    HostIdentityOwned, OS_HOST_IDENTITY_OPERATION, cstring_from_ptr,
};
use crate::runtime::BindingCallContext;

/// Read one host identity payload from unix APIs.
pub(crate) fn read_host_identity(
    _binding: &BindingCallContext,
) -> RuntimeResult<HostIdentityOwned> {
    // query host uname payload
    let mut uname = MaybeUninit::<libc::utsname>::zeroed();
    let status = unsafe { libc::uname(uname.as_mut_ptr()) };
    if status != 0 {
        return Err(core_platform::io_error("uname", None));
    }
    let uname = unsafe { uname.assume_init() };

    // decode host identity fields from uname strings
    let hostname = cstring_from_ptr(
        uname.nodename.as_ptr(),
        "hostname",
        OS_HOST_IDENTITY_OPERATION,
    )?;
    let kernel = cstring_from_ptr(uname.sysname.as_ptr(), "kernel", OS_HOST_IDENTITY_OPERATION)?;
    let release = cstring_from_ptr(
        uname.release.as_ptr(),
        "release",
        OS_HOST_IDENTITY_OPERATION,
    )?;
    let architecture = cstring_from_ptr(
        uname.machine.as_ptr(),
        "architecture",
        OS_HOST_IDENTITY_OPERATION,
    )?;

    Ok(HostIdentityOwned {
        hostname,
        kernel,
        release,
        architecture,
    })
}
