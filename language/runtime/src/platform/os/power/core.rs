use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

use super::backend;

#[cfg(not(any(unix, windows)))]
/// Binding operation name for power-state reads.
pub(crate) const OS_POWER_STATE_OPERATION: &str = "destack.os.power.state";

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
    binding: &BindingCallContext,
    out: *mut PowerState,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    core_platform::ensure_out(out, "out")?;

    // read one normalized host power state and write output
    let state = backend::read_power_state(binding)?;
    unsafe {
        out.write(state);
    }

    Ok(())
}
