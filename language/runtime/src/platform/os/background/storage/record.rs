use std::fs;
use std::path::PathBuf;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::HostRequestContext;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;

use super::core::DesktopBackgroundTaskRecord;
use super::path::{background_task_directory, background_task_record_path};

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

        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let payload = fs::read_to_string(&path).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoPermissionDenied),
                format!(
                    "destack.os.background.list: failed to read background task record {}: {error}",
                    path.display()
                ),
            ))
            .boxed()
        })?;
        let record =
            serde_json::from_str::<DesktopBackgroundTaskRecord>(&payload).map_err(|error| {
                RuntimeError::from(PlatformError::generic(
                    Some(PlatformErrorCode::IoInvalidData),
                    format!(
                        "destack.os.background.list: invalid background task record {}: {error}",
                        path.display()
                    ),
                ))
                .boxed()
            })?;

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

    let payload = fs::read_to_string(&path).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!(
                "destack.os.background: failed to read background task record {}: {error}",
                path.display()
            ),
        ))
        .boxed()
    })?;
    let record =
        serde_json::from_str::<DesktopBackgroundTaskRecord>(&payload).map_err(|error| {
            RuntimeError::from(PlatformError::generic(
                Some(PlatformErrorCode::IoInvalidData),
                format!(
                    "destack.os.background: invalid background task record {}: {error}",
                    path.display()
                ),
            ))
            .boxed()
        })?;

    Ok(Some(record))
}

/// Persist one desktop background task record.
pub(crate) fn write_background_task_record(
    context: &HostRequestContext,
    record: &DesktopBackgroundTaskRecord,
) -> RuntimeResult<()> {
    let path = background_task_record_path(context, &record.options.identifier)?;
    let payload = serde_json::to_vec_pretty(record).map_err(|error| {
        RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoInvalidData),
            format!(
                "destack.os.background.register: failed to encode background task record: {error}"
            ),
        ))
        .boxed()
    })?;

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
