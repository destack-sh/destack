use std::io::{self, Write};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{NativeStringRef, PlatformError};
use crate::runtime::RuntimeCallContext;

/// Write a line to stdout for native code.
pub unsafe fn destack_console_log(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stdout)
}

/// Write an info line to stdout for native code.
pub unsafe fn destack_console_info(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stdout)
}

/// Write a warning line to stderr for native code.
pub unsafe fn destack_console_warn(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stderr)
}

/// Write an error line to stderr for native code.
pub unsafe fn destack_console_error(
    _context: &RuntimeCallContext,
    value: NativeStringRef,
) -> RuntimeResult<()> {
    let line = unsafe { value.as_str()? };
    write_console_line(line, ConsoleStream::Stderr)
}

/// Output selector for console writes.
enum ConsoleStream {
    /// Write to stdout.
    Stdout,
    /// Write to stderr.
    Stderr,
}

/// Write a console line to the platform stream.
fn write_console_line(line: &str, stream: ConsoleStream) -> RuntimeResult<()> {
    match stream {
        ConsoleStream::Stdout => {
            let mut stdout = io::stdout();
            writeln!(stdout, "{line}").map_err(|error| {
                RuntimeError::from(PlatformError::io(format!("stdout write failed: {error}")))
                    .boxed()
            })?;
        }
        ConsoleStream::Stderr => {
            let mut stderr = io::stderr();
            writeln!(stderr, "{line}").map_err(|error| {
                RuntimeError::from(PlatformError::io(format!("stderr write failed: {error}")))
                    .boxed()
            })?;
        }
    }
    Ok(())
}
