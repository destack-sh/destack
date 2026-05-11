use std::ffi::c_void;

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Threading::{
    INFINITE, RegisterWaitForSingleObject, UnregisterWaitEx, WAITORTIMERCALLBACK,
    WT_EXECUTEONLYONCE,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, PlatformErrorCode, core as core_platform};

/// One registered Windows threadpool wait.
pub(crate) struct WindowsRegisteredWait {
    /// Native wait registration handle.
    handle: HANDLE,
}

impl WindowsRegisteredWait {
    /// Register one one-shot wait callback on one waitable handle.
    pub(crate) fn register(
        operation: &'static str,
        waitable_handle: HANDLE,
        callback: WAITORTIMERCALLBACK,
        context: *mut c_void,
    ) -> RuntimeResult<Self> {
        let mut handle = 0;
        let status = unsafe {
            RegisterWaitForSingleObject(
                &mut handle,
                waitable_handle,
                callback,
                context,
                INFINITE,
                WT_EXECUTEONLYONCE,
            )
        };
        if status == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::Io),
                None,
                Some(core_platform::last_error_code()),
                Some(operation.to_string()),
                Some(String::from("RegisterWaitForSingleObject")),
                "failed to register windows wait callback",
            ))
            .boxed());
        }

        Ok(Self { handle })
    }

    /// Unregister this wait and wait for in-flight callbacks to retire.
    pub(crate) fn unregister(&mut self, operation: &'static str) -> RuntimeResult<()> {
        if self.handle == 0 {
            return Ok(());
        }

        let handle = std::mem::replace(&mut self.handle, 0);
        let status = unsafe { UnregisterWaitEx(handle, INVALID_HANDLE_VALUE) };
        if status == 0 {
            return Err(RuntimeError::from(PlatformError::io_with(
                Some(PlatformErrorCode::Io),
                None,
                Some(core_platform::last_error_code()),
                Some(operation.to_string()),
                Some(String::from("UnregisterWaitEx")),
                "failed to unregister windows wait callback",
            ))
            .boxed());
        }

        Ok(())
    }
}
