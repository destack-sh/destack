#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) use std::collections::HashSet;
pub(super) use std::ffi::OsStr;
pub(super) use std::os::fd::RawFd;
pub(super) use std::os::unix::ffi::OsStrExt;
pub(super) use std::path::{Path, PathBuf};
pub(super) use std::sync::Arc;
pub(super) use std::time::Instant;

pub(super) use crate::diagnostic::{RuntimeError, RuntimeResult};
pub(super) use crate::platform::abi::{NativeSlice, NativeStringRef};
pub(super) use crate::platform::device::{
    SerialDataBits, SerialErrorKind, SerialEvent, SerialFlowControl, SerialInputSignals,
    SerialOutputSignals, SerialParity, SerialPortConfig, SerialPortDescriptor,
    SerialPortOpenOptions, SerialPortTransport, SerialStopBits, SerialWatchEvent,
};
pub(super) use crate::platform::diagnostic::PlatformErrorCode;
pub(super) use crate::platform::resource::{ResourceEntry, ResourceKind};
pub(super) use crate::platform::{PlatformError, core as core_platform, resource};
pub(super) use crate::runtime::BindingCallContext;

/// Candidate serial prefixes on BSD hosts.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) const SERIAL_DEVICE_PREFIXES: &[&str] = &["cua", "ttyU", "ttyu", "tty"];

/// Maximum poll timeout chunk that fits one host `poll` call.
pub(super) const MAX_POLL_TIMEOUT_MILLIS: i32 = i32::MAX;

/// One platform-correct ioctl request type.
#[cfg(target_os = "linux")]
pub(super) type UnixIoctlRequest = libc::Ioctl;

/// One platform-correct ioctl request type.
#[cfg(not(target_os = "linux"))]
pub(super) type UnixIoctlRequest = libc::c_ulong;
