use crate::native_binding_set;
use crate::platform::bindings::{BindingDescriptor, NativeBinding};
use crate::platform::host::{HostStatus, HostStringRef, with_host_call_context};

use super::core::{ConsoleStream, emit_console_line};

native_binding_set!(
    pub CONSOLE_NATIVE_BINDINGS,
    "console",
    [
        NativeBinding::new(
            BindingDescriptor::external_recordable("destack.console.log"),
            "destack.console.log"
        ),
        NativeBinding::new(
            BindingDescriptor::external_recordable("destack.console.info"),
            "destack.console.info"
        ),
        NativeBinding::new(
            BindingDescriptor::external_recordable("destack.console.warn"),
            "destack.console.warn"
        ),
        NativeBinding::new(
            BindingDescriptor::external_recordable("destack.console.error"),
            "destack.console.error"
        ),
    ]
);

/// Emit a console log line from native code.
#[unsafe(export_name = "destack.console.log")]
pub unsafe extern "C" fn destack_console_log(value: HostStringRef) -> HostStatus {
    write_console_line(
        value,
        ConsoleStream::Stdout,
        BindingDescriptor::external_recordable("destack.console.log"),
    )
}

/// Emit a console info line from native code.
#[unsafe(export_name = "destack.console.info")]
pub unsafe extern "C" fn destack_console_info(value: HostStringRef) -> HostStatus {
    write_console_line(
        value,
        ConsoleStream::Stdout,
        BindingDescriptor::external_recordable("destack.console.info"),
    )
}

/// Emit a console warning line from native code.
#[unsafe(export_name = "destack.console.warn")]
pub unsafe extern "C" fn destack_console_warn(value: HostStringRef) -> HostStatus {
    write_console_line(
        value,
        ConsoleStream::Stderr,
        BindingDescriptor::external_recordable("destack.console.warn"),
    )
}

/// Emit a console error line from native code.
#[unsafe(export_name = "destack.console.error")]
pub unsafe extern "C" fn destack_console_error(value: HostStringRef) -> HostStatus {
    write_console_line(
        value,
        ConsoleStream::Stderr,
        BindingDescriptor::external_recordable("destack.console.error"),
    )
}

/// Write a console line through the host context.
fn write_console_line(
    value: HostStringRef,
    stream: ConsoleStream,
    spec: BindingDescriptor,
) -> HostStatus {
    // resolve policy and host context
    let result = with_host_call_context(|context| {
        context.check_policy(spec)?;

        // decode the line payload
        let line = unsafe { value.as_str()? };
        emit_console_line(line, stream)
    });

    HostStatus::from_result(result)
}
