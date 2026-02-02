use std::io::{self, Write};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::{BindingDescriptor, native_call};
use crate::platform::console::bindings_generated as bindings;
use crate::platform::{NativeStringRef, PlatformError, RuntimeStatus};

/// Write a line to stdout for native code.
#[unsafe(export_name = "destack.console.log")]
pub unsafe extern "C" fn destack_console_log(value: NativeStringRef) -> RuntimeStatus {
    write_console(value, ConsoleStream::Stdout, bindings::LOG)
}

/// Write an info line to stdout for native code.
#[unsafe(export_name = "destack.console.info")]
pub unsafe extern "C" fn destack_console_info(value: NativeStringRef) -> RuntimeStatus {
    write_console(value, ConsoleStream::Stdout, bindings::INFO)
}

/// Write a warning line to stderr for native code.
#[unsafe(export_name = "destack.console.warn")]
pub unsafe extern "C" fn destack_console_warn(value: NativeStringRef) -> RuntimeStatus {
    write_console(value, ConsoleStream::Stderr, bindings::WARN)
}

/// Write an error line to stderr for native code.
#[unsafe(export_name = "destack.console.error")]
pub unsafe extern "C" fn destack_console_error(value: NativeStringRef) -> RuntimeStatus {
    write_console(value, ConsoleStream::Stderr, bindings::ERROR)
}

/// Output selector for console writes.
enum ConsoleStream {
    /// Write to stdout.
    Stdout,
    /// Write to stderr.
    Stderr,
}

/// Write a console line to the platform stream.
fn write_console(
    value: NativeStringRef,
    stream: ConsoleStream,
    spec: BindingDescriptor,
) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(spec)?;

        let line = unsafe { value.as_str()? };
        write_console_line(line, stream)
    })
}

/// Write a console line to the platform stream.
fn write_console_line(line: &str, stream: ConsoleStream) -> RuntimeResult<()> {
    // emit the formatted line
    write_console_to_stream(stream, line)?;

    Ok(())
}

/// Write a line to stdout or stderr.
fn write_console_to_stream(stream: ConsoleStream, line: &str) -> RuntimeResult<()> {
    // write to the selected stream
    match stream {
        ConsoleStream::Stdout => {
            let mut stdout = io::stdout();
            writeln!(stdout, "{line}").map_err(|error| {
                RuntimeError::platform(PlatformError::io(format!("stdout write failed: {error}")))
                    .boxed()
            })?;
        }
        ConsoleStream::Stderr => {
            let mut stderr = io::stderr();
            writeln!(stderr, "{line}").map_err(|error| {
                RuntimeError::platform(PlatformError::io(format!("stderr write failed: {error}")))
                    .boxed()
            })?;
        }
    }

    Ok(())
}
