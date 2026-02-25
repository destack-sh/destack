use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::NativeAbi;
use crate::platform::fs::{OsPath, PathBytes, PathBytesAbi, PathEncoding, PathUtf16, PathUtf16Abi};
use crate::platform::resource::{ResourceEntry, ResourceKind};
use crate::platform::{NativeArray, PlatformError, ResourceId};
use crate::runtime::BindingCallContext;

/// Resolve a file or directory handle to its resource entry.
#[cfg_attr(not(any(unix, windows)), allow(dead_code))]
pub(crate) fn require_resource<T>(
    context: &BindingCallContext,
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
pub(crate) fn os_path_from_utf8_string(context: &BindingCallContext, value: String) -> OsPath {
    #[cfg(unix)]
    {
        let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.into_bytes()));
        path_ref_from_bytes(bytes)
    }

    #[cfg(windows)]
    {
        let utf16_values = value.encode_utf16().collect::<Vec<_>>();
        let utf16 = PathUtf16Abi::<NativeAbi>(context.store_array(utf16_values));
        path_ref_from_utf16(utf16)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let bytes = PathBytesAbi::<NativeAbi>(context.store_array(value.into_bytes()));
        path_ref_from_bytes(bytes)
    }
}

/// Convert a UTF-16 path into a UTF-8 byte vector.
#[cfg_attr(not(unix), allow(dead_code))]
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

/// Return a mismatch error for a pair of paths.
pub(crate) fn path_ref_mismatch<T>(label: &str) -> RuntimeResult<T> {
    Err(RuntimeError::from(PlatformError::invalid_argument_value(
        label,
        "mismatched path encoding",
    ))
    .boxed())
}

/// Convert one UTF-16 path into a byte path for Unix handlers.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn with_utf16_as_bytes<T>(
    path: PathUtf16,
    label: &str,
    on_bytes: impl FnOnce(PathBytes) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut bytes = utf16_path_to_utf8_bytes(path, label)?;

    let path_bytes = PathBytesAbi::<NativeAbi>(NativeArray {
        data: bytes.as_mut_ptr(),
        len: bytes.len() as u32,
        capacity: bytes.len() as u32,
    });
    let result = on_bytes(path_bytes);
    drop(bytes);

    result
}

/// Convert a pair of UTF-16 paths into byte paths for Unix handlers.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) fn with_utf16_pair_as_bytes<T>(
    left: PathUtf16,
    right: PathUtf16,
    label: &str,
    on_bytes: impl FnOnce(PathBytes, PathBytes) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut left_bytes = utf16_path_to_utf8_bytes(left, label)?;
    let mut right_bytes = utf16_path_to_utf8_bytes(right, label)?;

    let left_path = PathBytesAbi::<NativeAbi>(NativeArray {
        data: left_bytes.as_mut_ptr(),
        len: left_bytes.len() as u32,
        capacity: left_bytes.len() as u32,
    });
    let right_path = PathBytesAbi::<NativeAbi>(NativeArray {
        data: right_bytes.as_mut_ptr(),
        len: right_bytes.len() as u32,
        capacity: right_bytes.len() as u32,
    });

    let result = on_bytes(left_path, right_path);
    drop((left_bytes, right_bytes));

    result
}

/// Convert one byte path into UTF-16 data for UTF-16 handlers.
#[allow(dead_code)]
pub(crate) fn path_utf16_from_bytes(
    context: &BindingCallContext,
    path: PathBytes,
    label: &str,
) -> RuntimeResult<PathUtf16> {
    let bytes = unsafe { path.0.as_slice()? };
    let text = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "path contains invalid utf8",
        ))
        .boxed()
    })?;
    let utf16: Vec<u16> = text.encode_utf16().collect();

    Ok(PathUtf16Abi::<NativeAbi>(context.store_array(utf16)))
}

/// Dispatch one OsPath through byte or UTF-16 handlers.
pub(crate) fn with_path_ref<T>(
    path: OsPath,
    label: &str,
    on_bytes: impl FnOnce(PathBytes) -> RuntimeResult<T>,
    _on_utf16: impl FnOnce(PathUtf16) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    #[cfg(not(unix))]
    let _ = label;

    match path.encoding {
        PathEncoding::Bytes => on_bytes(path.bytes),
        PathEncoding::Utf16 => {
            #[cfg(unix)]
            {
                with_utf16_as_bytes(path.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(path.utf16)
            }
        }
    }
}

/// Dispatch two OsPath values through byte or UTF-16 handlers.
pub(crate) fn with_path_ref_pair<T>(
    left: OsPath,
    right: OsPath,
    label: &str,
    on_bytes: impl FnOnce(PathBytes, PathBytes) -> RuntimeResult<T>,
    _on_utf16: impl FnOnce(PathUtf16, PathUtf16) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    match (left.encoding, right.encoding) {
        (PathEncoding::Bytes, PathEncoding::Bytes) => on_bytes(left.bytes, right.bytes),
        (PathEncoding::Utf16, PathEncoding::Utf16) => {
            #[cfg(unix)]
            {
                with_utf16_pair_as_bytes(left.utf16, right.utf16, label, on_bytes)
            }
            #[cfg(not(unix))]
            {
                _on_utf16(left.utf16, right.utf16)
            }
        }
        _ => path_ref_mismatch(label),
    }
}
