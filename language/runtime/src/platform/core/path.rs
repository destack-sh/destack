use std::path::{Path, PathBuf};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;

#[cfg(windows)]
use windows::Win32::System::Com::CoTaskMemFree;
#[cfg(windows)]
use windows::Win32::UI::Shell::{KF_FLAG_DONT_VERIFY, SHGetKnownFolderPath};
#[cfg(windows)]
use windows::core::GUID;

/// Encode one local host path into one stable `file://` URI.
pub(crate) fn file_uri_from_path(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes = path.as_os_str().as_bytes();
        let payload = percent_encode_bytes(bytes);

        format!("file://{payload}")
    }

    #[cfg(windows)]
    {
        let path = path.to_string_lossy().replace('\\', "/");
        let payload = percent_encode_bytes(path.as_bytes());

        format!("file:///{payload}")
    }

    #[cfg(not(any(unix, windows)))]
    {
        let payload = percent_encode_bytes(path.to_string_lossy().as_bytes());

        format!("file://{payload}")
    }
}

/// Decode one `file://` URI into one local host path.
pub(crate) fn pathbuf_from_file_uri(uri: &str, label: &str) -> RuntimeResult<PathBuf> {
    let suffix = uri.strip_prefix("file://").ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            label,
            "value must use the file:// scheme",
        ))
        .boxed()
    })?;

    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let path = if suffix.starts_with('/') {
            suffix
        } else if let Some(path) = suffix.strip_prefix("localhost/") {
            return percent_decode_bytes(&format!("/{path}"), label)
                .map(|bytes| PathBuf::from(OsString::from_vec(bytes)));
        } else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value must target one local file",
            ))
            .boxed());
        };

        percent_decode_bytes(path, label).map(|bytes| PathBuf::from(OsString::from_vec(bytes)))
    }

    #[cfg(windows)]
    {
        let path = if let Some(path) = suffix.strip_prefix('/') {
            path
        } else if let Some(path) = suffix.strip_prefix("localhost/") {
            path
        } else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value must target one local file",
            ))
            .boxed());
        };
        let path = percent_decode_bytes(path, label)?;
        let path = String::from_utf8(path).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value path is not valid utf8",
            ))
            .boxed()
        })?;

        Ok(PathBuf::from(path.replace('/', "\\")))
    }

    #[cfg(not(any(unix, windows)))]
    {
        let path = if suffix.starts_with('/') {
            suffix
        } else {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value must target one local file",
            ))
            .boxed());
        };
        let path = percent_decode_bytes(path, label)?;
        let path = String::from_utf8(path).map_err(|_| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value path is not valid utf8",
            ))
            .boxed()
        })?;

        Ok(PathBuf::from(path))
    }
}

/// Return one Windows known folder path.
#[cfg(windows)]
pub(crate) fn windows_known_folder_path(
    folder_id: &GUID,
    operation: &'static str,
) -> RuntimeResult<PathBuf> {
    let path =
        unsafe { SHGetKnownFolderPath(folder_id, KF_FLAG_DONT_VERIFY, None) }.map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                None,
                format!("{operation}: failed to resolve one Windows known folder path: {error}"),
            ))
            .boxed()
        })?;
    let path_string = unsafe { path.to_string() }.map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            None,
            format!("{operation}: failed to decode one Windows known folder path: {error}"),
        ))
        .boxed()
    })?;

    // free the system-allocated known folder path buffer
    unsafe {
        CoTaskMemFree(Some(path.0 as _));
    }

    Ok(PathBuf::from(path_string))
}

/// Percent-encode one URI byte payload.
fn percent_encode_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len());

    // uri-safe bytes
    for byte in bytes {
        let byte = *byte;
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~' | b':');

        if is_unreserved {
            encoded.push(byte as char);
            continue;
        }

        encoded.push('%');
        encoded.push_str(&format!("{byte:02X}"));
    }

    encoded
}

/// Decode one percent-encoded URI payload into raw bytes.
fn percent_decode_bytes(value: &str, label: &str) -> RuntimeResult<Vec<u8>> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0usize;

    // decode one percent-escaped byte sequence at a time
    while index < bytes.len() {
        let byte = bytes[index];

        if byte != b'%' {
            decoded.push(byte);
            index += 1;
            continue;
        }

        if index + 2 >= bytes.len() {
            return Err(RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value contains one truncated percent escape",
            ))
            .boxed());
        }

        let upper = (bytes[index + 1] as char).to_digit(16).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value contains one invalid percent escape",
            ))
            .boxed()
        })?;
        let lower = (bytes[index + 2] as char).to_digit(16).ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                label,
                "value contains one invalid percent escape",
            ))
            .boxed()
        })?;

        decoded.push(((upper << 4) | lower) as u8);
        index += 3;
    }

    Ok(decoded)
}
