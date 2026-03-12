use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::fs::{OsPath, core as core_fs};
use crate::platform::net::AcceptFlags;
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::BindingCallContext;

/// Normalized accept-flag bit for nonblocking sockets.
pub(crate) const ACCEPT_FLAG_NONBLOCK: u32 = 1 << 0;
/// Normalized accept-flag bit for close-on-exec or non-inheritable sockets.
pub(crate) const ACCEPT_FLAG_CLOEXEC: u32 = 1 << 1;
/// Bitmask of all supported normalized accept flags.
const ACCEPT_FLAG_SUPPORTED_BITS: u32 = ACCEPT_FLAG_NONBLOCK | ACCEPT_FLAG_CLOEXEC;

/// Decoded accept-flag behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AcceptBehavior {
    /// Whether the accepted socket should be nonblocking.
    pub(crate) nonblocking: bool,
    /// Whether the accepted socket should be close-on-exec or non-inheritable.
    pub(crate) cloexec: bool,
}

/// Decode one normalized accept-flag bitset.
pub(crate) fn decode_accept_flags(flags: AcceptFlags) -> RuntimeResult<AcceptBehavior> {
    let unsupported_flags = flags.0 & !ACCEPT_FLAG_SUPPORTED_BITS;
    if unsupported_flags != 0 {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "flags",
            format!("unsupported accept flags: {unsupported_flags:#x}"),
        ))
        .boxed());
    }

    Ok(AcceptBehavior {
        nonblocking: (flags.0 & ACCEPT_FLAG_NONBLOCK) != 0,
        cloexec: (flags.0 & ACCEPT_FLAG_CLOEXEC) != 0,
    })
}

/// Resolve a socket or listener handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    binding: &BindingCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = binding
        .agent()
        .resources
        .with_entry(id, |entry| {
            if entry.kind != kind {
                return None;
            }
            Some(with_entry(entry))
        })
        .flatten();

    match resolved {
        Some(Ok(value)) => Ok(value),
        Some(Err(error)) => Err(error),
        None => Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "handle",
            format!("unknown {label} handle"),
        ))
        .boxed()),
    }
}

/// Resolve an `OsPath` into byte path data on unix targets.
#[cfg(unix)]
pub(crate) fn unix_path_bytes(path: OsPath, label: &str) -> RuntimeResult<Vec<u8>> {
    match path {
        OsPath::OsPathBytes(path_bytes) => {
            let bytes = unsafe { path_bytes.bytes.0.as_slice()? };
            Ok(bytes.to_vec())
        }
        OsPath::OsPathUtf16(path_utf16) => {
            core_fs::utf16_path_to_utf8_bytes(path_utf16.utf16, label)
        }
    }
}
