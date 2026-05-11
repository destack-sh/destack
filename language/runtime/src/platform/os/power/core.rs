use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

/// Binding operation name for power-state reads.
#[cfg(not(target_os = "android"))]
pub(crate) const OS_POWER_STATE_OPERATION: &str = "destack.os.power.state";
/// Binding operation name for suspend requests.
pub(crate) const OS_POWER_SUSPEND_OPERATION: &str = "destack.os.power.suspend";

/// Read current host power state.
pub(crate) unsafe fn destack_os_power_state(
    binding: &BindingCallContext,
    out: *mut PowerState,
) -> RuntimeResult<()> {
    // validate output argument before host calls
    core_platform::ensure_out(out, "out")?;

    // read one normalized host power state and write output
    let state = super::target::read_power_state(binding)?;
    unsafe {
        out.write(state);
    }

    Ok(())
}

/// Read current host power state through the VM ABI surface.
pub(crate) fn read_power_state(binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    super::target::read_power_state(binding)
}

/// Request one host suspend transition through the active backend.
pub(crate) unsafe fn destack_os_suspend(binding: &BindingCallContext) -> RuntimeResult<()> {
    super::target::request_suspend(binding)
}

/// Request one host suspend transition through the VM ABI surface.
pub(crate) fn suspend(binding: &BindingCallContext) -> RuntimeResult<()> {
    super::target::request_suspend(binding)
}
