use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{OsPath, PathBytes, PathBytesAbi, PathEncoding, PathUtf16, PathUtf16Abi};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, PlatformError, ResourceId};
use crate::runtime::RuntimeCallContext;

/// Resolve a file or directory handle to its resource entry.
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

/// Build an empty byte path payload.
pub(crate) fn empty_path_bytes() -> PathBytes {
    PathBytesAbi::<NativeAbi>(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build an empty UTF-16 path payload.
pub(crate) fn empty_path_utf16() -> PathUtf16 {
    PathUtf16Abi::<NativeAbi>(NativeArray {
        data: std::ptr::null_mut(),
        len: 0,
        capacity: 0,
    })
}

/// Build an `OsPath` from raw byte data.
pub(crate) fn path_ref_from_bytes(bytes: PathBytes) -> OsPath {
    OsPath {
        encoding: PathEncoding::Bytes,
        bytes,
        utf16: empty_path_utf16(),
    }
}

/// Build an `OsPath` from UTF-16 data.
pub(crate) fn path_ref_from_utf16(utf16: PathUtf16) -> OsPath {
    OsPath {
        encoding: PathEncoding::Utf16,
        bytes: empty_path_bytes(),
        utf16,
    }
}

/// Decode an `OsPath` into a UTF-8 string.
pub(crate) fn os_path_to_utf8_string(path: OsPath, label: &str) -> RuntimeResult<String> {
    match path.encoding {
        PathEncoding::Bytes => {
            let bytes = unsafe { path.bytes.0.as_slice()? };
            String::from_utf8(bytes.to_vec()).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path bytes are not valid utf8",
                ))
                .boxed()
            })
        }
        PathEncoding::Utf16 => {
            let utf16 = unsafe { path.utf16.0.as_slice()? };
            String::from_utf16(utf16).map_err(|_| {
                RuntimeError::from(PlatformError::invalid_argument_value(
                    label,
                    "path utf16 is not valid",
                ))
                .boxed()
            })
        }
    }
}

/// Encode a UTF-8 path string into an `OsPath`.
pub(crate) fn os_path_from_utf8_string(context: &RuntimeCallContext, value: String) -> OsPath {
    #[cfg(unix)]
    {
        let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.into_bytes()));
        return path_ref_from_bytes(bytes);
    }

    #[cfg(windows)]
    {
        let utf16_values = value.encode_utf16().collect::<Vec<_>>();
        let utf16 = PathUtf16Abi::<NativeAbi>(context.store_array(utf16_values));
        return path_ref_from_utf16(utf16);
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.into_bytes()));
        path_ref_from_bytes(bytes)
    }
}

/// Convert a UTF-16 path into a UTF-8 byte vector.
pub(crate) fn utf16_path_to_utf8_bytes(path: PathUtf16, label: &str) -> RuntimeResult<Vec<u8>> {
    let units = unsafe { path.0.as_slice()? };
    let decoded = String::from_utf16(units).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf16",
        ))
        .boxed()
    })?;

    let bytes = decoded.into_bytes();
    if bytes.len() > u32::MAX as usize {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path is too large",
        ))
        .boxed());
    }

    Ok(bytes)
}
