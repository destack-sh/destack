use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use destack_core::fnv1a_64;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::host::app::identity::resolved_application_identifier;
use crate::host::core::HostRequestContext;
use crate::platform::PlatformError;
use crate::platform::core::{file_uri_from_path, invalid_argument};
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(not(windows))]
use crate::platform::fs::abi_generated::PathBytesValue;
#[cfg(windows)]
use crate::platform::fs::abi_generated::PathUtf16Value;
use crate::platform::fs::abi_generated::{OsPathBytesValue, OsPathUtf16Value, OsPathValue};
use crate::platform::os::abi_generated::DocumentDescriptorValue;

/// Directory name for imported document copies.
const DOCUMENT_APP_STORAGE_DIRECTORY: &str = "documents";
/// Prefix for one imported document copy name.
const DOCUMENT_APP_STORAGE_PREFIX: &str = "imported";

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
        content_type: None,
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

/// Import document descriptors into app-owned storage.
pub(crate) fn import_document_descriptors_to_app_storage(
    context: &HostRequestContext,
    descriptors: Vec<DocumentDescriptorValue>,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    let app_storage_directory = ensure_document_app_storage_directory(context)?;
    let mut imported = Vec::with_capacity(descriptors.len());

    // import each picked document into app-owned storage when possible
    for (index, descriptor) in descriptors.into_iter().enumerate() {
        let source_path = source_path_from_document_descriptor(&descriptor)?;
        let copied_path =
            import_document_path_to_app_storage(&source_path, &app_storage_directory, index)?;
        let mut descriptor = descriptor;
        descriptor.local_path = Some(os_path_value_from_path(&copied_path));
        imported.push(descriptor);
    }

    Ok(imported)
}

/// Return one app-storage directory for imported picker results.
pub(crate) fn ensure_document_app_storage_directory(
    context: &HostRequestContext,
) -> RuntimeResult<PathBuf> {
    let directory = ensure_document_state_directory(context, DOCUMENT_APP_STORAGE_DIRECTORY)?;

    Ok(directory)
}

/// Return one state directory for one document service lane.
pub(crate) fn ensure_document_state_directory(
    context: &HostRequestContext,
    directory_name: &str,
) -> RuntimeResult<PathBuf> {
    let directory = if let Some(data_directory) = &context.os_options.data_directory {
        data_directory.join(directory_name)
    } else {
        default_document_state_directory(context, directory_name)?
    };

    fs::create_dir_all(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.document: failed to create document state directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    Ok(directory)
}

/// Return the default state directory for one document service lane.
fn default_document_state_directory(
    context: &HostRequestContext,
    directory_name: &str,
) -> RuntimeResult<PathBuf> {
    let app_identifier = resolved_application_identifier(context)?;

    match context.platform {
        // macOS app data
        Platform::MacOS => {
            let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
                return Err(invalid_argument(
                    "path",
                    "HOME must be present for document app storage",
                ));
            };

            Ok(home
                .join("Library")
                .join("Application Support")
                .join(app_identifier)
                .join(directory_name))
        }

        // windows app data
        Platform::Windows => {
            let Some(root) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
                return Err(invalid_argument(
                    "path",
                    "LOCALAPPDATA must be present for document app storage",
                ));
            };

            Ok(root.join(app_identifier).join(directory_name))
        }

        // linux and unix app data
        Platform::Linux
        | Platform::FreeBsd
        | Platform::NetBsd
        | Platform::OpenBsd
        | Platform::DragonFly
        | Platform::Illumos
        | Platform::Solaris
        | Platform::Haiku => {
            let root = if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
                PathBuf::from(data_home)
            } else {
                let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
                    return Err(invalid_argument(
                        "path",
                        "HOME must be present for document app storage",
                    ));
                };

                home.join(".local").join("share")
            };

            Ok(root.join(app_identifier).join(directory_name))
        }

        // mobile app data
        Platform::Android | Platform::IOS => Err(invalid_argument(
            "platform",
            "desktop document app-storage import is unavailable on this host backend",
        )),
        _ => Err(invalid_argument(
            "platform",
            "desktop document app-storage import is unavailable on this host backend",
        )),
    }
}

/// Decode one local source path from one document descriptor.
pub(crate) fn source_path_from_document_descriptor(
    descriptor: &DocumentDescriptorValue,
) -> RuntimeResult<PathBuf> {
    let Some(local_path) = &descriptor.local_path else {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            "destack.os.document.pick: picked document does not expose one local path for app-storage import",
        ))
        .boxed());
    };

    os_path_value_to_path_buf(local_path)
}

/// Decode one runtime OS path value into one host path buffer.
pub(crate) fn os_path_value_to_path_buf(path: &OsPathValue) -> RuntimeResult<PathBuf> {
    match path {
        OsPathValue::OsPathBytes(value) => os_path_bytes_value_to_path_buf(value),
        OsPathValue::OsPathUtf16(value) => os_path_utf16_value_to_path_buf(value),
    }
}

/// Decode one runtime byte path value into one host path buffer.
#[cfg(not(windows))]
fn os_path_bytes_value_to_path_buf(value: &OsPathBytesValue) -> RuntimeResult<PathBuf> {
    use std::os::unix::ffi::OsStringExt;

    let os_string = OsString::from_vec(value.bytes.0.clone());

    Ok(PathBuf::from(os_string))
}

/// Reject one runtime byte path value on the Windows backend.
#[cfg(windows)]
fn os_path_bytes_value_to_path_buf(_value: &OsPathBytesValue) -> RuntimeResult<PathBuf> {
    Err(RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoInvalidData),
        "destack.os.document.pick: byte paths are not valid on this Windows backend",
    ))
    .boxed())
}

/// Decode one runtime UTF-16 path value into one host path buffer.
#[cfg(windows)]
fn os_path_utf16_value_to_path_buf(value: &OsPathUtf16Value) -> RuntimeResult<PathBuf> {
    use std::os::windows::ffi::OsStringExt;

    let os_string = OsString::from_wide(&value.utf16.0);

    Ok(PathBuf::from(os_string))
}

/// Reject one runtime UTF-16 path value on the Unix backend.
#[cfg(not(windows))]
fn os_path_utf16_value_to_path_buf(_value: &OsPathUtf16Value) -> RuntimeResult<PathBuf> {
    Err(RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::IoInvalidData),
        "destack.os.document.pick: UTF-16 paths are not valid on this Unix backend",
    ))
    .boxed())
}

/// Import one picked path into app-owned storage.
fn import_document_path_to_app_storage(
    source_path: &Path,
    app_storage_directory: &Path,
    index: usize,
) -> RuntimeResult<PathBuf> {
    let unique_path = app_storage_path_for_document(source_path, app_storage_directory, index);

    // files and directories need different import logic
    if source_path.is_dir() {
        copy_directory_recursively(source_path, &unique_path)?;
    } else {
        fs::copy(source_path, &unique_path).map_err(|error| {
            io_operation_error(
                "failed to import picked document into app-owned storage",
                source_path,
                Some(PlatformErrorCode::IoPermissionDenied),
                error,
            )
        })?;
    }

    Ok(unique_path)
}

/// Return one unique app-storage path for one imported picker result.
fn app_storage_path_for_document(
    source_path: &Path,
    app_storage_directory: &Path,
    index: usize,
) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let file_name = source_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_string());
    let identity = source_path.to_string_lossy();
    let hash = fnv1a_64(identity.as_bytes());

    app_storage_directory.join(format!(
        "{DOCUMENT_APP_STORAGE_PREFIX}-{index}-{timestamp}-{hash:016x}-{file_name}"
    ))
}

/// Copy one directory tree recursively into app-owned storage.
fn copy_directory_recursively(
    source_directory: &Path,
    target_directory: &Path,
) -> RuntimeResult<()> {
    fs::create_dir_all(target_directory).map_err(|error| {
        io_operation_error(
            "failed to create picked document directory in app-owned storage",
            target_directory,
            Some(PlatformErrorCode::IoPermissionDenied),
            error,
        )
    })?;

    let entries = fs::read_dir(source_directory).map_err(|error| {
        io_operation_error(
            "failed to read picked document directory",
            source_directory,
            Some(PlatformErrorCode::IoPermissionDenied),
            error,
        )
    })?;

    // copy each child entry into the new app-storage directory
    for entry in entries {
        let entry = entry.map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoInvalidData),
                format!("destack.os.document.pick: invalid picked directory entry: {error}"),
            ))
            .boxed()
        })?;
        let source_path = entry.path();
        let target_path = target_directory.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory_recursively(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).map_err(|error| {
                io_operation_error(
                    "failed to copy picked document file into app-owned storage",
                    &source_path,
                    Some(PlatformErrorCode::IoPermissionDenied),
                    error,
                )
            })?;
        }
    }

    Ok(())
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
