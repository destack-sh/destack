use std::fs::{self as stdfs};

use rustc_hash::FxHashSet;

use crate::diagnostic::RuntimeResult;
use crate::host::Platform;
use crate::platform::core::{io_not_found, io_operation_error, pathbuf_from_file_uri};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};

use super::super::roots::desktop_media_roots;
use super::super::storage::validated_media_path;
use super::asset::{
    HOST_MEDIA_LIST_OPERATION, HOST_MEDIA_READ_OPERATION, media_descriptor_from_path,
};
use super::scan::{collect_media_assets, query_cursor_offset, scan_roots_for_query};

/// Default media page size when the query does not request one.
const DEFAULT_MEDIA_PAGE_LIMIT: usize = 100;

/// List media assets for one desktop host query.
pub(in crate::host::app::media) fn list_media_assets(
    platform: Platform,
    query: &MediaQueryValue,
) -> RuntimeResult<MediaPageValue> {
    let roots = desktop_media_roots(platform)?;
    let roots = scan_roots_for_query(&roots, query);
    let offset = query_cursor_offset(query.cursor.as_deref(), HOST_MEDIA_LIST_OPERATION)?;
    let limit = query
        .limit
        .map(|value| value.max(1) as usize)
        .unwrap_or(DEFAULT_MEDIA_PAGE_LIMIT);
    let mut seen = FxHashSet::default();
    let mut assets = Vec::new();

    // scan one host media root at a time
    for root in roots {
        collect_media_assets(platform, &root, query, &mut seen, &mut assets)?;
    }

    // keep pagination deterministic across roots
    assets.sort_by(|left, right| left.id.cmp(&right.id));

    let total = assets.len();
    let page_assets = assets
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let next_offset = offset.saturating_add(page_assets.len());
    let has_more = next_offset < total;
    let next_cursor = if has_more {
        next_offset.to_string()
    } else {
        String::new()
    };

    Ok(MediaPageValue {
        assets: page_assets,
        next_cursor,
        has_more,
    })
}

/// Read one media asset by stable identifier.
pub(in crate::host::app::media) fn read_media_asset(
    platform: Platform,
    id: &str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    let path = pathbuf_from_file_uri(id, "id")?;
    let path = validated_media_path(platform, &path, HOST_MEDIA_READ_OPERATION)?;
    let metadata = stdfs::metadata(&path).map_err(|error| {
        io_operation_error(
            HOST_MEDIA_READ_OPERATION,
            Some(PlatformErrorCode::IoNotFound),
            format!("media metadata failed: {error}"),
        )
    })?;

    if !metadata.is_file() {
        return Err(io_not_found(
            HOST_MEDIA_READ_OPERATION,
            "media asset was not found",
        ));
    }

    media_descriptor_from_path(
        platform,
        &path,
        &metadata,
        None,
        HOST_MEDIA_READ_OPERATION,
        Some(&path),
    )?
    .ok_or_else(|| io_not_found(HOST_MEDIA_READ_OPERATION, "media asset was not found"))
}
