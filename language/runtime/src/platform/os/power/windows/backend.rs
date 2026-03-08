use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::os::PowerState;
use crate::runtime::BindingCallContext;

/// Read one host power-state value from windows APIs.
pub(crate) fn read_power_state(_binding: &BindingCallContext) -> RuntimeResult<PowerState> {
    // query host power-status payload
    let mut status = unsafe { std::mem::zeroed::<SYSTEM_POWER_STATUS>() };
    let result = unsafe { GetSystemPowerStatus(&mut status) };
    if result == 0 {
        return Err(core_platform::io_error("GetSystemPowerStatus"));
    }

    // map windows ac-line status into runtime power-state enum
    let state = match status.ACLineStatus {
        0 => PowerState::Battery,
        1 => PowerState::AC,
        _ => PowerState::Unknown,
    };

    Ok(state)
}
