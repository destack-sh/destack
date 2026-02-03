use std::fmt;

use libc::{
    EACCES, EADDRINUSE, EADDRNOTAVAIL, EAGAIN, EBUSY, ECONNABORTED, ECONNREFUSED, ECONNRESET,
    EEXIST, EFBIG, EHOSTUNREACH, EINTR, EINVAL, EISDIR, EMFILE, ENAMETOOLONG, ENETUNREACH, ENFILE,
    ENOENT, ENOTDIR, ENOTEMPTY, EPERM, EPIPE, EROFS, ETIMEDOUT, EWOULDBLOCK, EXDEV,
};
#[cfg(unix)]
use libc::{EAI_AGAIN, EAI_FAIL, EAI_NONAME};

use serde::{Deserialize, Serialize};

use destack_vm as vm;

/// Error kind for platform bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformErrorKind {
    /// Invalid arguments provided to a platform binding.
    InvalidArgument,
    /// Null pointer passed across the platform boundary.
    NullPointer,
    /// Operation not supported on the current platform.
    NotSupported,
    /// I/O error at the platform boundary.
    Io,
    /// Network error at the platform boundary.
    Net,
    /// Process error at the platform boundary.
    Process,
    /// Randomness or entropy error.
    Random,
    /// Time source error.
    Time,
    /// Generic platform error.
    Generic,
}

/// Error code for platform bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum PlatformErrorCode {
    /// Unclassified invalid argument.
    InvalidArgument = 1000,
    /// Invalid argument type.
    InvalidArgumentType = 1001,
    /// Invalid argument value.
    InvalidArgumentValue = 1002,
    /// Null pointer passed across the platform boundary.
    NullPointer = 1100,
    /// Feature is not supported.
    NotSupported = 1200,
    /// Generic I/O error.
    Io = 2000,
    /// I/O read failed.
    IoReadFailed = 2100,
    /// I/O write failed.
    IoWriteFailed = 2101,
    /// File or resource not found.
    IoNotFound = 2102,
    /// Permission denied.
    IoPermissionDenied = 2103,
    /// File or resource already exists.
    IoAlreadyExists = 2104,
    /// Path component is not a directory.
    IoNotDirectory = 2105,
    /// Path points to a directory where a file is required.
    IoIsDirectory = 2106,
    /// Directory is not empty.
    IoNotEmpty = 2107,
    /// Read-only file system.
    IoReadOnly = 2108,
    /// Path name is too long.
    IoNameTooLong = 2109,
    /// File is too large.
    IoFileTooLarge = 2110,
    /// Too many open files.
    IoTooManyOpenFiles = 2111,
    /// System file table is full.
    IoFileTableOverflow = 2112,
    /// Invalid data or format.
    IoInvalidData = 2113,
    /// Cross-device link not permitted.
    IoCrossDevice = 2114,
    /// Broken pipe.
    IoBrokenPipe = 2115,
    /// I/O timed out.
    IoTimedOut = 2116,
    /// I/O interrupted.
    IoInterrupted = 2117,
    /// Resource is busy.
    IoBusy = 2118,
    /// Operation would block.
    IoWouldBlock = 2119,
    /// Generic network error.
    Net = 3000,
    /// Connection refused.
    NetConnectionRefused = 3100,
    /// Connection timed out.
    NetTimedOut = 3101,
    /// Connection reset.
    NetConnectionReset = 3102,
    /// Address already in use.
    NetAddressInUse = 3103,
    /// Address not available.
    NetAddressNotAvailable = 3104,
    /// Network unreachable.
    NetNetworkUnreachable = 3105,
    /// Host unreachable.
    NetHostUnreachable = 3106,
    /// Connection aborted.
    NetConnectionAborted = 3107,
    /// Broken pipe.
    NetBrokenPipe = 3108,
    /// DNS lookup failed.
    NetDnsFailed = 3109,
    /// Generic process error.
    Process = 4000,
    /// Process spawn failed.
    ProcessSpawnFailed = 4100,
    /// Process not found.
    ProcessNotFound = 4101,
    /// Process permission denied.
    ProcessPermissionDenied = 4102,
    /// Randomness error.
    Random = 5000,
    /// Randomness unavailable.
    RandomUnavailable = 5100,
    /// Time source error.
    Time = 6000,
    /// Time source unavailable.
    TimeUnavailable = 6100,
    /// Generic platform error.
    Generic = 9000,
}

impl PlatformErrorCode {
    /// Return the numeric code value.
    pub const fn number(self) -> u32 {
        self as u16 as u32
    }

    /// Return a stable string representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            PlatformErrorCode::InvalidArgument => "invalidArgument",
            PlatformErrorCode::InvalidArgumentType => "invalidArgumentType",
            PlatformErrorCode::InvalidArgumentValue => "invalidArgumentValue",
            PlatformErrorCode::NullPointer => "nullPointer",
            PlatformErrorCode::NotSupported => "notSupported",
            PlatformErrorCode::Io => "io",
            PlatformErrorCode::IoReadFailed => "ioReadFailed",
            PlatformErrorCode::IoWriteFailed => "ioWriteFailed",
            PlatformErrorCode::IoNotFound => "ioNotFound",
            PlatformErrorCode::IoPermissionDenied => "ioPermissionDenied",
            PlatformErrorCode::IoAlreadyExists => "ioAlreadyExists",
            PlatformErrorCode::IoNotDirectory => "ioNotDirectory",
            PlatformErrorCode::IoIsDirectory => "ioIsDirectory",
            PlatformErrorCode::IoNotEmpty => "ioNotEmpty",
            PlatformErrorCode::IoReadOnly => "ioReadOnly",
            PlatformErrorCode::IoNameTooLong => "ioNameTooLong",
            PlatformErrorCode::IoFileTooLarge => "ioFileTooLarge",
            PlatformErrorCode::IoTooManyOpenFiles => "ioTooManyOpenFiles",
            PlatformErrorCode::IoFileTableOverflow => "ioFileTableOverflow",
            PlatformErrorCode::IoInvalidData => "ioInvalidData",
            PlatformErrorCode::IoCrossDevice => "ioCrossDevice",
            PlatformErrorCode::IoBrokenPipe => "ioBrokenPipe",
            PlatformErrorCode::IoTimedOut => "ioTimedOut",
            PlatformErrorCode::IoInterrupted => "ioInterrupted",
            PlatformErrorCode::IoBusy => "ioBusy",
            PlatformErrorCode::IoWouldBlock => "ioWouldBlock",
            PlatformErrorCode::Net => "net",
            PlatformErrorCode::NetConnectionRefused => "netConnectionRefused",
            PlatformErrorCode::NetTimedOut => "netTimedOut",
            PlatformErrorCode::NetConnectionReset => "netConnectionReset",
            PlatformErrorCode::NetAddressInUse => "netAddressInUse",
            PlatformErrorCode::NetAddressNotAvailable => "netAddressNotAvailable",
            PlatformErrorCode::NetNetworkUnreachable => "netNetworkUnreachable",
            PlatformErrorCode::NetHostUnreachable => "netHostUnreachable",
            PlatformErrorCode::NetConnectionAborted => "netConnectionAborted",
            PlatformErrorCode::NetBrokenPipe => "netBrokenPipe",
            PlatformErrorCode::NetDnsFailed => "netDnsFailed",
            PlatformErrorCode::Process => "process",
            PlatformErrorCode::ProcessSpawnFailed => "processSpawnFailed",
            PlatformErrorCode::ProcessNotFound => "processNotFound",
            PlatformErrorCode::ProcessPermissionDenied => "processPermissionDenied",
            PlatformErrorCode::Random => "random",
            PlatformErrorCode::RandomUnavailable => "randomUnavailable",
            PlatformErrorCode::Time => "time",
            PlatformErrorCode::TimeUnavailable => "timeUnavailable",
            PlatformErrorCode::Generic => "generic",
        }
    }
}

/// Error type for platform binding handlers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformError {
    /// Error kind tag for platform diagnostics.
    pub kind: PlatformErrorKind,
    /// Human-readable error message.
    pub message: String,
    /// Optional error class name.
    pub name: Option<String>,
    /// Optional error code.
    pub code: Option<PlatformErrorCode>,
    /// Optional raw system error code (e.g. ENOENT, UV_ECONNRESET).
    pub system_code: Option<String>,
    /// Optional OS errno.
    pub errno: Option<i32>,
    /// Optional syscall name.
    pub syscall: Option<String>,
    /// Optional path associated with the failure.
    pub path: Option<String>,
    /// Optional destination path.
    pub dest: Option<String>,
    /// Optional file descriptor.
    pub fd: Option<i32>,
    /// Optional address involved in the failure.
    pub address: Option<String>,
    /// Optional port involved in the failure.
    pub port: Option<u16>,
    /// Optional hostname involved in the failure.
    pub hostname: Option<String>,
    /// Optional signal name.
    pub signal: Option<String>,
    /// Optional exit code.
    pub exit_code: Option<i32>,
    /// Optional error cause.
    pub cause: Option<String>,
    /// Optional argument name or label.
    pub argument: Option<String>,
    /// Optional pointer or parameter name.
    pub pointer: Option<String>,
    /// Optional feature identifier.
    pub feature: Option<String>,
}

impl PlatformError {
    /// Build an invalid argument error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            kind: PlatformErrorKind::InvalidArgument,
            message: message.into(),
            name: None,
            code: Some(PlatformErrorCode::InvalidArgument),
            system_code: None,
            errno: None,
            syscall: None,
            path: None,
            dest: None,
            fd: None,
            address: None,
            port: None,
            hostname: None,
            signal: None,
            exit_code: None,
            cause: None,
            argument: None,
            pointer: None,
            feature: None,
        }
    }

    /// Build an invalid argument error with a named argument.
    pub fn invalid_argument_named(argument: impl Into<String>, message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument(message);
        error.argument = Some(argument.into());
        error
    }

    /// Build an invalid argument type error.
    pub fn invalid_argument_type(argument: impl Into<String>, expected: impl Into<String>) -> Self {
        let expected = expected.into();
        let mut error = Self::invalid_argument_named(argument, format!("expected {expected}"));
        error.code = Some(PlatformErrorCode::InvalidArgumentType);
        error
    }

    /// Build an invalid argument value error.
    pub fn invalid_argument_value(argument: impl Into<String>, message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument_named(argument, message);
        error.code = Some(PlatformErrorCode::InvalidArgumentValue);
        error
    }

    /// Build a null pointer error.
    pub fn null_pointer(pointer: impl Into<String>) -> Self {
        let pointer = pointer.into();
        let mut error = Self::invalid_argument(format!("null pointer: {pointer}"));
        error.kind = PlatformErrorKind::NullPointer;
        error.code = Some(PlatformErrorCode::NullPointer);
        error.pointer = Some(pointer);
        error
    }

    /// Build a not supported error.
    pub fn not_supported(feature: impl Into<String>) -> Self {
        let feature = feature.into();
        let mut error = Self::invalid_argument(format!("not supported: {feature}"));
        error.kind = PlatformErrorKind::NotSupported;
        error.code = Some(PlatformErrorCode::NotSupported);
        error.feature = Some(feature);
        error
    }

    /// Build a generic I/O error.
    pub fn io(message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument(message);
        error.kind = PlatformErrorKind::Io;
        error.code = Some(PlatformErrorCode::Io);
        error
    }

    /// Build an I/O error with context fields.
    pub fn io_with(
        code: Option<PlatformErrorCode>,
        system_code: Option<String>,
        errno: Option<i32>,
        syscall: Option<String>,
        path: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        let mut error = Self::io(message);
        let mapped = errno.and_then(io_error_code_from_errno);
        error.code = code.or(mapped).or(error.code);
        error.system_code = system_code;
        error.errno = errno;
        error.syscall = syscall;
        error.path = path;
        error
    }

    /// Build a network error with context fields.
    pub fn net_with(
        code: Option<PlatformErrorCode>,
        system_code: Option<String>,
        errno: Option<i32>,
        syscall: Option<String>,
        address: Option<String>,
        port: Option<u16>,
        message: impl Into<String>,
    ) -> Self {
        let mut error = Self::io(message);
        error.kind = PlatformErrorKind::Net;
        let mapped = errno.and_then(net_error_code_from_errno);
        error.code = code.or(mapped).or(Some(PlatformErrorCode::Net));
        error.system_code = system_code;
        error.errno = errno;
        error.syscall = syscall;
        error.address = address;
        error.port = port;
        error
    }

    /// Build a process error with context fields.
    pub fn process_with(
        code: Option<PlatformErrorCode>,
        system_code: Option<String>,
        exit_code: Option<i32>,
        signal: Option<String>,
        syscall: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        let mut error = Self::io(message);
        error.kind = PlatformErrorKind::Process;
        error.code = code.or(Some(PlatformErrorCode::Process));
        error.system_code = system_code;
        error.exit_code = exit_code;
        error.signal = signal;
        error.syscall = syscall;
        error
    }

    /// Build a randomness error.
    pub fn random(code: Option<PlatformErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.kind = PlatformErrorKind::Random;
        error.code = code.or(Some(PlatformErrorCode::Random));
        error
    }

    /// Build a time error.
    pub fn time(code: Option<PlatformErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.kind = PlatformErrorKind::Time;
        error.code = code.or(Some(PlatformErrorCode::Time));
        error
    }

    /// Build a generic platform error.
    pub fn generic(code: Option<PlatformErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.kind = PlatformErrorKind::Generic;
        error.code = code.or(Some(PlatformErrorCode::Generic));
        error
    }

    /// Return a human-readable error message.
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// Boxed platform error for result propagation.
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    /// Return a sub-code for status mapping.
    pub fn sub_code(&self) -> u32 {
        self.code
            .map(PlatformErrorCode::number)
            .unwrap_or_else(|| match self.kind {
                PlatformErrorKind::InvalidArgument => PlatformErrorCode::InvalidArgument.number(),
                PlatformErrorKind::NullPointer => PlatformErrorCode::NullPointer.number(),
                PlatformErrorKind::NotSupported => PlatformErrorCode::NotSupported.number(),
                PlatformErrorKind::Io => PlatformErrorCode::Io.number(),
                PlatformErrorKind::Net => PlatformErrorCode::Net.number(),
                PlatformErrorKind::Process => PlatformErrorCode::Process.number(),
                PlatformErrorKind::Random => PlatformErrorCode::Random.number(),
                PlatformErrorKind::Time => PlatformErrorCode::Time.number(),
                PlatformErrorKind::Generic => PlatformErrorCode::Generic.number(),
            })
    }
}

/// Map an errno value to an IO error code when possible.
pub fn io_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    if errno == EWOULDBLOCK || errno == EAGAIN {
        return Some(PlatformErrorCode::IoWouldBlock);
    }

    match errno {
        ENOENT => Some(PlatformErrorCode::IoNotFound),
        EACCES | EPERM => Some(PlatformErrorCode::IoPermissionDenied),
        EEXIST => Some(PlatformErrorCode::IoAlreadyExists),
        ENOTDIR => Some(PlatformErrorCode::IoNotDirectory),
        EISDIR => Some(PlatformErrorCode::IoIsDirectory),
        ENOTEMPTY => Some(PlatformErrorCode::IoNotEmpty),
        EROFS => Some(PlatformErrorCode::IoReadOnly),
        ENAMETOOLONG => Some(PlatformErrorCode::IoNameTooLong),
        EFBIG => Some(PlatformErrorCode::IoFileTooLarge),
        EMFILE => Some(PlatformErrorCode::IoTooManyOpenFiles),
        ENFILE => Some(PlatformErrorCode::IoFileTableOverflow),
        EINVAL => Some(PlatformErrorCode::IoInvalidData),
        EXDEV => Some(PlatformErrorCode::IoCrossDevice),
        EPIPE => Some(PlatformErrorCode::IoBrokenPipe),
        ETIMEDOUT => Some(PlatformErrorCode::IoTimedOut),
        EINTR => Some(PlatformErrorCode::IoInterrupted),
        EBUSY => Some(PlatformErrorCode::IoBusy),
        _ => None,
    }
}

/// Map an errno value to a network error code when possible.
pub fn net_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    match errno {
        ECONNREFUSED => Some(PlatformErrorCode::NetConnectionRefused),
        ETIMEDOUT => Some(PlatformErrorCode::NetTimedOut),
        ECONNRESET => Some(PlatformErrorCode::NetConnectionReset),
        EADDRINUSE => Some(PlatformErrorCode::NetAddressInUse),
        EADDRNOTAVAIL => Some(PlatformErrorCode::NetAddressNotAvailable),
        ENETUNREACH => Some(PlatformErrorCode::NetNetworkUnreachable),
        EHOSTUNREACH => Some(PlatformErrorCode::NetHostUnreachable),
        ECONNABORTED => Some(PlatformErrorCode::NetConnectionAborted),
        EPIPE => Some(PlatformErrorCode::NetBrokenPipe),
        #[cfg(unix)]
        EAI_NONAME | EAI_FAIL | EAI_AGAIN => Some(PlatformErrorCode::NetDnsFailed),
        _ => None,
    }
}

/// Map an errno value to a process error code when possible.
pub fn process_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    match errno {
        ENOENT => Some(PlatformErrorCode::ProcessNotFound),
        EACCES | EPERM => Some(PlatformErrorCode::ProcessPermissionDenied),
        _ => None,
    }
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PlatformError {}

impl From<PlatformError> for vm::Error {
    fn from(error: PlatformError) -> Self {
        match error.kind {
            PlatformErrorKind::InvalidArgument => vm::Error::TypeMismatch {
                expected: "valid argument".to_string(),
                actual: error.message,
            },
            PlatformErrorKind::NullPointer => vm::Error::NullPointerDereference,
            PlatformErrorKind::NotSupported => vm::Error::ExternalCallForbidden {
                name: error.feature.unwrap_or(error.message),
            },
            _ => vm::Error::Panic {
                message: error.message,
            },
        }
    }
}

/// Result type for platform binding handlers.
pub type PlatformResult<T> = Result<T, Box<PlatformError>>;
