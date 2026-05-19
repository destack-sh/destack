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

/// Error code for host bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum HostErrorCode {
    /// Unclassified invalid argument.
    InvalidArgument = 1000,
    /// Invalid argument type.
    InvalidArgumentType = 1001,
    /// Invalid argument value.
    InvalidArgumentValue = 1002,
    /// Null pointer passed across the host boundary.
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

    /// Generic host error.
    Generic = 9000,
}

impl HostErrorCode {
    /// Return the numeric code value.
    pub const fn number(self) -> u32 {
        self as u16 as u32
    }

    /// Return a stable string representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            HostErrorCode::InvalidArgument => "invalidArgument",
            HostErrorCode::InvalidArgumentType => "invalidArgumentType",
            HostErrorCode::InvalidArgumentValue => "invalidArgumentValue",
            HostErrorCode::NullPointer => "nullPointer",
            HostErrorCode::NotSupported => "notSupported",

            HostErrorCode::Io => "io",
            HostErrorCode::IoReadFailed => "ioReadFailed",
            HostErrorCode::IoWriteFailed => "ioWriteFailed",
            HostErrorCode::IoNotFound => "ioNotFound",
            HostErrorCode::IoPermissionDenied => "ioPermissionDenied",
            HostErrorCode::IoAlreadyExists => "ioAlreadyExists",
            HostErrorCode::IoNotDirectory => "ioNotDirectory",
            HostErrorCode::IoIsDirectory => "ioIsDirectory",
            HostErrorCode::IoNotEmpty => "ioNotEmpty",
            HostErrorCode::IoReadOnly => "ioReadOnly",
            HostErrorCode::IoNameTooLong => "ioNameTooLong",
            HostErrorCode::IoFileTooLarge => "ioFileTooLarge",
            HostErrorCode::IoTooManyOpenFiles => "ioTooManyOpenFiles",
            HostErrorCode::IoFileTableOverflow => "ioFileTableOverflow",
            HostErrorCode::IoInvalidData => "ioInvalidData",
            HostErrorCode::IoCrossDevice => "ioCrossDevice",
            HostErrorCode::IoBrokenPipe => "ioBrokenPipe",
            HostErrorCode::IoTimedOut => "ioTimedOut",
            HostErrorCode::IoInterrupted => "ioInterrupted",
            HostErrorCode::IoBusy => "ioBusy",
            HostErrorCode::IoWouldBlock => "ioWouldBlock",

            HostErrorCode::Net => "net",
            HostErrorCode::NetConnectionRefused => "netConnectionRefused",
            HostErrorCode::NetTimedOut => "netTimedOut",
            HostErrorCode::NetConnectionReset => "netConnectionReset",
            HostErrorCode::NetAddressInUse => "netAddressInUse",
            HostErrorCode::NetAddressNotAvailable => "netAddressNotAvailable",
            HostErrorCode::NetNetworkUnreachable => "netNetworkUnreachable",
            HostErrorCode::NetHostUnreachable => "netHostUnreachable",
            HostErrorCode::NetConnectionAborted => "netConnectionAborted",
            HostErrorCode::NetBrokenPipe => "netBrokenPipe",
            HostErrorCode::NetDnsFailed => "netDnsFailed",
            HostErrorCode::NetNotConnected => "netNotConnected",
            HostErrorCode::NetAlreadyConnected => "netAlreadyConnected",
            HostErrorCode::NetMessageTooLarge => "netMessageTooLarge",
            HostErrorCode::NetNotSocket => "netNotSocket",
            HostErrorCode::NetProtocolError => "netProtocolError",
            HostErrorCode::NetInProgress => "netInProgress",
            HostErrorCode::NetShutdown => "netShutdown",
            HostErrorCode::NetUnsupportedFamily => "netUnsupportedFamily",
            HostErrorCode::NetUnsupportedProtocol => "netUnsupportedProtocol",
            HostErrorCode::NetNoBufferSpace => "netNoBufferSpace",

            HostErrorCode::Process => "process",
            HostErrorCode::ProcessSpawnFailed => "processSpawnFailed",
            HostErrorCode::ProcessNotFound => "processNotFound",
            HostErrorCode::ProcessPermissionDenied => "processPermissionDenied",
            HostErrorCode::ProcessExecFailed => "processExecFailed",
            HostErrorCode::ProcessWaitFailed => "processWaitFailed",
            HostErrorCode::ProcessSignaled => "processSignaled",
            HostErrorCode::ProcessTimedOut => "processTimedOut",

            HostErrorCode::Random => "random",
            HostErrorCode::RandomUnavailable => "randomUnavailable",

            HostErrorCode::Time => "time",
            HostErrorCode::TimeUnavailable => "timeUnavailable",

            HostErrorCode::Ipc => "ipc",
            HostErrorCode::IpcMessageTooLarge => "ipcMessageTooLarge",
            HostErrorCode::IpcTimedOut => "ipcTimedOut",
            HostErrorCode::IpcClosed => "ipcClosed",
            HostErrorCode::IpcWouldBlock => "ipcWouldBlock",
            HostErrorCode::IpcAlreadyExists => "ipcAlreadyExists",

            HostErrorCode::Security => "security",
            HostErrorCode::SecurityDenied => "securityDenied",
            HostErrorCode::SecurityViolation => "securityViolation",

            HostErrorCode::Thread => "thread",
            HostErrorCode::ThreadSpawnFailed => "threadSpawnFailed",
            HostErrorCode::ThreadJoinFailed => "threadJoinFailed",
            HostErrorCode::ThreadDeadlock => "threadDeadlock",

            HostErrorCode::Ffi => "ffi",
            HostErrorCode::FfiLibraryLoadFailed => "ffiLibraryLoadFailed",
            HostErrorCode::FfiSymbolNotFound => "ffiSymbolNotFound",
            HostErrorCode::FfiCallFailed => "ffiCallFailed",

            HostErrorCode::Device => "device",
            HostErrorCode::DeviceUnavailable => "deviceUnavailable",

            HostErrorCode::Display => "display",
            HostErrorCode::DisplayUnavailable => "displayUnavailable",

            HostErrorCode::Audio => "audio",
            HostErrorCode::AudioUnavailable => "audioUnavailable",

            HostErrorCode::Gpu => "gpu",
            HostErrorCode::GpuUnavailable => "gpuUnavailable",
            HostErrorCode::GpuOutOfMemory => "gpuOutOfMemory",
            HostErrorCode::GpuDeviceLost => "gpuDeviceLost",

            HostErrorCode::Resource => "resource",
            HostErrorCode::ResourceNotFound => "resourceNotFound",
            HostErrorCode::ResourceClosed => "resourceClosed",
            HostErrorCode::ResourceBusy => "resourceBusy",
            HostErrorCode::ResourceTypeMismatch => "resourceTypeMismatch",

            HostErrorCode::IoDriver => "ioDriver",
            HostErrorCode::IoSubmissionFailed => "ioSubmissionFailed",
            HostErrorCode::IoCompletionFailed => "ioCompletionFailed",
            HostErrorCode::IoCancelled => "ioCancelled",

            HostErrorCode::Generic => "generic",
        }
    }
}

/// Host-level system source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostSystemSourceKind {
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
pub struct HostSystemSource {
    /// Host source kind.
    pub kind: HostSystemSourceKind,
    /// Numeric source value.
    pub value: i32,
    /// Optional symbolic source name.
    pub name: Option<String>,
}

/// Path payload encoding for error context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostPathEncoding {
    /// Raw byte payload.
    Bytes,
    /// UTF-16 little-endian payload.
    Utf16,
}

/// Path payload attached to an error context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPathPayload {
    /// Path payload encoding.
    pub encoding: HostPathEncoding,
    /// Encoded path bytes.
    pub data: Vec<u8>,
}

/// Context kind for host error metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostErrorContextKind {
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

/// Typed context for host error metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostErrorContext {
    /// Context kind.
    pub kind: HostErrorContextKind,
    /// Optional syscall or host API name.
    pub syscall: Option<String>,
    /// Optional primary path payload.
    pub path: Option<HostPathPayload>,
    /// Optional destination path payload.
    pub dest: Option<HostPathPayload>,
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

impl HostErrorContext {
    /// Build a context with a specific kind.
    pub fn with_kind(kind: HostErrorContextKind) -> Self {
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

/// Error type for host binding handlers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostError {
    /// Stable host error code.
    pub code: HostErrorCode,
    /// Optional binding operation name.
    pub op: Option<String>,
    /// Optional host-level system source metadata.
    pub source: Option<HostSystemSource>,
    /// Optional typed context.
    pub context: Option<HostErrorContext>,
    /// Optional human-readable message.
    pub message: Option<String>,
}

impl HostError {
    /// Build an invalid argument error.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            code: HostErrorCode::InvalidArgument,
            op: None,
            source: None,
            context: None,
            message: Some(message.into()),
        }
    }

    /// Build an invalid argument error with a named argument.
    pub fn invalid_argument_named(argument: impl Into<String>, message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument(message);
        let mut context = HostErrorContext::with_kind(HostErrorContextKind::Generic);
        context.argument = Some(argument.into());
        error.context = Some(context);

        error
    }

    /// Build an invalid argument type error.
    pub fn invalid_argument_type(argument: impl Into<String>, expected: impl Into<String>) -> Self {
        let expected = expected.into();
        let mut error = Self::invalid_argument_named(argument, format!("expected {expected}"));
        error.code = HostErrorCode::InvalidArgumentType;

        error
    }

    /// Build an invalid argument value error.
    pub fn invalid_argument_value(argument: impl Into<String>, message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument_named(argument, message);
        error.code = HostErrorCode::InvalidArgumentValue;

        error
    }

    /// Build a null pointer error.
    pub fn null_pointer(pointer: impl Into<String>) -> Self {
        let pointer = pointer.into();
        let mut error = Self::invalid_argument(format!("null pointer: {pointer}"));
        error.code = HostErrorCode::NullPointer;

        let mut context = HostErrorContext::with_kind(HostErrorContextKind::Generic);
        context.pointer = Some(pointer);
        error.context = Some(context);

        error
    }

    /// Build a not supported error.
    pub fn not_supported(feature: impl Into<String>) -> Self {
        let feature = feature.into();
        let mut error = Self::invalid_argument(format!("not supported: {feature}"));
        error.code = HostErrorCode::NotSupported;

        let mut context = HostErrorContext::with_kind(HostErrorContextKind::Generic);
        context.feature = Some(feature);
        error.context = Some(context);

        error
    }

    /// Build a generic I/O error.
    pub fn io(message: impl Into<String>) -> Self {
        let mut error = Self::invalid_argument(message);
        error.code = HostErrorCode::Io;

        error
    }

    /// Build an invalid data I/O error.
    pub fn invalid_data(message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = HostErrorCode::IoInvalidData;

        error
    }

    /// Build an I/O error with context fields.
    pub fn io_with(
        code: Option<HostErrorCode>,
        system_code: Option<String>,
        errno: Option<i32>,
        syscall: Option<String>,
        path: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        let mut error = Self::io(message);

        let mapped = errno.and_then(io_error_code_from_errno);
        error.code = code.or(mapped).unwrap_or(HostErrorCode::Io);
        error.source = source_from_errno(HostSystemSourceKind::Errno, errno, system_code);

        let mut context = HostErrorContext::with_kind(HostErrorContextKind::Io);
        context.syscall = syscall;
        context.path_text = path;
        error.context = Some(context);

        error
    }

    /// Build a network error with context fields.
    pub fn net_with(
        code: Option<HostErrorCode>,
        system_code: Option<String>,
        errno: Option<i32>,
        syscall: Option<String>,
        address: Option<String>,
        port: Option<u16>,
        message: impl Into<String>,
    ) -> Self {
        let mut error = Self::io(message);

        let mapped = errno.and_then(net_error_code_from_errno);
        error.code = code.or(mapped).unwrap_or(HostErrorCode::Net);
        error.source = source_from_errno(HostSystemSourceKind::Errno, errno, system_code);

        let mut context = HostErrorContext::with_kind(HostErrorContextKind::Net);
        context.syscall = syscall;
        context.address = address;
        context.port = port;
        error.context = Some(context);

        error
    }

    /// Build a process error with context fields.
    pub fn process_with(
        code: Option<HostErrorCode>,
        system_code: Option<String>,
        exit_code: Option<i32>,
        signal: Option<String>,
        syscall: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        let mut error = Self::io(message);
        error.code = code.unwrap_or(HostErrorCode::Process);
        error.source = system_code.map(|name| HostSystemSource {
            kind: HostSystemSourceKind::Other,
            value: 0,
            name: Some(name),
        });

        let mut context = HostErrorContext::with_kind(HostErrorContextKind::Process);
        context.syscall = syscall;
        context.exit_code = exit_code;
        context.signal = signal;
        error.context = Some(context);

        error
    }

    /// Build a randomness error.
    pub fn random(code: Option<HostErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = code.unwrap_or(HostErrorCode::Random);

        error
    }

    /// Build a time error.
    pub fn time(code: Option<HostErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = code.unwrap_or(HostErrorCode::Time);

        error
    }

    /// Build a generic host error.
    pub fn generic(code: Option<HostErrorCode>, message: impl Into<String>) -> Self {
        let mut error = Self::io(message);
        error.code = code.unwrap_or(HostErrorCode::Generic);

        error
    }

    /// Return a human-readable error message.
    pub fn message(&self) -> String {
        if let Some(message) = &self.message {
            return message.clone();
        }

        self.code.as_str().to_string()
    }

    /// Boxed host error for result propagation.
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
    kind: HostSystemSourceKind,
    errno: Option<i32>,
    name: Option<String>,
) -> Option<HostSystemSource> {
    errno.map(|value| HostSystemSource { kind, value, name })
}

/// Map an errno value to an IO error code when possible.
#[cfg(any(unix, windows))]
pub fn io_error_code_from_errno(errno: i32) -> Option<HostErrorCode> {
    if errno == EWOULDBLOCK || errno == EAGAIN {
        return Some(HostErrorCode::IoWouldBlock);
    }

    match errno {
        ENOENT => Some(HostErrorCode::IoNotFound),
        EACCES | libc::EPERM => Some(HostErrorCode::IoPermissionDenied),
        EEXIST => Some(HostErrorCode::IoAlreadyExists),
        ENOTDIR => Some(HostErrorCode::IoNotDirectory),
        EISDIR => Some(HostErrorCode::IoIsDirectory),
        ENOTEMPTY => Some(HostErrorCode::IoNotEmpty),
        EROFS => Some(HostErrorCode::IoReadOnly),
        ENAMETOOLONG => Some(HostErrorCode::IoNameTooLong),
        EFBIG => Some(HostErrorCode::IoFileTooLarge),
        EMFILE => Some(HostErrorCode::IoTooManyOpenFiles),
        ENFILE => Some(HostErrorCode::IoFileTableOverflow),
        EINVAL => Some(HostErrorCode::IoInvalidData),
        EBADF => Some(HostErrorCode::IoInvalidData),
        EXDEV => Some(HostErrorCode::IoCrossDevice),
        EPIPE => Some(HostErrorCode::IoBrokenPipe),
        ETIMEDOUT => Some(HostErrorCode::IoTimedOut),
        EINTR => Some(HostErrorCode::IoInterrupted),
        EBUSY => Some(HostErrorCode::IoBusy),
        _ => None,
    }
}

/// Map an errno value to an IO error code when possible.
#[cfg(not(any(unix, windows)))]
pub fn io_error_code_from_errno(errno: i32) -> Option<HostErrorCode> {
    let _ = errno;

    // non-OS targets do not expose a stable errno domain here
    None
}

/// Map an errno value to a network error code when possible.
#[cfg(any(unix, windows))]
pub fn net_error_code_from_errno(errno: i32) -> Option<HostErrorCode> {
    if errno == EWOULDBLOCK || errno == EAGAIN {
        return Some(HostErrorCode::IoWouldBlock);
    }

    match errno {
        ECONNREFUSED => Some(HostErrorCode::NetConnectionRefused),
        ETIMEDOUT => Some(HostErrorCode::NetTimedOut),
        ECONNRESET => Some(HostErrorCode::NetConnectionReset),
        EADDRINUSE => Some(HostErrorCode::NetAddressInUse),
        EADDRNOTAVAIL => Some(HostErrorCode::NetAddressNotAvailable),
        ENETUNREACH => Some(HostErrorCode::NetNetworkUnreachable),
        EHOSTUNREACH => Some(HostErrorCode::NetHostUnreachable),
        ECONNABORTED => Some(HostErrorCode::NetConnectionAborted),
        EPIPE => Some(HostErrorCode::NetBrokenPipe),
        ENOTCONN => Some(HostErrorCode::NetNotConnected),
        EISCONN => Some(HostErrorCode::NetAlreadyConnected),
        EMSGSIZE => Some(HostErrorCode::NetMessageTooLarge),
        ENOTSOCK => Some(HostErrorCode::NetNotSocket),
        EPROTONOSUPPORT => Some(HostErrorCode::NetUnsupportedProtocol),
        ENOBUFS => Some(HostErrorCode::NetNoBufferSpace),
        EINPROGRESS | EALREADY => Some(HostErrorCode::NetInProgress),
        #[cfg(unix)]
        ESHUTDOWN => Some(HostErrorCode::NetShutdown),
        #[cfg(unix)]
        EAFNOSUPPORT => Some(HostErrorCode::NetUnsupportedFamily),
        #[cfg(unix)]
        EAI_NONAME | EAI_FAIL | EAI_AGAIN => Some(HostErrorCode::NetDnsFailed),
        _ => None,
    }
}

/// Map an errno value to a network error code when possible.
#[cfg(not(any(unix, windows)))]
pub fn net_error_code_from_errno(errno: i32) -> Option<HostErrorCode> {
    let _ = errno;

    // non-OS targets do not expose a stable errno domain here
    None
}

/// Map one Winsock error value to a network or I/O error code when possible.
#[cfg(windows)]
pub fn net_error_code_from_winsock(code: i32) -> Option<HostErrorCode> {
    match code {
        WSAEWOULDBLOCK => Some(HostErrorCode::IoWouldBlock),
        WSAECONNREFUSED => Some(HostErrorCode::NetConnectionRefused),
        WSAETIMEDOUT => Some(HostErrorCode::NetTimedOut),
        WSAECONNRESET => Some(HostErrorCode::NetConnectionReset),
        WSAEADDRINUSE => Some(HostErrorCode::NetAddressInUse),
        WSAEADDRNOTAVAIL => Some(HostErrorCode::NetAddressNotAvailable),
        WSAENETUNREACH => Some(HostErrorCode::NetNetworkUnreachable),
        WSAEHOSTUNREACH => Some(HostErrorCode::NetHostUnreachable),
        WSAECONNABORTED => Some(HostErrorCode::NetConnectionAborted),
        WSAENOTCONN => Some(HostErrorCode::NetNotConnected),
        WSAEISCONN => Some(HostErrorCode::NetAlreadyConnected),
        WSAEMSGSIZE => Some(HostErrorCode::NetMessageTooLarge),
        WSAENOTSOCK => Some(HostErrorCode::NetNotSocket),
        WSAEPROTONOSUPPORT => Some(HostErrorCode::NetUnsupportedProtocol),
        WSAENOBUFS => Some(HostErrorCode::NetNoBufferSpace),
        WSAEINPROGRESS | WSAEALREADY => Some(HostErrorCode::NetInProgress),
        WSAESHUTDOWN => Some(HostErrorCode::NetShutdown),
        WSAEAFNOSUPPORT => Some(HostErrorCode::NetUnsupportedFamily),
        _ => None,
    }
}

/// Map an errno value to a process error code when possible.
#[cfg(any(unix, windows))]
pub fn process_error_code_from_errno(errno: i32) -> Option<HostErrorCode> {
    match errno {
        ENOENT => Some(HostErrorCode::ProcessNotFound),
        EACCES | libc::EPERM => Some(HostErrorCode::ProcessPermissionDenied),
        _ => None,
    }
}

/// Map an errno value to a process error code when possible.
#[cfg(not(any(unix, windows)))]
pub fn process_error_code_from_errno(errno: i32) -> Option<HostErrorCode> {
    let _ = errno;

    // non-OS targets do not expose a stable errno domain here
    None
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for HostError {}

impl From<HostError> for vm::Error {
    fn from(error: HostError) -> Self {
        if matches!(
            error.code,
            HostErrorCode::InvalidArgument
                | HostErrorCode::InvalidArgumentType
                | HostErrorCode::InvalidArgumentValue
        ) {
            return vm::Error::TypeMismatch {
                expected: "valid argument".to_string(),
                actual: error.message(),
            };
        }

        if error.code == HostErrorCode::NullPointer {
            return vm::Error::NullPointerDereference;
        }

        if error.code == HostErrorCode::NotSupported {
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

/// Result type for host binding handlers.
pub type HostResult<T> = Result<T, Box<HostError>>;
