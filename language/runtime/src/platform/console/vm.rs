use destack_vm as vm;
use std::io::{self, Write};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::RuntimeCallContext;

/// Write a line to stdout.
pub fn destack_console_log(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stdout)
}

/// Write an info line to stdout.
pub fn destack_console_info(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stdout)
}

/// Write a warning line to stderr.
pub fn destack_console_warn(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stderr)
}

/// Write an error line to stderr.
pub fn destack_console_error(
    _runtime: &RuntimeCallContext,
    context: &mut vm::RuntimeContext<'_>,
    value: vm::StringHandle,
) -> RuntimeResult<()> {
    let value = context
        .string_ref(value)
        .map_err(Box::<RuntimeError>::from)?;
    write_console_line(value.as_str(), ConsoleStream::Stderr)
}

/// Output selector for console writes.
enum ConsoleStream {
    /// Write to stdout.
    Stdout,
    /// Write to stderr.
    Stderr,
}

/// Write a console line to the platform stream.
#[inline]
fn write_console_line(value: &str, stream: ConsoleStream) -> RuntimeResult<()> {
    // emit the formatted line
    write_console_to_stream(stream, value)?;

    Ok(())
}

/// Write a line to stdout or stderr.
#[inline]
fn write_console_to_stream(stream: ConsoleStream, line: &str) -> RuntimeResult<()> {
    // write to the selected stream
    match stream {
        ConsoleStream::Stdout => {
            let mut stdout = io::stdout();
            writeln!(stdout, "{line}").map_err(|error| {
                RuntimeError::vm(vm::Error::Panic {
                    message: format!("stdout write failed: {error}"),
                })
                .boxed()
            })?;
        }
        ConsoleStream::Stderr => {
            let mut stderr = io::stderr();
            writeln!(stderr, "{line}").map_err(|error| {
                RuntimeError::vm(vm::Error::Panic {
                    message: format!("stderr write failed: {error}"),
                })
                .boxed()
            })?;
        }
    }

    Ok(())
}
