#![cfg_attr(not(any(unix, windows)), allow(unused_imports))]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::ResourceKind;
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Resource-table label for tty worker entries.
pub(crate) const TTY_RESOURCE_LABEL: &str = "tty.worker";
/// Resource-table label for pty controller entries.
pub(crate) const PTY_RESOURCE_LABEL: &str = "tty.controller";

pub(crate) use core_platform::{ensure_out, ensure_zero_flags as validate_zero_flags};

/// Build one invalid tty-handle error.
pub(crate) fn invalid_tty_handle(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown tty handle",
    )
}

/// Build one invalid pty-handle error.
pub(crate) fn invalid_pty_handle(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown pty handle",
    )
}

/// Build one invalid file-handle error.
pub(crate) fn invalid_file_handle(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::InvalidArgumentValue),
        "unknown file handle",
    )
}

/// Build one not-terminal error for stdio tty operations.
pub(crate) fn not_terminal_error(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_operation_error(
        operation,
        Some(PlatformErrorCode::IoInvalidData),
        "standard stream is not attached to a terminal",
    )
}

/// Remove one tty resource and run finalization.
pub(crate) fn close_tty_resource(
    binding: &BindingCallContext,
    handle: resource::TtyHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_tty_handle(operation))?;
    if kind != ResourceKind::Tty {
        return Err(invalid_tty_handle(operation));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_tty_handle(operation));
    }

    Ok(())
}

/// Remove one pty resource and run finalization.
pub(crate) fn close_pty_resource(
    binding: &BindingCallContext,
    handle: resource::PtyHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    let kind = binding
        .worker()
        .resources
        .with_entry(handle.0, |entry| entry.kind)
        .ok_or_else(|| invalid_pty_handle(operation))?;
    if kind != ResourceKind::Pty {
        return Err(invalid_pty_handle(operation));
    }

    if !binding.worker().resources.remove_and_finalize(
        binding.world(),
        handle.0,
        Some(binding.engine()),
    ) {
        return Err(invalid_pty_handle(operation));
    }

    Ok(())
}
