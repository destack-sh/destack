use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

/// Build one invalid-argument runtime error.
pub(crate) fn invalid_argument(
    field: impl Into<String>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::invalid_argument_value(field, message)).boxed()
}

/// Build one not-supported runtime error.
pub(crate) fn not_supported(operation: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(operation)).boxed()
}

/// Build one I/O runtime error scoped to one binding operation.
pub(crate) fn io_operation_error(
    operation: &str,
    code: Option<PlatformErrorCode>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        code,
        None,
        None,
        Some(operation.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Build one io-not-found runtime error scoped to one binding operation.
pub(crate) fn io_not_found(operation: &str, message: impl Into<String>) -> Box<RuntimeError> {
    io_operation_error(operation, Some(PlatformErrorCode::IoNotFound), message)
}

/// Build one io-would-block runtime error scoped to one binding operation.
pub(crate) fn io_would_block(operation: &str, message: impl Into<String>) -> Box<RuntimeError> {
    io_operation_error(operation, Some(PlatformErrorCode::IoWouldBlock), message)
}

/// Build one io-busy runtime error scoped to one binding operation.
#[cfg(any(target_os = "linux", target_os = "macos", windows))]
pub(crate) fn io_busy(operation: &str, message: impl Into<String>) -> Box<RuntimeError> {
    io_operation_error(operation, Some(PlatformErrorCode::IoBusy), message)
}

/// Build one invalid-state runtime error.
#[allow(dead_code)]
pub(crate) fn invalid_state(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        message,
    ))
    .boxed()
}

/// Build one unsupported-flags runtime error for one flag field.
pub(crate) fn unsupported_flags(field: &str, flags: u32) -> Box<RuntimeError> {
    invalid_argument(field, format!("unsupported flag bits: 0x{flags:x}"))
}

/// Validate one output pointer argument.
pub(crate) fn ensure_out<T>(out: *mut T, field: &'static str) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(field)).boxed());
    }

    Ok(())
}

/// Validate one flags payload that only supports zero.
pub(crate) fn ensure_zero_flags(flags: u32, field: &'static str) -> RuntimeResult<()> {
    if flags != 0 {
        return Err(unsupported_flags(field, flags));
    }

    Ok(())
}
