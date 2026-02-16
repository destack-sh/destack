use crate::diagnostic::{RuntimeError, RuntimeResult};
#[cfg(unix)]
use crate::platform::fs::{OsPath, PathEncoding, core as core_fs};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;

/// Resolve a socket or listener handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    context: &RuntimeCallContext,
    id: ResourceId,
    kind: ResourceKind,
    label: &str,
    with_entry: impl FnOnce(&ResourceEntry) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let resolved = context
        .runtime()
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
    if path.encoding == PathEncoding::Bytes {
        let bytes = unsafe { path.bytes.0.as_slice()? };
        return Ok(bytes.to_vec());
    }

    core_fs::utf16_path_to_utf8_bytes(path.utf16, label)
}
