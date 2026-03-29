use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::callback::decode_callback_host_required_string_ref;
use crate::platform::PlatformError;
use crate::platform::abi::NativeStringRef;
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::runtime::BindingCallContext;

use crate::host::abi::media::{
    HostMediaAssetDescriptor, HostMediaAssetKind, HostMediaDeleteRequest, HostMediaDeleteResponse,
    HostMediaImportPathRequest, HostMediaImportPathResponse, HostMediaListRequest,
    HostMediaListResponse, HostMediaListResult, HostMediaReadResponse,
};

/// Encode one media-list request payload for the host ABI.
pub(crate) fn encode_media_list_request(
    binding: &BindingCallContext,
    query: &MediaQueryValue,
) -> HostMediaListRequest {
    HostMediaListRequest {
        has_cursor: query.cursor.is_some(),
        cursor: binding.store_string(query.cursor.as_deref().unwrap_or("")),
        has_limit: query.limit.is_some(),
        limit: query.limit.unwrap_or_default(),
        kinds: binding.store_slice(
            query
                .kinds
                .iter()
                .copied()
                .map(encode_media_asset_kind)
                .collect(),
        ),
        include_hidden: query.include_hidden,
    }
}

/// Encode one media-import request payload for the host ABI.
pub(crate) fn encode_media_import_path_request(
    binding: &BindingCallContext,
    path: &str,
    kind: MediaAssetKind,
) -> HostMediaImportPathRequest {
    HostMediaImportPathRequest {
        path: binding.store_string(path),
        kind: encode_media_asset_kind(kind),
    }
}

/// Encode one media-delete request payload for the host ABI.
pub(crate) fn encode_media_delete_request(
    binding: &BindingCallContext,
    identifiers: &[String],
) -> HostMediaDeleteRequest {
    HostMediaDeleteRequest {
        identifiers: binding
            .store_string_slice(identifiers.iter().map(NativeStringRef::from).collect()),
    }
}

/// Decode one host media-list response payload.
pub(crate) unsafe fn decode_media_list_response(
    response: HostMediaListResponse,
    operation: &'static str,
) -> RuntimeResult<MediaPageValue> {
    if !response.has_page {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "response.page",
            format!("{operation} returned one missing response.page"),
        ))
        .boxed());
    }

    unsafe { decode_media_list_result(response.page, operation) }
}

/// Decode one host media-read response payload.
pub(crate) unsafe fn decode_media_read_response(
    response: HostMediaReadResponse,
    operation: &'static str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    if !response.has_descriptor {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "response.descriptor",
            format!("{operation} returned one missing response.descriptor"),
        ))
        .boxed());
    }

    decode_media_asset_descriptor(response.descriptor, operation)
}

/// Decode one host media-import response payload.
pub(crate) unsafe fn decode_media_import_path_response(
    response: HostMediaImportPathResponse,
    operation: &'static str,
) -> RuntimeResult<String> {
    if !response.has_identifier {
        return Err(RuntimeError::from(PlatformError::invalid_argument_value(
            "response.identifier",
            format!("{operation} returned one missing response.identifier"),
        ))
        .boxed());
    }

    decode_callback_host_required_string_ref(
        operation,
        "HostMediaImportPathResponse.identifier",
        response.identifier,
    )
}

/// Decode one host media-delete response payload.
pub(crate) fn decode_media_delete_response(response: HostMediaDeleteResponse) -> u32 {
    response.deleted_count
}

/// Encode one runtime media asset kind as one host media asset kind.
fn encode_media_asset_kind(kind: MediaAssetKind) -> HostMediaAssetKind {
    match kind {
        MediaAssetKind::Image => HostMediaAssetKind::Image,
        MediaAssetKind::Video => HostMediaAssetKind::Video,
        MediaAssetKind::Audio => HostMediaAssetKind::Audio,
        MediaAssetKind::Other => HostMediaAssetKind::Other,
    }
}

/// Decode one host media asset kind into one runtime media asset kind.
fn decode_media_asset_kind(kind: HostMediaAssetKind) -> MediaAssetKind {
    match kind {
        HostMediaAssetKind::Image => MediaAssetKind::Image,
        HostMediaAssetKind::Video => MediaAssetKind::Video,
        HostMediaAssetKind::Audio => MediaAssetKind::Audio,
        HostMediaAssetKind::Other => MediaAssetKind::Other,
    }
}

/// Decode one host media-list result payload.
unsafe fn decode_media_list_result(
    page: HostMediaListResult,
    operation: &'static str,
) -> RuntimeResult<MediaPageValue> {
    let assets = unsafe { page.assets.as_slice()? };
    let assets = assets
        .iter()
        .copied()
        .map(|asset| decode_media_asset_descriptor(asset, operation))
        .collect::<RuntimeResult<Vec<_>>>()?;

    let next_cursor = decode_callback_host_required_string_ref(
        operation,
        "HostMediaListResult.next_cursor",
        page.next_cursor,
    )?;

    Ok(MediaPageValue {
        assets,
        next_cursor,
        has_more: page.has_more,
    })
}

/// Decode one host media asset-descriptor payload.
fn decode_media_asset_descriptor(
    descriptor: HostMediaAssetDescriptor,
    operation: &'static str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    Ok(MediaAssetDescriptorValue {
        id: decode_callback_host_required_string_ref(
            operation,
            "HostMediaAssetDescriptor.identifier",
            descriptor.identifier,
        )?,
        uri: decode_callback_host_required_string_ref(
            operation,
            "HostMediaAssetDescriptor.uri",
            descriptor.uri,
        )?,
        filename: decode_callback_host_required_string_ref(
            operation,
            "HostMediaAssetDescriptor.filename",
            descriptor.filename,
        )?,
        mime_type: decode_callback_host_required_string_ref(
            operation,
            "HostMediaAssetDescriptor.mime_type",
            descriptor.mime_type,
        )?,
        kind: decode_media_asset_kind(descriptor.kind),
        width: descriptor.width,
        height: descriptor.height,
        duration_ms: descriptor.duration_ms,
        size_bytes: descriptor.size_bytes,
        created_unix_ns: descriptor.created_unix_ns,
        modified_unix_ns: descriptor.modified_unix_ns,
    })
}
