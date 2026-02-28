use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceKind;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resource-table label for tty worker entries.
pub(crate) const TTY_RESOURCE_LABEL: &str = "tty.worker";
/// Resource-table label for pty controller entries.
pub(crate) const PTY_RESOURCE_LABEL: &str = "tty.controller";

/// Validate one output pointer argument.
pub(crate) fn ensure_out<T>(out: *mut T, field: &'static str) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer(field)).boxed());
    }

    Ok(())
}

/// Validate one flags argument that only supports zero.
pub(crate) fn validate_zero_flags(flags: u32, field: &'static str) -> RuntimeResult<()> {
    if flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            format!("unsupported flag bits: 0x{flags:x}"),
        ))
        .boxed());
    }

    Ok(())
}

/// Build one invalid tty-handle error.
pub(crate) fn invalid_tty_handle(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::InvalidArgumentValue),
        None,
        None,
        Some(operation.to_string()),
        None,
        "unknown tty handle".to_string(),
    ))
    .boxed()
}

/// Build one invalid pty-handle error.
pub(crate) fn invalid_pty_handle(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::InvalidArgumentValue),
        None,
        None,
        Some(operation.to_string()),
        None,
        "unknown pty handle".to_string(),
    ))
    .boxed()
}

/// Build one invalid file-handle error.
pub(crate) fn invalid_file_handle(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::InvalidArgumentValue),
        None,
        None,
        Some(operation.to_string()),
        None,
        "unknown file handle".to_string(),
    ))
    .boxed()
}

/// Build one not-terminal error for stdio tty operations.
pub(crate) fn not_terminal_error(operation: &'static str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        "standard stream is not attached to a terminal".to_string(),
    ))
    .boxed()
}

/// Remove one tty resource and run finalization.
pub(crate) fn close_tty_resource(
    context: &BindingCallContext,
    handle: resource::TtyHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_tty_handle(operation))?;
    if kind != ResourceKind::Tty {
        return Err(invalid_tty_handle(operation));
    }

    if !context.runtime().resources.remove_and_finalize(handle.0) {
        return Err(invalid_tty_handle(operation));
    }

    Ok(())
}

/// Remove one pty resource and run finalization.
pub(crate) fn close_pty_resource(
    context: &BindingCallContext,
    handle: resource::PtyHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = context
        .runtime()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_pty_handle(operation))?;
    if kind != ResourceKind::Pty {
        return Err(invalid_pty_handle(operation));
    }

    if !context.runtime().resources.remove_and_finalize(handle.0) {
        return Err(invalid_pty_handle(operation));
    }

    Ok(())
}
