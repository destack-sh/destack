use std::ptr;

use crate::diagnostic::{RuntimeError, RuntimeErrorId};
use crate::platform::RuntimeStatus;
use crate::platform::abi::PlatformStringRef;
use crate::platform::diagnostic::{PlatformError as DiagnosticPlatformError, PlatformErrorKind};
use crate::platform::error::{PlatformError, PlatformErrorCode, bindings_generated as bindings};
use crate::runtime::with_runtime_call_context;

/// Take a runtime platform error by id.
#[unsafe(export_name = "destack.error.takePlatformError")]
pub unsafe extern "C" fn destack_error_take_platform_error(
    out: *mut PlatformError,
    error_id: u64,
) -> RuntimeStatus {
    let status = with_runtime_call_context(|context| {
        let result = (|| {
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
            let platform_error = abi_platform_error_from_runtime(&platform_error);

            unsafe {
                out.write(platform_error);
            }

            Ok(())
        })();

        Ok(RuntimeStatus::from_result(result, Some(context)))
    });

    match status {
        Ok(status) => status,
        Err(error) => RuntimeStatus::from_error(error, None),
    }
}

fn abi_platform_error_from_runtime(error: &DiagnosticPlatformError) -> PlatformError {
    // NOTE #Incomplete: platform error strings are leaked for now
    PlatformError {
        kind: string_ref_from_str(kind_str(error.kind)),
        message: string_ref_from_str(&error.message),
        name: string_ref_from_option(error.name.as_ref()),
        code: map_platform_error_code(
            error
                .code
                .unwrap_or(crate::platform::diagnostic::PlatformErrorCode::Generic),
        ),
        system_code: string_ref_from_option(error.system_code.as_ref()),
        errno: error.errno.unwrap_or(0),
        syscall: string_ref_from_option(error.syscall.as_ref()),
        path: string_ref_from_option(error.path.as_ref()),
        dest: string_ref_from_option(error.dest.as_ref()),
        fd: error.fd.unwrap_or(0),
        address: string_ref_from_option(error.address.as_ref()),
        port: error.port.unwrap_or(0),
        hostname: string_ref_from_option(error.hostname.as_ref()),
        signal: string_ref_from_option(error.signal.as_ref()),
        exit_code: error.exit_code.unwrap_or(0),
        cause: string_ref_from_option(error.cause.as_ref()),
        argument: string_ref_from_option(error.argument.as_ref()),
        pointer: string_ref_from_option(error.pointer.as_ref()),
        feature: string_ref_from_option(error.feature.as_ref()),
    }
}

fn string_ref_from_option(value: Option<&String>) -> PlatformStringRef {
    match value {
        Some(value) => string_ref_from_str(value),
        None => PlatformStringRef {
            data: ptr::null(),
            len: 0,
        },
    }
}

fn string_ref_from_str(value: &str) -> PlatformStringRef {
    let leaked: &'static str = Box::leak(value.to_string().into_boxed_str());
    PlatformStringRef::from(leaked)
}

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

fn map_platform_error_code(
    code: crate::platform::diagnostic::PlatformErrorCode,
) -> PlatformErrorCode {
    match code {
        crate::platform::diagnostic::PlatformErrorCode::InvalidArgument => {
            PlatformErrorCode::InvalidArgument
        }
        crate::platform::diagnostic::PlatformErrorCode::InvalidArgumentType => {
            PlatformErrorCode::InvalidArgumentType
        }
        crate::platform::diagnostic::PlatformErrorCode::InvalidArgumentValue => {
            PlatformErrorCode::InvalidArgumentValue
        }
        crate::platform::diagnostic::PlatformErrorCode::NullPointer => {
            PlatformErrorCode::NullPointer
        }
        crate::platform::diagnostic::PlatformErrorCode::NotSupported => {
            PlatformErrorCode::NotSupported
        }
        crate::platform::diagnostic::PlatformErrorCode::Io => PlatformErrorCode::Io,
        crate::platform::diagnostic::PlatformErrorCode::IoReadFailed => {
            PlatformErrorCode::IoReadFailed
        }
        crate::platform::diagnostic::PlatformErrorCode::IoWriteFailed => {
            PlatformErrorCode::IoWriteFailed
        }
        crate::platform::diagnostic::PlatformErrorCode::IoNotFound => PlatformErrorCode::IoNotFound,
        crate::platform::diagnostic::PlatformErrorCode::IoPermissionDenied => {
            PlatformErrorCode::IoPermissionDenied
        }
        crate::platform::diagnostic::PlatformErrorCode::IoAlreadyExists => {
            PlatformErrorCode::IoAlreadyExists
        }
        crate::platform::diagnostic::PlatformErrorCode::IoNotDirectory => {
            PlatformErrorCode::IoNotDirectory
        }
        crate::platform::diagnostic::PlatformErrorCode::IoIsDirectory => {
            PlatformErrorCode::IoIsDirectory
        }
        crate::platform::diagnostic::PlatformErrorCode::IoNotEmpty => PlatformErrorCode::IoNotEmpty,
        crate::platform::diagnostic::PlatformErrorCode::IoReadOnly => PlatformErrorCode::IoReadOnly,
        crate::platform::diagnostic::PlatformErrorCode::IoNameTooLong => {
            PlatformErrorCode::IoNameTooLong
        }
        crate::platform::diagnostic::PlatformErrorCode::IoFileTooLarge => {
            PlatformErrorCode::IoFileTooLarge
        }
        crate::platform::diagnostic::PlatformErrorCode::IoTooManyOpenFiles => {
            PlatformErrorCode::IoTooManyOpenFiles
        }
        crate::platform::diagnostic::PlatformErrorCode::IoFileTableOverflow => {
            PlatformErrorCode::IoFileTableOverflow
        }
        crate::platform::diagnostic::PlatformErrorCode::IoInvalidData => {
            PlatformErrorCode::IoInvalidData
        }
        crate::platform::diagnostic::PlatformErrorCode::IoCrossDevice => {
            PlatformErrorCode::IoCrossDevice
        }
        crate::platform::diagnostic::PlatformErrorCode::IoBrokenPipe => {
            PlatformErrorCode::IoBrokenPipe
        }
        crate::platform::diagnostic::PlatformErrorCode::IoTimedOut => PlatformErrorCode::IoTimedOut,
        crate::platform::diagnostic::PlatformErrorCode::IoInterrupted => {
            PlatformErrorCode::IoInterrupted
        }
        crate::platform::diagnostic::PlatformErrorCode::IoBusy => PlatformErrorCode::IoBusy,
        crate::platform::diagnostic::PlatformErrorCode::IoWouldBlock => {
            PlatformErrorCode::IoWouldBlock
        }
        crate::platform::diagnostic::PlatformErrorCode::Net => PlatformErrorCode::Net,
        crate::platform::diagnostic::PlatformErrorCode::NetConnectionRefused => {
            PlatformErrorCode::NetConnectionRefused
        }
        crate::platform::diagnostic::PlatformErrorCode::NetTimedOut => {
            PlatformErrorCode::NetTimedOut
        }
        crate::platform::diagnostic::PlatformErrorCode::NetConnectionReset => {
            PlatformErrorCode::NetConnectionReset
        }
        crate::platform::diagnostic::PlatformErrorCode::NetAddressInUse => {
            PlatformErrorCode::NetAddressInUse
        }
        crate::platform::diagnostic::PlatformErrorCode::NetAddressNotAvailable => {
            PlatformErrorCode::NetAddressNotAvailable
        }
        crate::platform::diagnostic::PlatformErrorCode::NetNetworkUnreachable => {
            PlatformErrorCode::NetNetworkUnreachable
        }
        crate::platform::diagnostic::PlatformErrorCode::NetHostUnreachable => {
            PlatformErrorCode::NetHostUnreachable
        }
        crate::platform::diagnostic::PlatformErrorCode::NetConnectionAborted => {
            PlatformErrorCode::NetConnectionAborted
        }
        crate::platform::diagnostic::PlatformErrorCode::NetBrokenPipe => {
            PlatformErrorCode::NetBrokenPipe
        }
        crate::platform::diagnostic::PlatformErrorCode::NetDnsFailed => {
            PlatformErrorCode::NetDnsFailed
        }
        crate::platform::diagnostic::PlatformErrorCode::Process => PlatformErrorCode::Process,
        crate::platform::diagnostic::PlatformErrorCode::ProcessSpawnFailed => {
            PlatformErrorCode::ProcessSpawnFailed
        }
        crate::platform::diagnostic::PlatformErrorCode::ProcessNotFound => {
            PlatformErrorCode::ProcessNotFound
        }
        crate::platform::diagnostic::PlatformErrorCode::ProcessPermissionDenied => {
            PlatformErrorCode::ProcessPermissionDenied
        }
        crate::platform::diagnostic::PlatformErrorCode::Random => PlatformErrorCode::Random,
        crate::platform::diagnostic::PlatformErrorCode::RandomUnavailable => {
            PlatformErrorCode::RandomUnavailable
        }
        crate::platform::diagnostic::PlatformErrorCode::Time => PlatformErrorCode::Time,
        crate::platform::diagnostic::PlatformErrorCode::TimeUnavailable => {
            PlatformErrorCode::TimeUnavailable
        }
        crate::platform::diagnostic::PlatformErrorCode::Generic => PlatformErrorCode::Generic,
    }
}
