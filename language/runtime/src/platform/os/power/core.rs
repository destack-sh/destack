use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

use super::backend;

#[cfg(not(any(unix, windows)))]
/// Binding operation name for power-state reads.
pub(crate) const OS_POWER_STATE_OPERATION: &str = "destack.os.power.state";

/// Validate one out pointer argument.
pub(super) fn ensure_out<T>(out: *mut T, field: &'static str) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(field)).boxed());
    }

    Ok(())
}

#[cfg(not(any(unix, windows)))]
/// Build one notSupported runtime error.
pub(super) fn not_supported(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Read current host power state.
///
/// Return one normalized host power-state classification.
/// State mapping follows runtime normalization over host power APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses host power management APIs such as sysfs and IOKit on Unix-like systems and GetSystemPowerStatus on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.power`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_power_state(
    context: &BindingCallContext,
    out: *mut PowerState,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    ensure_out(out, "out")?;

    // read one normalized host power state and write output
    let state = backend::read_power_state(context)?;
    unsafe {
        out.write(state);
    }

    Ok(())
}
