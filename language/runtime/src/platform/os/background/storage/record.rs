use std::fs;
use std::path::PathBuf;

use postcard::{from_bytes, to_allocvec};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::HostRequestContext;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

use super::core::DesktopBackgroundTaskRecord;
use super::path::{
    DESKTOP_BACKGROUND_RECORD_EXTENSION, background_task_directory, background_task_record_path,
};

/// Ensure the persisted background task directory exists.
pub(crate) fn ensure_background_task_directory(
    context: &HostRequestContext,
) -> RuntimeResult<PathBuf> {
    let directory = background_task_directory(context)?;

    fs::create_dir_all(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background.register: failed to create background task directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    Ok(directory)
}

/// Read every persisted desktop background task record.
pub(crate) fn read_background_task_records(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<DesktopBackgroundTaskRecord>> {
    let directory = background_task_directory(context)?;

    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    let entries = fs::read_dir(&directory).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background.list: failed to read background task directory {}: {error}",
                directory.display()
            ),
        ))
        .boxed()
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoInvalidData),
                format!("destack.os.background.list: invalid background task entry: {error}"),
            ))
            .boxed()
        })?;
        let path = entry.path();

        if path.extension().and_then(|value| value.to_str())
            != Some(DESKTOP_BACKGROUND_RECORD_EXTENSION)
        {
            continue;
        }

        let payload = fs::read(&path).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.list: failed to read background task record {}: {error}",
                    path.display()
                ),
            ))
            .boxed()
        })?;
        let record = decode_background_task_record("destack.os.background.list", &path, &payload)?;

        records.push(record);
    }

    Ok(records)
}

/// Read one persisted desktop background task record when it exists.
pub(crate) fn read_background_task_record(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<Option<DesktopBackgroundTaskRecord>> {
    let path = background_task_record_path(context, identifier)?;

    if !path.exists() {
        return Ok(None);
    }

    let payload = fs::read(&path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background: failed to read background task record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;
    let record = decode_background_task_record("destack.os.background", &path, &payload)?;

    Ok(Some(record))
}

/// Persist one desktop background task record.
pub(crate) fn write_background_task_record(
    context: &HostRequestContext,
    record: &DesktopBackgroundTaskRecord,
) -> RuntimeResult<()> {
    let path = background_task_record_path(context, &record.options.identifier)?;
    let payload = encode_background_task_record("destack.os.background.register", record)?;

    fs::write(&path, payload).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background.register: failed to write background task record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;

    Ok(())
}

/// Decode one persisted desktop background task record.
fn decode_background_task_record(
    operation: &'static str,
    path: &std::path::Path,
    payload: &[u8],
) -> RuntimeResult<DesktopBackgroundTaskRecord> {
    from_bytes::<DesktopBackgroundTaskRecord>(payload).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            format!(
                "{operation}: invalid background task record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })
}

/// Encode one persisted desktop background task record.
fn encode_background_task_record(
    operation: &'static str,
    record: &DesktopBackgroundTaskRecord,
) -> RuntimeResult<Vec<u8>> {
    to_allocvec(record).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            format!("{operation}: failed to encode background task record: {error}"),
        ))
        .boxed()
    })
}

/// Remove one persisted desktop background task record when it exists.
pub(crate) fn remove_background_task_record(
    context: &HostRequestContext,
    identifier: &str,
) -> RuntimeResult<()> {
    let path = background_task_record_path(context, identifier)?;

    if !path.exists() {
        return Ok(());
    }

    fs::remove_file(&path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background.unregister: failed to remove background task record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;

    Ok(())
}
