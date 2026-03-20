use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::core::file_uri_from_path;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::fs::abi_generated::OsPathValue;
#[cfg(not(windows))]
use crate::platform::fs::abi_generated::{OsPathBytesValue, PathBytesValue};
#[cfg(windows)]
use crate::platform::fs::abi_generated::{OsPathUtf16Value, PathUtf16Value};
use crate::platform::os::abi_generated::DocumentDescriptorValue;

/// Build one document descriptor from one local path.
pub(crate) fn document_descriptor_value_from_path(
    path: &Path,
) -> RuntimeResult<DocumentDescriptorValue> {
    let metadata = fs::metadata(path).map_err(|error| {
        io_operation_error(
            "document path metadata failed",
            path,
            Some(PlatformErrorCode::IoNotFound),
            error,
        )
    })?;
    let is_directory = metadata.is_dir();
    let modified_unix_nanos = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0);

    Ok(DocumentDescriptorValue {
        uri: file_uri_from_path(path),
        local_path: Some(os_path_value_from_path(path)),
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string()),
        size_bytes: (!is_directory).then_some(metadata.len()),
        mime_type: None,
        is_directory,
        modified_unix_ns: Some(modified_unix_nanos),
    })
}

/// Encode one local host path into one runtime path value.
pub(crate) fn os_path_value_from_path(path: &Path) -> OsPathValue {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let utf16 = path.as_os_str().encode_wide().collect::<Vec<_>>();

        return OsPathValue::OsPathUtf16(OsPathUtf16Value {
            kind: "utf16".to_string(),
            utf16: PathUtf16Value(utf16),
        });
    }

    #[cfg(not(windows))]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes = path.as_os_str().as_bytes().to_vec();

        OsPathValue::OsPathBytes(OsPathBytesValue {
            kind: "bytes".to_string(),
            bytes: PathBytesValue(bytes),
        })
    }
}

/// Map one filesystem error into one picker runtime error.
fn io_operation_error(
    message: &str,
    path: &Path,
    code: Option<PlatformErrorCode>,
    error: impl std::fmt::Display,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        code,
        format!(
            "destack.os.document.pick: {message} {}: {error}",
            path.display()
        ),
    ))
    .boxed()
}
