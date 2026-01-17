use destack_vm::{ExternalContext, Value, ValueTag};

use crate::platform::bindings::BindingDescriptor;
use crate::platform::host::{HostContext, HostResult, IoStream};
use crate::{binding, binding_set};

binding_set!(
    pub CONSOLE_VM_BINDINGS,
    "console",
    |registry, isolate, host| {
        // register console handlers
        let host_stdout = host.clone();
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.log"),
            move |context: &mut ExternalContext<'_>, args: &[Value]| -> HostResult<Value> {
                write_console_line(&host_stdout, context, args, IoStream::Stdout)
            }
        );

        let host_info = host.clone();
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.info"),
            move |context: &mut ExternalContext<'_>, args: &[Value]| -> HostResult<Value> {
                write_console_line(&host_info, context, args, IoStream::Stdout)
            }
        );

        let host_warn = host.clone();
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.warn"),
            move |context: &mut ExternalContext<'_>, args: &[Value]| -> HostResult<Value> {
                write_console_line(&host_warn, context, args, IoStream::Stderr)
            }
        );

        let host_error = host.clone();
        binding!(
            registry,
            isolate,
            BindingDescriptor::external_recordable("destack.console.error"),
            move |context: &mut ExternalContext<'_>, args: &[Value]| -> HostResult<Value> {
                write_console_line(&host_error, context, args, IoStream::Stderr)
            }
        );
    }
);

/// Write a console line to the host stream.
fn write_console_line(
    host: &HostContext,
    context: &mut ExternalContext<'_>,
    args: &[Value],
    stream: IoStream,
) -> HostResult<Value> {
    // format the line payload
    let line = format_values(context, args);

    // emit the formatted line
    host.io().write_line(stream, &line)?;

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
