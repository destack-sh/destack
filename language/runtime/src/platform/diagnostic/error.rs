use std::fmt;

#[cfg(any(unix, windows))]
use libc::{
    EACCES, EADDRINUSE, EADDRNOTAVAIL, EAGAIN, EALREADY, EBADF, EBUSY, ECONNABORTED, ECONNREFUSED,
    ECONNRESET, EEXIST, EFBIG, EHOSTUNREACH, EINPROGRESS, EINTR, EINVAL, EISCONN, EISDIR, EMFILE,
    EMSGSIZE, ENAMETOOLONG, ENETUNREACH, ENFILE, ENOBUFS, ENOENT, ENOTCONN, ENOTDIR, ENOTEMPTY,
    ENOTSOCK, EPIPE, EPROTONOSUPPORT, EROFS, ETIMEDOUT, EWOULDBLOCK, EXDEV,
};
#[cfg(unix)]
use libc::{EAFNOSUPPORT, EAI_AGAIN, EAI_FAIL, EAI_NONAME, ESHUTDOWN};
#[cfg(windows)]
use windows_sys::Win32::Networking::WinSock::{
    WSAEADDRINUSE, WSAEADDRNOTAVAIL, WSAEAFNOSUPPORT, WSAEALREADY, WSAECONNABORTED,
    WSAECONNREFUSED, WSAECONNRESET, WSAEHOSTUNREACH, WSAEINPROGRESS, WSAEISCONN, WSAEMSGSIZE,
    WSAENETUNREACH, WSAENOBUFS, WSAENOTCONN, WSAENOTSOCK, WSAEPROTONOSUPPORT, WSAESHUTDOWN,
    WSAETIMEDOUT, WSAEWOULDBLOCK,
};

use serde::{Deserialize, Serialize};

use destack_vm as vm;

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
    /// Socket is not connected.
    NetNotConnected = 3110,
    /// Socket is already connected.
    NetAlreadyConnected = 3111,
    /// Message is too large for target transport.
    NetMessageTooLarge = 3112,
    /// Handle is not a socket.
    NetNotSocket = 3113,
    /// Protocol operation failed.
    NetProtocolError = 3114,
    /// Operation is in progress.
    NetInProgress = 3115,
    /// Socket is shutdown for this operation.
    NetShutdown = 3116,
    /// Address family is not supported.
    NetUnsupportedFamily = 3117,
    /// Protocol is not supported.
    NetUnsupportedProtocol = 3118,
    /// No buffer space available.
    NetNoBufferSpace = 3119,

    /// Generic process error.
    Process = 4000,
    /// Process spawn failed.
    ProcessSpawnFailed = 4100,
    /// Process not found.
    ProcessNotFound = 4101,
    /// Process permission denied.
    ProcessPermissionDenied = 4102,
    /// Process exec failed.
    ProcessExecFailed = 4103,
    /// Process wait failed.
    ProcessWaitFailed = 4104,
    /// Process terminated by signal.
    ProcessSignaled = 4105,
    /// Process operation timed out.
    ProcessTimedOut = 4106,

    /// Randomness error.
    Random = 5000,
    /// Randomness unavailable.
    RandomUnavailable = 5100,

    /// Time source error.
    Time = 6000,
    /// Time source unavailable.
    TimeUnavailable = 6100,

    /// Generic IPC error.
    Ipc = 7000,
    /// IPC payload too large.
    IpcMessageTooLarge = 7100,
    /// IPC operation timed out.
    IpcTimedOut = 7101,
    /// IPC endpoint is closed.
    IpcClosed = 7102,
    /// IPC operation would block.
    IpcWouldBlock = 7103,
    /// IPC resource already exists.
    IpcAlreadyExists = 7104,

    /// Generic security error.
    Security = 7200,
    /// Security policy denied operation.
    SecurityDenied = 7210,
    /// Security policy violation.
    SecurityViolation = 7211,

    /// Generic thread error.
    Thread = 7300,
    /// Thread spawn failed.
    ThreadSpawnFailed = 7310,
    /// Thread join failed.
    ThreadJoinFailed = 7311,
    /// Thread synchronization deadlock.
    ThreadDeadlock = 7312,

    /// Generic FFI error.
    Ffi = 7400,
    /// FFI library load failed.
    FfiLibraryLoadFailed = 7410,
    /// FFI symbol not found.
    FfiSymbolNotFound = 7411,
    /// FFI call failed.
    FfiCallFailed = 7412,

    /// Generic device error.
    Device = 7500,
    /// Device unavailable.
    DeviceUnavailable = 7510,

    /// Generic display error.
    Display = 7600,
    /// Display unavailable.
    DisplayUnavailable = 7610,

    /// Generic audio error.
    Audio = 7700,
    /// Audio device unavailable.
    AudioUnavailable = 7710,

    /// Generic GPU error.
    Gpu = 7800,
    /// GPU device unavailable.
    GpuUnavailable = 7810,
    /// GPU out of memory.
    GpuOutOfMemory = 7811,
    /// GPU device lost.
    GpuDeviceLost = 7812,

    /// Generic resource-table error.
    Resource = 7900,
    /// Resource not found.
    ResourceNotFound = 7910,
    /// Resource already closed.
    ResourceClosed = 7911,
    /// Resource busy.
    ResourceBusy = 7912,
    /// Resource type mismatch.
    ResourceTypeMismatch = 7913,

    /// Generic I/O driver error.
    IoDriver = 8000,
    /// I/O submission failed.
    IoSubmissionFailed = 8010,
    /// I/O completion failed.
    IoCompletionFailed = 8011,
    /// I/O operation canceled.
    IoCancelled = 8012,

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
            PlatformErrorCode::NetNotConnected => "netNotConnected",
            PlatformErrorCode::NetAlreadyConnected => "netAlreadyConnected",
            PlatformErrorCode::NetMessageTooLarge => "netMessageTooLarge",
            PlatformErrorCode::NetNotSocket => "netNotSocket",
            PlatformErrorCode::NetProtocolError => "netProtocolError",
            PlatformErrorCode::NetInProgress => "netInProgress",
            PlatformErrorCode::NetShutdown => "netShutdown",
            PlatformErrorCode::NetUnsupportedFamily => "netUnsupportedFamily",
            PlatformErrorCode::NetUnsupportedProtocol => "netUnsupportedProtocol",
            PlatformErrorCode::NetNoBufferSpace => "netNoBufferSpace",

            PlatformErrorCode::Process => "process",
            PlatformErrorCode::ProcessSpawnFailed => "processSpawnFailed",
            PlatformErrorCode::ProcessNotFound => "processNotFound",
            PlatformErrorCode::ProcessPermissionDenied => "processPermissionDenied",
            PlatformErrorCode::ProcessExecFailed => "processExecFailed",
            PlatformErrorCode::ProcessWaitFailed => "processWaitFailed",
            PlatformErrorCode::ProcessSignaled => "processSignaled",
            PlatformErrorCode::ProcessTimedOut => "processTimedOut",

            PlatformErrorCode::Random => "random",
            PlatformErrorCode::RandomUnavailable => "randomUnavailable",

            PlatformErrorCode::Time => "time",
            PlatformErrorCode::TimeUnavailable => "timeUnavailable",

            PlatformErrorCode::Ipc => "ipc",
            PlatformErrorCode::IpcMessageTooLarge => "ipcMessageTooLarge",
            PlatformErrorCode::IpcTimedOut => "ipcTimedOut",
            PlatformErrorCode::IpcClosed => "ipcClosed",
            PlatformErrorCode::IpcWouldBlock => "ipcWouldBlock",
            PlatformErrorCode::IpcAlreadyExists => "ipcAlreadyExists",

            PlatformErrorCode::Security => "security",
            PlatformErrorCode::SecurityDenied => "securityDenied",
            PlatformErrorCode::SecurityViolation => "securityViolation",

            PlatformErrorCode::Thread => "thread",
            PlatformErrorCode::ThreadSpawnFailed => "threadSpawnFailed",
            PlatformErrorCode::ThreadJoinFailed => "threadJoinFailed",
            PlatformErrorCode::ThreadDeadlock => "threadDeadlock",

            PlatformErrorCode::Ffi => "ffi",
            PlatformErrorCode::FfiLibraryLoadFailed => "ffiLibraryLoadFailed",
            PlatformErrorCode::FfiSymbolNotFound => "ffiSymbolNotFound",
            PlatformErrorCode::FfiCallFailed => "ffiCallFailed",

            PlatformErrorCode::Device => "device",
            PlatformErrorCode::DeviceUnavailable => "deviceUnavailable",

            PlatformErrorCode::Display => "display",
            PlatformErrorCode::DisplayUnavailable => "displayUnavailable",

            PlatformErrorCode::Audio => "audio",
            PlatformErrorCode::AudioUnavailable => "audioUnavailable",

            PlatformErrorCode::Gpu => "gpu",
            PlatformErrorCode::GpuUnavailable => "gpuUnavailable",
            PlatformErrorCode::GpuOutOfMemory => "gpuOutOfMemory",
            PlatformErrorCode::GpuDeviceLost => "gpuDeviceLost",

            PlatformErrorCode::Resource => "resource",
            PlatformErrorCode::ResourceNotFound => "resourceNotFound",
            PlatformErrorCode::ResourceClosed => "resourceClosed",
            PlatformErrorCode::ResourceBusy => "resourceBusy",
            PlatformErrorCode::ResourceTypeMismatch => "resourceTypeMismatch",

            PlatformErrorCode::IoDriver => "ioDriver",
            PlatformErrorCode::IoSubmissionFailed => "ioSubmissionFailed",
            PlatformErrorCode::IoCompletionFailed => "ioCompletionFailed",
            PlatformErrorCode::IoCancelled => "ioCancelled",

            PlatformErrorCode::Generic => "generic",
        }
    }
}

/// Host-level system source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformSystemSourceKind {
    /// POSIX-style errno value.
    Errno,
    /// Winsock WSA error value.
    Winsock,
    /// HRESULT value.
    HResult,
    /// getaddrinfo/getnameinfo error value.
    Eai,
    /// Signal value.
    Signal,
    /// Other host source.
    Other,
}

/// Host-level system source metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformSystemSource {
    /// Host source kind.
    pub kind: PlatformSystemSourceKind,
    /// Numeric source value.
    pub value: i32,
    /// Optional symbolic source name.
    pub name: Option<String>,
}

/// Path payload encoding for error context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformPathEncoding {
    /// Raw byte payload.
    Bytes,
    /// UTF-16 little-endian payload.
    Utf16,
}

/// Path payload attached to an error context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformPathPayload {
    /// Path payload encoding.
    pub encoding: PlatformPathEncoding,
    /// Encoded path bytes.
    pub data: Vec<u8>,
}

/// Context kind for platform error metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformErrorContextKind {
    /// Generic context.
    Generic,
    /// File system or generic I/O context.
    Io,
    /// Network context.
    Net,
    /// Process context.
    Process,
    /// Timer context.
    Timer,
    /// Resource-table context.
    Resource,
    /// Security context.
    Security,
    /// FFI context.
    Ffi,
    /// Thread context.
    Thread,
    /// IPC context.
    Ipc,
    /// Device context.
    Device,
    /// Display context.
    Display,
    /// Audio context.
    Audio,
    /// GPU context.
    Gpu,
    /// I/O driver context.
    IoDriver,
}

/// Typed context for platform error metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformErrorContext {
    /// Context kind.
    pub kind: PlatformErrorContextKind,
    /// Optional syscall or host API name.
    pub syscall: Option<String>,
    /// Optional primary path payload.
    pub path: Option<PlatformPathPayload>,
    /// Optional destination path payload.
    pub dest: Option<PlatformPathPayload>,
    /// Optional primary path text fallback.
    pub path_text: Option<String>,
    /// Optional destination path text fallback.
    pub dest_text: Option<String>,
    /// Optional file descriptor.
    pub fd: Option<i32>,
    /// Optional network address text.
    pub address: Option<String>,
    /// Optional network port.
    pub port: Option<u16>,
    /// Optional host name.
    pub hostname: Option<String>,
    /// Optional process id.
    pub pid: Option<u64>,
    /// Optional signal name.
    pub signal: Option<String>,
    /// Optional exit code.
    pub exit_code: Option<i32>,
    /// Optional timer id.
    pub timer_id: Option<u64>,
    /// Optional timer deadline.
    pub deadline_ns: Option<u64>,
    /// Optional resource id.
    pub resource_id: Option<u64>,
    /// Optional resource kind.
    pub resource_kind: Option<String>,
    /// Optional capability name.
    pub capability: Option<String>,
    /// Optional policy name.
    pub policy: Option<String>,
    /// Optional FFI library name.
    pub library: Option<String>,
    /// Optional FFI symbol name.
    pub symbol: Option<String>,
    /// Optional thread id.
    pub thread_id: Option<u64>,
    /// Optional argument name.
    pub argument: Option<String>,
    /// Optional pointer label.
    pub pointer: Option<String>,
    /// Optional feature identifier.
    pub feature: Option<String>,
}

impl PlatformErrorContext {
    /// Build a context with a specific kind.
    pub fn with_kind(kind: PlatformErrorContextKind) -> Self {
        Self {
            kind,
            syscall: None,
            path: None,
            dest: None,
            path_text: None,
            dest_text: None,
            fd: None,
            address: None,
            port: None,
            hostname: None,
            pid: None,
            signal: None,
            exit_code: None,
            timer_id: None,
            deadline_ns: None,
            resource_id: None,
            resource_kind: None,
            capability: None,
            policy: None,
            library: None,
            symbol: None,
            thread_id: None,
            argument: None,
            pointer: None,
            feature: None,
        }
    }
}

/// Error type for platform binding handlers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformError {
    /// Stable platform error code.
    pub code: PlatformErrorCode,
    /// Optional binding operation name.
    pub op: Option<String>,
    /// Optional host-level system source metadata.
    pub source: Option<PlatformSystemSource>,
    /// Optional typed context.
    pub context: Option<PlatformErrorContext>,
    /// Optional human-readable message.
    pub message: Option<String>,
}

impl PlatformError {
    /// Build an invalid argument error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            code: PlatformErrorCode::InvalidArgument,
            op: None,
            source: None,
            context: None,
            message: Some(message.into()),
        }
    }

    /// Build an invalid argument error with a named argument.
    pub fn invalid_argument_named(argument: impl Into<String>, message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument(message);
        let mut context = PlatformErrorContext::with_kind(PlatformErrorContextKind::Generic);
        context.argument = Some(argument.into());
        error.context = Some(context);

        error
    }

    /// Build an invalid argument type error.
    pub fn invalid_argument_type(argument: impl Into<String>, expected: impl Into<String>) -> Self {
        let expected = expected.into();
        let mut error = Self::invalid_argument_named(argument, format!("expected {expected}"));
        error.code = PlatformErrorCode::InvalidArgumentType;

        error
    }

    /// Build an invalid argument value error.
    pub fn invalid_argument_value(argument: impl Into<String>, message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument_named(argument, message);
        error.code = PlatformErrorCode::InvalidArgumentValue;

        error
    }

    /// Build a null pointer error.
    pub fn null_pointer(pointer: impl Into<String>) -> Self {
        let pointer = pointer.into();
        let mut error = Self::invalid_argument(format!("null pointer: {pointer}"));
        error.code = PlatformErrorCode::NullPointer;

        let mut context = PlatformErrorContext::with_kind(PlatformErrorContextKind::Generic);
        context.pointer = Some(pointer);
        error.context = Some(context);

        error
    }

    /// Build a not supported error.
    pub fn not_supported(feature: impl Into<String>) -> Self {
        let feature = feature.into();
        let mut error = Self::invalid_argument(format!("not supported: {feature}"));
        error.code = PlatformErrorCode::NotSupported;

        let mut context = PlatformErrorContext::with_kind(PlatformErrorContextKind::Generic);
        context.feature = Some(feature);
        error.context = Some(context);

        error
    }

    /// Build a generic I/O error.
    pub fn io(message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument(message);
        error.code = PlatformErrorCode::Io;

        error
    }

    /// Build an invalid data I/O error.
    pub fn invalid_data(message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = PlatformErrorCode::IoInvalidData;

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
        error.code = code.or(mapped).unwrap_or(PlatformErrorCode::Io);
        error.source = source_from_errno(PlatformSystemSourceKind::Errno, errno, system_code);

        let mut context = PlatformErrorContext::with_kind(PlatformErrorContextKind::Io);
        context.syscall = syscall;
        context.path_text = path;
        error.context = Some(context);

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

        let mapped = errno.and_then(net_error_code_from_errno);
        error.code = code.or(mapped).unwrap_or(PlatformErrorCode::Net);
        error.source = source_from_errno(PlatformSystemSourceKind::Errno, errno, system_code);

        let mut context = PlatformErrorContext::with_kind(PlatformErrorContextKind::Net);
        context.syscall = syscall;
        context.address = address;
        context.port = port;
        error.context = Some(context);

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
        error.code = code.unwrap_or(PlatformErrorCode::Process);
        error.source = system_code.map(|name| PlatformSystemSource {
            kind: PlatformSystemSourceKind::Other,
            value: 0,
            name: Some(name),
        });

        let mut context = PlatformErrorContext::with_kind(PlatformErrorContextKind::Process);
        context.syscall = syscall;
        context.exit_code = exit_code;
        context.signal = signal;
        error.context = Some(context);

        error
    }

    /// Build a randomness error.
    pub fn random(code: Option<PlatformErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = code.unwrap_or(PlatformErrorCode::Random);

        error
    }

    /// Build a time error.
    pub fn time(code: Option<PlatformErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = code.unwrap_or(PlatformErrorCode::Time);

        error
    }

    /// Build a generic platform error.
    pub fn generic(code: Option<PlatformErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = code.unwrap_or(PlatformErrorCode::Generic);

        error
    }

    /// Return a human-readable error message.
    pub fn message(&self) -> String {
        if let Some(message) = &self.message {
            return message.clone();
        }

        self.code.as_str().to_string()
    }

    /// Boxed platform error for result propagation.
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    /// Return a sub-code for status mapping.
    pub fn sub_code(&self) -> u32 {
        self.code.number()
    }
}

/// Build a source object from errno fields.
fn source_from_errno(
    kind: PlatformSystemSourceKind,
    errno: Option<i32>,
    name: Option<String>,
) -> Option<PlatformSystemSource> {
    errno.map(|value| PlatformSystemSource { kind, value, name })
}

/// Map an errno value to an IO error code when possible.
#[cfg(any(unix, windows))]
pub fn io_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    if errno == EWOULDBLOCK || errno == EAGAIN {
        return Some(PlatformErrorCode::IoWouldBlock);
    }

    match errno {
        ENOENT => Some(PlatformErrorCode::IoNotFound),
        EACCES | libc::EPERM => Some(PlatformErrorCode::IoPermissionDenied),
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
        EBADF => Some(PlatformErrorCode::IoInvalidData),
        EXDEV => Some(PlatformErrorCode::IoCrossDevice),
        EPIPE => Some(PlatformErrorCode::IoBrokenPipe),
        ETIMEDOUT => Some(PlatformErrorCode::IoTimedOut),
        EINTR => Some(PlatformErrorCode::IoInterrupted),
        EBUSY => Some(PlatformErrorCode::IoBusy),
        _ => None,
    }
}

/// Map an errno value to an IO error code when possible.
#[cfg(not(any(unix, windows)))]
pub fn io_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    let _ = errno;

    // non-OS targets do not expose a stable errno domain here
    None
}

/// Map an errno value to a network error code when possible.
#[cfg(any(unix, windows))]
pub fn net_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    if errno == EWOULDBLOCK || errno == EAGAIN {
        return Some(PlatformErrorCode::IoWouldBlock);
    }

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
        ENOTCONN => Some(PlatformErrorCode::NetNotConnected),
        EISCONN => Some(PlatformErrorCode::NetAlreadyConnected),
        EMSGSIZE => Some(PlatformErrorCode::NetMessageTooLarge),
        ENOTSOCK => Some(PlatformErrorCode::NetNotSocket),
        EPROTONOSUPPORT => Some(PlatformErrorCode::NetUnsupportedProtocol),
        ENOBUFS => Some(PlatformErrorCode::NetNoBufferSpace),
        EINPROGRESS | EALREADY => Some(PlatformErrorCode::NetInProgress),
        #[cfg(unix)]
        ESHUTDOWN => Some(PlatformErrorCode::NetShutdown),
        #[cfg(unix)]
        EAFNOSUPPORT => Some(PlatformErrorCode::NetUnsupportedFamily),
        #[cfg(unix)]
        EAI_NONAME | EAI_FAIL | EAI_AGAIN => Some(PlatformErrorCode::NetDnsFailed),
        _ => None,
    }
}

/// Map an errno value to a network error code when possible.
#[cfg(not(any(unix, windows)))]
pub fn net_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    let _ = errno;

    // non-OS targets do not expose a stable errno domain here
    None
}

/// Map one Winsock error value to a network or I/O error code when possible.
#[cfg(windows)]
pub fn net_error_code_from_winsock(code: i32) -> Option<PlatformErrorCode> {
    match code {
        WSAEWOULDBLOCK => Some(PlatformErrorCode::IoWouldBlock),
        WSAECONNREFUSED => Some(PlatformErrorCode::NetConnectionRefused),
        WSAETIMEDOUT => Some(PlatformErrorCode::NetTimedOut),
        WSAECONNRESET => Some(PlatformErrorCode::NetConnectionReset),
        WSAEADDRINUSE => Some(PlatformErrorCode::NetAddressInUse),
        WSAEADDRNOTAVAIL => Some(PlatformErrorCode::NetAddressNotAvailable),
        WSAENETUNREACH => Some(PlatformErrorCode::NetNetworkUnreachable),
        WSAEHOSTUNREACH => Some(PlatformErrorCode::NetHostUnreachable),
        WSAECONNABORTED => Some(PlatformErrorCode::NetConnectionAborted),
        WSAENOTCONN => Some(PlatformErrorCode::NetNotConnected),
        WSAEISCONN => Some(PlatformErrorCode::NetAlreadyConnected),
        WSAEMSGSIZE => Some(PlatformErrorCode::NetMessageTooLarge),
        WSAENOTSOCK => Some(PlatformErrorCode::NetNotSocket),
        WSAEPROTONOSUPPORT => Some(PlatformErrorCode::NetUnsupportedProtocol),
        WSAENOBUFS => Some(PlatformErrorCode::NetNoBufferSpace),
        WSAEINPROGRESS | WSAEALREADY => Some(PlatformErrorCode::NetInProgress),
        WSAESHUTDOWN => Some(PlatformErrorCode::NetShutdown),
        WSAEAFNOSUPPORT => Some(PlatformErrorCode::NetUnsupportedFamily),
        _ => None,
    }
}

/// Map an errno value to a process error code when possible.
#[cfg(any(unix, windows))]
pub fn process_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    match errno {
        ENOENT => Some(PlatformErrorCode::ProcessNotFound),
        EACCES | libc::EPERM => Some(PlatformErrorCode::ProcessPermissionDenied),
        _ => None,
    }
}

/// Map an errno value to a process error code when possible.
#[cfg(not(any(unix, windows)))]
pub fn process_error_code_from_errno(errno: i32) -> Option<PlatformErrorCode> {
    let _ = errno;

    // non-OS targets do not expose a stable errno domain here
    None
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for PlatformError {}

impl From<PlatformError> for vm::Error {
    fn from(error: PlatformError) -> Self {
        if matches!(
            error.code,
            PlatformErrorCode::InvalidArgument
                | PlatformErrorCode::InvalidArgumentType
                | PlatformErrorCode::InvalidArgumentValue
        ) {
            return vm::Error::TypeMismatch {
                expected: "valid argument".to_string(),
                actual: error.message(),
            };
        }

        if error.code == PlatformErrorCode::NullPointer {
            return vm::Error::NullPointerDereference;
        }

        if error.code == PlatformErrorCode::NotSupported {
            let name = error
                .context
                .as_ref()
                .and_then(|context| context.feature.clone())
                .unwrap_or_else(|| error.message());
            return vm::Error::BindingCallForbidden { name };
        }

        vm::Error::Panic {
            message: error.message(),
        }
    }
}

/// Result type for platform binding handlers.
pub type PlatformResult<T> = Result<T, Box<PlatformError>>;
