use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::console::{ConsoleStream, write_console_line};
use crate::runtime::RuntimeCallContext;

/// Write a line to stdout.
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
