use crate::diagnostic::RuntimeResult;
use crate::platform::NativeStringRef;
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
pub(crate) unsafe fn destack_console_log(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stdout)
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
pub(crate) unsafe fn destack_console_info(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stdout)
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
pub(crate) unsafe fn destack_console_warn(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stderr)
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
pub(crate) unsafe fn destack_console_error(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stderr)
}
