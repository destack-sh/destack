use std::collections::VecDeque;
use std::fs::{self as stdfs};
use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::platform::PlatformError;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{MediaAssetKind, MediaAssetSummaryValue, MediaQueryValue};

use super::super::roots::DesktopMediaRoots;
use super::asset::{HOST_MEDIA_LIST_OPERATION, media_summary_from_path};

/// Return the scan roots relevant to one media query.
pub(in crate::host::app::media) fn scan_roots_for_query(
    roots: &DesktopMediaRoots,
    query: &MediaQueryValue,
) -> Vec<PathBuf> {
    if query.kinds.is_empty() {
        return vec![
            roots.pictures.clone(),
            roots.videos.clone(),
            roots.music.clone(),
        ];
    }

    let mut values = Vec::new();

    // select one root per requested media class
    for kind in &query.kinds {
        match kind {
            MediaAssetKind::Image => values.push(roots.pictures.clone()),
            MediaAssetKind::Video => values.push(roots.videos.clone()),
            MediaAssetKind::Audio => values.push(roots.music.clone()),
        }
    }

    values
}

/// Parse one query cursor into a stable page offset.
pub(in crate::host::app::media) fn query_cursor_offset(
    cursor: Option<&str>,
    operation: &'static str,
) -> RuntimeResult<usize> {
    let Some(cursor) = cursor else {
        return Ok(0);
    };

    if cursor.is_empty() {
        return Ok(0);
    }

    cursor.parse::<usize>().map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "query.cursor",
            format!("{operation} cursor must be one nonnegative integer"),
        ))
        .boxed()
    })
}

/// Collect all query-matching assets under one media root.
pub(in crate::host::app::media) fn collect_media_assets(
    platform: Platform,
    root: &Path,
    query: &MediaQueryValue,
    seen: &mut FxHashSet<String>,
    assets: &mut Vec<MediaAssetSummaryValue>,
) -> RuntimeResult<()> {
    if !root.exists() {
        return Ok(());
    }

    // resolve one stable root path up front
    let root = stdfs::canonicalize(root).map_err(|error| {
        io_operation_error(
            HOST_MEDIA_LIST_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!(
                "media root canonicalization failed for `{}`: {error}",
                root.display()
            ),
        )
    })?;
    let mut pending = VecDeque::from([root]);

    // walk one directory tree breadth first for deterministic pagination
    while let Some(directory) = pending.pop_front() {
        let entries = stdfs::read_dir(&directory).map_err(|error| {
            io_operation_error(
                HOST_MEDIA_LIST_OPERATION,
                Some(PlatformErrorCode::IoInvalidData),
                format!(
                    "media directory read failed for `{}`: {error}",
                    directory.display()
                ),
            )
        })?;

        // scan one directory entry at a time
        for entry in entries {
            let entry = entry.map_err(|error| {
                io_operation_error(
                    HOST_MEDIA_LIST_OPERATION,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!(
                        "media directory entry read failed for `{}`: {error}",
                        directory.display()
                    ),
                )
            })?;
            let path = entry.path();
            let metadata = entry.metadata().map_err(|error| {
                io_operation_error(
                    HOST_MEDIA_LIST_OPERATION,
                    Some(PlatformErrorCode::IoInvalidData),
                    format!(
                        "media metadata read failed for `{}`: {error}",
                        path.display()
                    ),
                )
            })?;

            if !query.include_hidden && is_hidden_path(&path, &metadata) {
                continue;
            }

            if metadata.is_dir() {
                pending.push_back(path);
                continue;
            }

            if !metadata.is_file() {
                continue;
            }

            let summary = match media_summary_from_path(
                platform,
                &path,
                &metadata,
                Some(query),
                HOST_MEDIA_LIST_OPERATION,
                None,
            )? {
                Some(summary) => summary,
                None => continue,
            };

            if seen.insert(summary.id.clone()) {
                assets.push(summary);
            }
        }
    }

    Ok(())
}

/// Return whether one path should be hidden from default desktop media queries.
fn is_hidden_path(path: &Path, _metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

        if _metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0 {
            return true;
        }
    }

    path.file_name()
        .map(|value| value.to_string_lossy().starts_with('.'))
        .unwrap_or(false)
}
