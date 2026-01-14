use destack_vm::{ExternalContext, Value, ValueTag};

use crate::binding;
use crate::binding_set;
use crate::platform::bindings::BindingDescriptor;
use crate::platform::host::HostResult;

use super::core::{emit_console_line, ConsoleStream};

binding_set!(
    pub CONSOLE_VM_BINDINGS,
    "console",
    |registry, isolate, _host| {
        // register console handlers
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.log"),
            fn console_log(context: &mut ExternalContext<'_>, args: &[Value]) -> HostResult<Value> {
                write_console_line(context, args, ConsoleStream::Stdout)
            }
        );
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.info"),
            fn console_info(
                context: &mut ExternalContext<'_>,
                args: &[Value],
            ) -> HostResult<Value> {
                write_console_line(context, args, ConsoleStream::Stdout)
            }
        );
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.warn"),
            fn console_warn(
                context: &mut ExternalContext<'_>,
                args: &[Value],
            ) -> HostResult<Value> {
                write_console_line(context, args, ConsoleStream::Stderr)
            }
        );
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.error"),
            fn console_error(
                context: &mut ExternalContext<'_>,
                args: &[Value],
            ) -> HostResult<Value> {
                write_console_line(context, args, ConsoleStream::Stderr)
            }
        );
    }
);

/// Write a console line to the host stream.
fn write_console_line(
    context: &mut ExternalContext<'_>,
    args: &[Value],
    stream: ConsoleStream,
) -> HostResult<Value> {
    // format the line payload
    let line = format_values(context, args);

    // emit the formatted line
    emit_console_line(&line, stream)?;

    Ok(Value::VOID)
}

/// Format multiple values for console output.
fn format_values(context: &mut ExternalContext<'_>, args: &[Value]) -> String {
    // short circuit empty arguments
    if args.is_empty() {
        return String::new();
    }

    // build the formatted output
    let mut out = String::new();
    for (index, value) in args.iter().copied().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(&format_value(context, value));
    }

    out
}

/// Format a single value for console output.
fn format_value(context: &mut ExternalContext<'_>, value: Value) -> String {
    // format by value tag
    match value.tag() {
        ValueTag::String => context
            .string_value(value)
            .unwrap_or_else(|_| format!("{value:?}")),
        ValueTag::Bool => value
            .as_bool()
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{value:?}")),
        ValueTag::Int => value
            .as_int()
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{value:?}")),
        ValueTag::UInt => value
            .as_uint()
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{value:?}")),
        ValueTag::Float32 => value
            .as_float32()
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{value:?}")),
        ValueTag::Float64 => value
            .as_float64()
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{value:?}")),
        ValueTag::Char => value
            .as_char()
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{value:?}")),
        _ => format!("{value:?}"),
    }
}
