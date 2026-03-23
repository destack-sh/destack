use std::collections::VecDeque;
use std::fs::{self as stdfs};
use std::path::{Path, PathBuf};

use rustc_hash::FxHashSet;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::Platform;
use crate::platform::PlatformError;
use crate::platform::core::io_operation_error;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaAssetKind, MediaQueryValue,
};

use super::asset::{MEDIA_LIST_OPERATION, media_descriptor_from_path};
use super::roots::MediaRoots;

/// Return the scan roots relevant to one media query.
pub(crate) fn scan_roots_for_query(roots: &MediaRoots, query: &MediaQueryValue) -> Vec<PathBuf> {
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
            MediaAssetKind::Other => {}
        }
    }

    values
}

/// Parse one query cursor into a stable page offset.
pub(crate) fn query_cursor_offset(
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
pub(crate) fn collect_media_assets(
    platform: Platform,
    root: &Path,
    query: &MediaQueryValue,
    seen: &mut FxHashSet<String>,
    assets: &mut Vec<MediaAssetDescriptorValue>,
) -> RuntimeResult<()> {
    if !root.exists() {
        return Ok(());
    }

    // resolve one stable root path up front
    let root = stdfs::canonicalize(root).map_err(|error| {
        io_operation_error(
            MEDIA_LIST_OPERATION,
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
                MEDIA_LIST_OPERATION,
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
                    MEDIA_LIST_OPERATION,
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
                    MEDIA_LIST_OPERATION,
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

            let descriptor = match media_descriptor_from_path(
                platform,
                &path,
                &metadata,
                Some(query),
                MEDIA_LIST_OPERATION,
                None,
            )? {
                Some(descriptor) => descriptor,
                None => continue,
            };

            if seen.insert(descriptor.id.clone()) {
                assets.push(descriptor);
            }
        }
    }

    Ok(())
}

/// Return whether one path should be hidden from default media queries.
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
