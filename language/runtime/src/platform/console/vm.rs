use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::console::{ConsoleStream, write_console_line};
use crate::runtime::RuntimeCallContext;

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
pub fn destack_console_log(
    _runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stdout)
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
pub fn destack_console_info(
    _runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stdout)
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
pub fn destack_console_warn(
    _runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stderr)
}

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
pub fn destack_console_error(
    _runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stderr)
}
