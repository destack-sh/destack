use std::io::{self, Write};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::bindings::BindingDescriptor;
use crate::platform::console::bindings_generated as bindings;
use crate::platform::{PlatformError, PlatformStringRef, RuntimeStatus};
use crate::runtime::with_runtime_call_context;

/// Write a line to stdout for native code.
#[unsafe(export_name = "destack.console.log")]
pub unsafe extern "C" fn destack_console_log(value: PlatformStringRef) -> RuntimeStatus {
    write_console(value, ConsoleStream::Stdout, bindings::LOG)
}

/// Write an info line to stdout for native code.
#[unsafe(export_name = "destack.console.info")]
pub unsafe extern "C" fn destack_console_info(value: PlatformStringRef) -> RuntimeStatus {
    write_console(value, ConsoleStream::Stdout, bindings::INFO)
}

/// Write a warning line to stderr for native code.
#[unsafe(export_name = "destack.console.warn")]
pub unsafe extern "C" fn destack_console_warn(value: PlatformStringRef) -> RuntimeStatus {
    write_console(value, ConsoleStream::Stderr, bindings::WARN)
}

/// Write an error line to stderr for native code.
#[unsafe(export_name = "destack.console.error")]
pub unsafe extern "C" fn destack_console_error(value: PlatformStringRef) -> RuntimeStatus {
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
    value: PlatformStringRef,
    stream: ConsoleStream,
    spec: BindingDescriptor,
) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
            context.check_policy(spec)?;

            let line = unsafe { value.as_str()? };
            write_console_line(line, stream)
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
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
