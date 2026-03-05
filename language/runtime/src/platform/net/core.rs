use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::fs::{OsPath, core as core_fs};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::BindingCallContext;

/// Resolve a socket or listener handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    context: &BindingCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = context
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
