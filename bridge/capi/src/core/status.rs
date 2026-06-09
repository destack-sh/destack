use super::{DestackError, clear_error, set_error};

/// C ABI operation status.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestackStatus {
    /// Operation completed successfully.
    Ok = 0,
    /// Operation failed.
    Error = 1,
}

/// Return a C ABI status for one fallible operation.
pub(crate) fn return_status(
    error: *mut *mut DestackError,
    operation: impl FnOnce() -> Result<(), String>,
) -> DestackStatus {
    clear_error(error);
    match operation() {
        Ok(()) => DestackStatus::Ok,
        Err(message) => {
            set_error(error, message);
            DestackStatus::Error
        }
    }
}
