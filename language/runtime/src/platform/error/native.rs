use crate::diagnostic::{RuntimeError, RuntimeErrorId};
use crate::platform::RuntimeStatus;
use crate::platform::bindings::native_call;
use crate::platform::diagnostic::{
    PlatformError as DiagnosticPlatformError, PlatformErrorCode as DiagnosticPlatformErrorCode,
    PlatformErrorKind,
};
use crate::platform::error::{PlatformError, PlatformErrorCode, bindings_generated as bindings};
use crate::runtime::RuntimeCallContext;

/// Take a runtime platform error by id.
#[unsafe(export_name = "destack.error.takePlatformError")]
pub unsafe extern "C" fn destack_error_take_platform_error(
    out: *mut PlatformError,
    error_id: u64,
) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(bindings::TAKE_PLATFORM_ERROR)?;

        if out.is_null() {
            return Err(
                RuntimeError::platform(DiagnosticPlatformError::null_pointer("out")).boxed(),
            );
        }

        let error = context
            .runtime()
            .errors
            .take(RuntimeErrorId::from_raw(error_id))
            .unwrap_or_else(|| RuntimeError::internal("missing runtime error for id").boxed());
        let platform_error = error.into_platform_error();
        let platform_error = abi_platform_error_from_runtime(context, &platform_error);

        unsafe {
            out.write(platform_error);
        }

        Ok(())
    })
}

/// Convert a runtime error into the ABI platform error shape.
fn abi_platform_error_from_runtime(
    context: &RuntimeCallContext,
    error: &DiagnosticPlatformError,
) -> PlatformError {
    PlatformError {
        kind: context.store_string(kind_str(error.kind)),
        message: context.store_string(&error.message),
        name: context.store_string_option(error.name.as_ref()),
        code: map_platform_error_code(error.code.unwrap_or(DiagnosticPlatformErrorCode::Generic)),
        system_code: context.store_string_option(error.system_code.as_ref()),
        errno: error.errno.unwrap_or(0),
        syscall: context.store_string_option(error.syscall.as_ref()),
        path: context.store_string_option(error.path.as_ref()),
        dest: context.store_string_option(error.dest.as_ref()),
        fd: error.fd.unwrap_or(0),
        address: context.store_string_option(error.address.as_ref()),
        port: error.port.unwrap_or(0),
        hostname: context.store_string_option(error.hostname.as_ref()),
        signal: context.store_string_option(error.signal.as_ref()),
        exit_code: error.exit_code.unwrap_or(0),
        cause: context.store_string_option(error.cause.as_ref()),
        argument: context.store_string_option(error.argument.as_ref()),
        pointer: context.store_string_option(error.pointer.as_ref()),
        feature: context.store_string_option(error.feature.as_ref()),
    }
}

/// Return the string representation for a platform error kind.
fn kind_str(kind: PlatformErrorKind) -> &'static str {
    match kind {
        PlatformErrorKind::InvalidArgument => "invalidArgument",
        PlatformErrorKind::NullPointer => "nullPointer",
        PlatformErrorKind::NotSupported => "notSupported",
        PlatformErrorKind::Io => "io",
        PlatformErrorKind::Net => "net",
        PlatformErrorKind::Process => "process",
        PlatformErrorKind::Random => "random",
        PlatformErrorKind::Time => "time",
        PlatformErrorKind::Generic => "generic",
    }
}

/// Map diagnostic error codes into ABI platform error codes.
fn map_platform_error_code(code: DiagnosticPlatformErrorCode) -> PlatformErrorCode {
    match code {
        DiagnosticPlatformErrorCode::InvalidArgument => PlatformErrorCode::InvalidArgument,
        DiagnosticPlatformErrorCode::InvalidArgumentType => PlatformErrorCode::InvalidArgumentType,
        DiagnosticPlatformErrorCode::InvalidArgumentValue => {
            PlatformErrorCode::InvalidArgumentValue
        }
        DiagnosticPlatformErrorCode::NullPointer => PlatformErrorCode::NullPointer,
        DiagnosticPlatformErrorCode::NotSupported => PlatformErrorCode::NotSupported,
        DiagnosticPlatformErrorCode::Io => PlatformErrorCode::Io,
        DiagnosticPlatformErrorCode::IoReadFailed => PlatformErrorCode::IoReadFailed,
        DiagnosticPlatformErrorCode::IoWriteFailed => PlatformErrorCode::IoWriteFailed,
        DiagnosticPlatformErrorCode::IoNotFound => PlatformErrorCode::IoNotFound,
        DiagnosticPlatformErrorCode::IoPermissionDenied => PlatformErrorCode::IoPermissionDenied,
        DiagnosticPlatformErrorCode::IoAlreadyExists => PlatformErrorCode::IoAlreadyExists,
        DiagnosticPlatformErrorCode::IoNotDirectory => PlatformErrorCode::IoNotDirectory,
        DiagnosticPlatformErrorCode::IoIsDirectory => PlatformErrorCode::IoIsDirectory,
        DiagnosticPlatformErrorCode::IoNotEmpty => PlatformErrorCode::IoNotEmpty,
        DiagnosticPlatformErrorCode::IoReadOnly => PlatformErrorCode::IoReadOnly,
        DiagnosticPlatformErrorCode::IoNameTooLong => PlatformErrorCode::IoNameTooLong,
        DiagnosticPlatformErrorCode::IoFileTooLarge => PlatformErrorCode::IoFileTooLarge,
        DiagnosticPlatformErrorCode::IoTooManyOpenFiles => PlatformErrorCode::IoTooManyOpenFiles,
        DiagnosticPlatformErrorCode::IoFileTableOverflow => PlatformErrorCode::IoFileTableOverflow,
        DiagnosticPlatformErrorCode::IoInvalidData => PlatformErrorCode::IoInvalidData,
        DiagnosticPlatformErrorCode::IoCrossDevice => PlatformErrorCode::IoCrossDevice,
        DiagnosticPlatformErrorCode::IoBrokenPipe => PlatformErrorCode::IoBrokenPipe,
        DiagnosticPlatformErrorCode::IoTimedOut => PlatformErrorCode::IoTimedOut,
        DiagnosticPlatformErrorCode::IoInterrupted => PlatformErrorCode::IoInterrupted,
        DiagnosticPlatformErrorCode::IoBusy => PlatformErrorCode::IoBusy,
        DiagnosticPlatformErrorCode::IoWouldBlock => PlatformErrorCode::IoWouldBlock,
        DiagnosticPlatformErrorCode::Net => PlatformErrorCode::Net,
        DiagnosticPlatformErrorCode::NetConnectionRefused => {
            PlatformErrorCode::NetConnectionRefused
        }
        DiagnosticPlatformErrorCode::NetTimedOut => PlatformErrorCode::NetTimedOut,
        DiagnosticPlatformErrorCode::NetConnectionReset => PlatformErrorCode::NetConnectionReset,
        DiagnosticPlatformErrorCode::NetAddressInUse => PlatformErrorCode::NetAddressInUse,
        DiagnosticPlatformErrorCode::NetAddressNotAvailable => {
            PlatformErrorCode::NetAddressNotAvailable
        }
        DiagnosticPlatformErrorCode::NetNetworkUnreachable => {
            PlatformErrorCode::NetNetworkUnreachable
        }
        DiagnosticPlatformErrorCode::NetHostUnreachable => PlatformErrorCode::NetHostUnreachable,
        DiagnosticPlatformErrorCode::NetConnectionAborted => {
            PlatformErrorCode::NetConnectionAborted
        }
        DiagnosticPlatformErrorCode::NetBrokenPipe => PlatformErrorCode::NetBrokenPipe,
        DiagnosticPlatformErrorCode::NetDnsFailed => PlatformErrorCode::NetDnsFailed,
        DiagnosticPlatformErrorCode::Process => PlatformErrorCode::Process,
        DiagnosticPlatformErrorCode::ProcessSpawnFailed => PlatformErrorCode::ProcessSpawnFailed,
        DiagnosticPlatformErrorCode::ProcessNotFound => PlatformErrorCode::ProcessNotFound,
        DiagnosticPlatformErrorCode::ProcessPermissionDenied => {
            PlatformErrorCode::ProcessPermissionDenied
        }
        DiagnosticPlatformErrorCode::Random => PlatformErrorCode::Random,
        DiagnosticPlatformErrorCode::RandomUnavailable => PlatformErrorCode::RandomUnavailable,
        DiagnosticPlatformErrorCode::Time => PlatformErrorCode::Time,
        DiagnosticPlatformErrorCode::TimeUnavailable => PlatformErrorCode::TimeUnavailable,
        DiagnosticPlatformErrorCode::Generic => PlatformErrorCode::Generic,
    }
}
