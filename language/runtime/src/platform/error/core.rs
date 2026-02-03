use destack_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeErrorId, RuntimeErrorStore};
use crate::platform::diagnostic::{
    PlatformError as DiagnosticPlatformError, PlatformErrorCode as DiagnosticPlatformErrorCode,
    PlatformErrorKind,
};
use crate::platform::error::PlatformErrorCode;
use crate::runtime::RuntimeCallContext;

/// Converted platform error fields for ABI construction.
#[derive(Debug)]
pub struct PlatformErrorFields<S> {
    /// The kind field.
    pub kind: S,
    /// The message field.
    pub message: S,
    /// The name field.
    pub name: S,
    /// The code field.
    pub code: PlatformErrorCode,
    /// The system_code field.
    pub system_code: S,
    /// The errno field.
    pub errno: i32,
    /// The syscall field.
    pub syscall: S,
    /// The path field.
    pub path: S,
    /// The dest field.
    pub dest: S,
    /// The fd field.
    pub fd: i32,
    /// The address field.
    pub address: S,
    /// The port field.
    pub port: u16,
    /// The hostname field.
    pub hostname: S,
    /// The signal field.
    pub signal: S,
    /// The exit_code field.
    pub exit_code: i32,
    /// The cause field.
    pub cause: S,
    /// The argument field.
    pub argument: S,
    /// The pointer field.
    pub pointer: S,
    /// The feature field.
    pub feature: S,
}

/// Store strings for platform error conversion.
pub trait PlatformErrorStringStore {
    /// String representation for stored fields.
    type String;

    /// Store a required string.
    fn store_string(&mut self, value: &str) -> Self::String;
    /// Store an optional string.
    fn store_string_option(&mut self, value: Option<&String>) -> Self::String;
}

/// String storage adapter for native runtime calls.
#[derive(Debug)]
pub struct NativeStringStore<'a> {
    /// Runtime call context for the current binding call.
    context: &'a RuntimeCallContext,
}

impl<'a> NativeStringStore<'a> {
    /// Create a native string store for the runtime context.
    pub fn new(context: &'a RuntimeCallContext) -> Self {
        Self { context }
    }
}

impl PlatformErrorStringStore for NativeStringStore<'_> {
    type String = crate::platform::NativeStringRef;

    fn store_string(&mut self, value: &str) -> Self::String {
        self.context.store_string(value)
    }

    fn store_string_option(&mut self, value: Option<&String>) -> Self::String {
        self.context.store_string_option(value)
    }
}

/// String storage adapter for VM runtime calls.
#[derive(Debug)]
pub struct VmStringStore<'a, 'ctx> {
    /// Runtime context for the current VM call.
    context: &'a mut vm::RuntimeContext<'ctx>,
}

impl<'a, 'ctx> VmStringStore<'a, 'ctx> {
    /// Create a VM string store for the runtime context.
    pub fn new(context: &'a mut vm::RuntimeContext<'ctx>) -> Self {
        Self { context }
    }

    /// Return the VM sentinel for an absent string.
    fn none_sentinel() -> vm::StringHandle {
        vm::StringHandle::new(vm::Value::VOID)
    }
}

impl PlatformErrorStringStore for VmStringStore<'_, '_> {
    type String = vm::StringHandle;

    fn store_string(&mut self, value: &str) -> Self::String {
        vm::StringHandle::new(self.context.intern_string(value))
    }

    fn store_string_option(&mut self, value: Option<&String>) -> Self::String {
        match value {
            Some(value) => self.store_string(value),
            None => Self::none_sentinel(),
        }
    }
}

/// Take a runtime error by id and normalize it as a platform error.
pub fn take_platform_error(
    errors: &RuntimeErrorStore,
    error_id: RuntimeErrorId,
) -> DiagnosticPlatformError {
    let error = errors.take(error_id).unwrap_or_else(|| {
        RuntimeError::Internal {
            message: "missing runtime error for id".to_string(),
        }
        .boxed()
    });

    DiagnosticPlatformError::from(error.as_ref())
}

/// Convert a diagnostic platform error into ABI-ready fields.
pub fn platform_error_fields<S: PlatformErrorStringStore>(
    store: &mut S,
    error: &DiagnosticPlatformError,
) -> PlatformErrorFields<S::String> {
    PlatformErrorFields {
        kind: store.store_string(kind_str(error.kind)),
        message: store.store_string(&error.message),
        name: store.store_string_option(error.name.as_ref()),
        code: map_platform_error_code(error.code.unwrap_or(DiagnosticPlatformErrorCode::Generic)),
        system_code: store.store_string_option(error.system_code.as_ref()),
        errno: error.errno.unwrap_or(0),
        syscall: store.store_string_option(error.syscall.as_ref()),
        path: store.store_string_option(error.path.as_ref()),
        dest: store.store_string_option(error.dest.as_ref()),
        fd: error.fd.unwrap_or(0),
        address: store.store_string_option(error.address.as_ref()),
        port: error.port.unwrap_or(0),
        hostname: store.store_string_option(error.hostname.as_ref()),
        signal: store.store_string_option(error.signal.as_ref()),
        exit_code: error.exit_code.unwrap_or(0),
        cause: store.store_string_option(error.cause.as_ref()),
        argument: store.store_string_option(error.argument.as_ref()),
        pointer: store.store_string_option(error.pointer.as_ref()),
        feature: store.store_string_option(error.feature.as_ref()),
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

#[cfg(test)]
mod tests {
    use super::VmStringStore;

    #[test]
    fn test_vm_string_none_sentinel_is_void() {
        let sentinel = VmStringStore::none_sentinel();
        assert!(sentinel.value().is_void_value());
    }
}
