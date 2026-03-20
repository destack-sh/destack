use std::fs::{self as stdfs, Metadata};
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::platform::core::{file_uri_from_path, io_operation_error};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaAssetKind, MediaQueryValue,
};

use super::super::roots::{desktop_media_roots, media_kind_for_path};
/// The media list operation name.
pub(in crate::host::app::media) const HOST_MEDIA_LIST_OPERATION: &str = "destack.os.media.list";

/// The media read operation name.
pub(in crate::host::app::media) const HOST_MEDIA_READ_OPERATION: &str = "destack.os.media.read";

/// Build one media descriptor from one file path.
pub(in crate::host::app::media) fn media_descriptor_from_path(
    platform: Platform,
    path: &Path,
    metadata: &Metadata,
    query: Option<&MediaQueryValue>,
    operation: &'static str,
    identity_path: Option<&Path>,
) -> RuntimeResult<Option<MediaAssetDescriptorValue>> {
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

    Ok(Some(MediaAssetDescriptorValue {
        id: file_uri_from_path(&identity_path),
        uri: file_uri_from_path(&identity_path),
        filename,
        mime_type: String::new(),
        kind,
        width: 0,
        height: 0,
        duration_ms: 0,
        size_bytes: metadata.len(),
        created_unix_ns: created_unix_ns.unwrap_or(0),
        modified_unix_ns: modified_unix_ns.unwrap_or(0),
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
