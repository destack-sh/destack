use crate::diagnostic::RuntimeResult;
use crate::platform::core::{self as core_platform};

use super::super::executor::thread::ServiceThreadGuard;

/// Initialize one WinRT multithreaded apartment for one service thread.
pub(crate) fn initialize_windows_winrt_mta(name: &str) -> RuntimeResult<ServiceThreadGuard> {
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};

    match unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        Ok(()) => Ok(ServiceThreadGuard::WindowsMta),
        Err(error) if error.code() == RPC_E_CHANGED_MODE => Ok(ServiceThreadGuard::None),
        Err(error) => Err(core_platform::io_operation_error(
            "platform.service.bootstrap",
            None,
            format!("failed to initialize WinRT MTA for {name}: {error}"),
        )),
    }
}
