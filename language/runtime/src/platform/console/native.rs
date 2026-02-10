use crate::diagnostic::RuntimeResult;
use crate::platform::NativeStringRef;
use crate::platform::console::{ConsoleStream, write_console_line};
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
