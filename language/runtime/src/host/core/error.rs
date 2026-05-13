use crate::diagnostic::{HostErrorCode, RuntimeError};
use crate::host::HostError;

/// Build one I/O runtime error scoped to one binding operation.
fn io_operation_error(
    operation: &str,
    code: Option<HostErrorCode>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(HostError::io_with(
        code,
        None,
        None,
        Some(operation.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Build one io-would-block runtime error scoped to one binding operation.
pub(crate) fn io_would_block(operation: &str, message: impl Into<String>) -> Box<RuntimeError> {
    io_operation_error(operation, Some(HostErrorCode::IoWouldBlock), message)
}

/// Build one invalid-state runtime error.
#[allow(dead_code)]
pub(crate) fn invalid_state(message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(HostError::generic(Some(HostErrorCode::Generic), message)).boxed()
}
