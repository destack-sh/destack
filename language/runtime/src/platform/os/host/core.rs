#[cfg(unix)]
use std::ffi::CStr;

use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::{HostIdentity, HostIdentityVm};
use crate::platform::{PlatformError, core as core_platform};
use crate::runtime::BindingCallContext;

/// Binding operation name for host identity reads.
pub(crate) const OS_HOST_IDENTITY_OPERATION: &str = "destack.os.host.identity";

/// Decoded host identity payload.
#[derive(Debug, Clone)]
pub(super) struct HostIdentityOwned {
    /// Hostname value.
    pub hostname: String,
    /// Kernel family value.
    pub kernel: String,
    /// Kernel release value.
    pub release: String,
    /// Host architecture value.
    pub architecture: String,
}

/// Build one ioInvalidData runtime error.
pub(super) fn invalid_data(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

#[cfg(unix)]
/// Decode one nul-terminated c string pointer into owned UTF-8 text.
pub(super) fn cstring_from_ptr(
    pointer: *const libc::c_char,
    field: &'static str,
    operation: &'static str,
) -> RuntimeResult<String> {
    // reject null pointers from host APIs
    if pointer.is_null() {
        return Err(invalid_data(operation, format!("{field} pointer was null")));
    }

    // decode one host c string and validate utf-8 payload
    let bytes = unsafe { CStr::from_ptr(pointer) };
    let value = bytes
        .to_str()
        .map_err(|_| invalid_data(operation, format!("{field} contained non utf-8 host bytes")))?;

    Ok(value.to_string())
}

#[cfg(windows)]
/// Decode one utf16 host buffer into one utf8 string.
pub(super) fn decode_utf16_buffer(
    operation: &'static str,
    field: &'static str,
    buffer: &[u16],
) -> RuntimeResult<String> {
    let value = String::from_utf16(buffer).map_err(|_| {
        invalid_data(
            operation,
            format!("{field} contained invalid utf16 host payload"),
        )
    })?;

    Ok(value)
}

/// Read host identity.
pub(crate) unsafe fn destack_os_host_identity(
    binding: &BindingCallContext,
    out: *mut HostIdentity,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    core_platform::ensure_out(out, "out")?;

    // read one normalized host identity payload
    let identity = super::target::read_host_identity(binding)?;

    // encode output payload in call-local runtime storage
    let output = HostIdentity {
        hostname: binding.store_string(&identity.hostname),
        kernel: binding.store_string(&identity.kernel),
        release: binding.store_string(&identity.release),
        architecture: binding.store_string(&identity.architecture),
    };

    // write the output payload
    unsafe {
        out.write(output);
    }

    Ok(())
}

/// Read host identity through the VM ABI surface.
pub(crate) fn destack_os_host_identity_vm(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<HostIdentityVm> {
    // read one normalized host identity payload
    let identity = super::target::read_host_identity(binding)?;

    // encode the payload in the VM call context
    let output = HostIdentityVm {
        hostname: vm::StringHandle::new(context.intern_string(&identity.hostname)?),
        kernel: vm::StringHandle::new(context.intern_string(&identity.kernel)?),
        release: vm::StringHandle::new(context.intern_string(&identity.release)?),
        architecture: vm::StringHandle::new(context.intern_string(&identity.architecture)?),
    };

    Ok(output)
}
