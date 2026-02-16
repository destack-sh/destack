#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Write an error line to stderr.
///
/// Writes the provided string payload directly to the selected console stream.
/// Does not apply formatting, buffering, or retry behavior beyond the underlying platform write call.
///
/// # Platform
/// all runtime targets that include the native platform runtime.
/// Uses write(2) to stderr on Unix, WriteFile on Windows stderr handle.
///
/// # Errors
/// Returns ioWriteFailed, ioBrokenPipe, notSupported.
///
/// # Security
/// Requires `console.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_console_error(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_value: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = argument_value;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.console.console.error",
    ))
    .boxed())
}

/// Write an info line to stdout.
///
/// Writes the provided string payload directly to the selected console stream.
/// Does not apply formatting, buffering, or retry behavior beyond the underlying platform write call.
///
/// # Platform
/// all runtime targets that include the native platform runtime.
/// Uses write(2) to stdout on Unix, WriteFile on Windows stdout handle.
///
/// # Errors
/// Returns ioWriteFailed, ioBrokenPipe, notSupported.
///
/// # Security
/// Requires `console.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_console_info(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_value: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = argument_value;
    Err(RuntimeError::from(PlatformError::not_supported("destack.console.console.info")).boxed())
}

/// Write a line to stdout.
///
/// Writes the provided string payload directly to the selected console stream.
/// Does not apply formatting, buffering, or retry behavior beyond the underlying platform write call.
///
/// # Platform
/// all runtime targets that include the native platform runtime.
/// Uses write(2) to stdout on Unix, WriteFile on Windows stdout handle.
///
/// # Errors
/// Returns ioWriteFailed, ioBrokenPipe, notSupported.
///
/// # Security
/// Requires `console.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_console_log(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_value: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = argument_value;
    Err(RuntimeError::from(PlatformError::not_supported("destack.console.console.log")).boxed())
}

/// Write a warning line to stderr.
///
/// Writes the provided string payload directly to the selected console stream.
/// Does not apply formatting, buffering, or retry behavior beyond the underlying platform write call.
///
/// # Platform
/// all runtime targets that include the native platform runtime.
/// Uses write(2) to stderr on Unix, WriteFile on Windows stderr handle.
///
/// # Errors
/// Returns ioWriteFailed, ioBrokenPipe, notSupported.
///
/// # Security
/// Requires `console.write`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_console_warn(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    argument_value: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = argument_value;
    Err(RuntimeError::from(PlatformError::not_supported("destack.console.console.warn")).boxed())
}
