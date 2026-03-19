use std::fs::{self as stdfs, Metadata};
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::platform::core::{file_uri_from_path, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaAssetKind, MediaAssetSummaryValue, MediaQueryValue,
};

use super::super::roots::{desktop_media_roots, media_kind_for_path};
use super::super::storage::validated_media_path;

/// The media list operation name.
pub(in crate::host::app::media) const HOST_MEDIA_LIST_OPERATION: &str = "destack.os.media.list";

/// The media describe operation name.
pub(in crate::host::app::media) const HOST_MEDIA_DESCRIBE_OPERATION: &str =
    "destack.os.media.describe";

/// Build one watch summary for one path when it belongs to the media roots and matches the filter.
pub(in crate::host::app::media) fn media_summary_for_watch_path(
    platform: Platform,
    path: &Path,
    kinds: &[MediaAssetKind],
    include_hidden: bool,
) -> RuntimeResult<Option<MediaAssetSummaryValue>> {
    let path = validated_media_path(platform, path, HOST_MEDIA_LIST_OPERATION)?;
    let metadata = stdfs::metadata(&path).map_err(|error| {
        io_operation_error(
            HOST_MEDIA_LIST_OPERATION,
            Some(PlatformErrorCode::IoNotFound),
            format!("media metadata failed: {error}"),
        )
    })?;

    if !metadata.is_file() {
        return Ok(None);
    }

    if !include_hidden && is_hidden_path(&path, &metadata) {
        return Ok(None);
    }

    let query = MediaQueryValue {
        cursor: None,
        limit: None,
        kinds: kinds.to_vec(),
        include_hidden,
    };

    media_summary_from_path(
        platform,
        &path,
        &metadata,
        Some(&query),
        HOST_MEDIA_LIST_OPERATION,
        Some(&path),
    )
}

/// Build one media summary from one file path.
pub(in crate::host::app::media) fn media_summary_from_path(
    platform: Platform,
    path: &Path,
    metadata: &Metadata,
    query: Option<&MediaQueryValue>,
    operation: &'static str,
    identity_path: Option<&Path>,
) -> RuntimeResult<Option<MediaAssetSummaryValue>> {
    let kind = media_kind_from_path(platform, path)?;

    if let Some(query) = query
        && !query.kinds.is_empty()
        && !query.kinds.contains(&kind)
    {
        return Ok(None);
    }

    let identity_path = match identity_path {
        Some(identity_path) => identity_path.to_path_buf(),
        None => stdfs::canonicalize(path).map_err(|error| {
            io_operation_error(
                operation,
                Some(PlatformErrorCode::IoInvalidData),
                format!(
                    "media asset canonicalization failed for `{}`: {error}",
                    path.display()
                ),
            )
        })?,
    };
    let filename = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let created_unix_ns = metadata
        .created()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().min(u64::MAX as u128) as u64);
    let modified_unix_ns = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos().min(u64::MAX as u128) as u64);

    Ok(Some(MediaAssetSummaryValue {
        id: file_uri_from_path(&identity_path),
        uri: file_uri_from_path(&identity_path),
        filename,
        kind,
        content_type: None,
        size_bytes: Some(metadata.len()),
        created_unix_ns,
        modified_unix_ns,
    }))
}

/// Build one media descriptor from one file path.
pub(in crate::host::app::media) fn media_descriptor_from_path(
    platform: Platform,
    path: &Path,
    metadata: &Metadata,
    query: Option<&MediaQueryValue>,
    operation: &'static str,
    identity_path: Option<&Path>,
) -> RuntimeResult<Option<MediaAssetDescriptorValue>> {
    let summary =
        media_summary_from_path(platform, path, metadata, query, operation, identity_path)?;
    let Some(summary) = summary else {
        return Ok(None);
    };

    Ok(Some(MediaAssetDescriptorValue {
        asset: summary,
        dimensions: None,
        duration_ms: None,
    }))
}

/// Return one desktop media class from root ownership.
fn media_kind_from_path(platform: Platform, path: &Path) -> RuntimeResult<MediaAssetKind> {
    let roots = desktop_media_roots(platform)?;
    media_kind_for_path(&roots, path)?.ok_or_else(|| {
        io_operation_error(
            HOST_MEDIA_LIST_OPERATION,
            Some(PlatformErrorCode::IoInvalidData),
            format!(
                "media asset `{}` is not rooted in one declared media library",
                path.display()
            ),
        )
    })
}

/// Return whether one path should be hidden from default desktop media queries.
fn is_hidden_path(path: &Path, _metadata: &Metadata) -> bool {
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
