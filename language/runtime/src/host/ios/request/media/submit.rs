use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::media::{
    HostMediaAssetKind, HostMediaDeleteRequest, HostMediaDeleteResponse,
    HostMediaImportPathRequest, HostMediaImportPathResponse, HostMediaListRequest,
    HostMediaListResponse, HostMediaReadResponse,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::error::invalid_argument_value;
use crate::host::os::apple::abi::media::{
    destack_host_ios_media_delete, destack_host_ios_media_import_path, destack_host_ios_media_list,
    destack_host_ios_media_read,
};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::NativeAbiCodec;
use crate::platform::abi::{NativeStringRef, NativeStringSlice};
use crate::platform::fs::{OsPath, core as core_fs};
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::runtime::BindingCallContext;

/// Return one iOS media request outcome when supported.
pub(crate) fn submit_media_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsMediaList { query } => {
            let page = submit_media_list(runtime_id, request.operation_name(), query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaPage(page),
            )))
        }
        HostRequest::OsMediaRead { id } => {
            let descriptor = submit_media_read(runtime_id, request.operation_name(), id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaAssetDescriptor(descriptor),
            )))
        }
        HostRequest::OsMediaImportPath { path, kind } => {
            let id = submit_media_import_path(runtime_id, request.operation_name(), *path, *kind)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsMediaDelete { ids } => {
            let deleted_count = submit_media_delete(runtime_id, request.operation_name(), ids)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::U32(
                deleted_count,
            ))))
        }
        _ => Ok(None),
    }
}

/// Submit one iOS media-list request.
fn submit_media_list(
    runtime_id: u64,
    operation: &'static str,
    query: &MediaQueryValue,
) -> RuntimeResult<MediaPageValue> {
    let binding = BindingCallContext::from_current_worker_for_native()?;
    let request = HostMediaListRequest::from_value(&binding, query.clone());
    let mut response = MaybeUninit::<HostMediaListResponse>::uninit();
    let status = unsafe { destack_host_ios_media_list(runtime_id, request, response.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    decode_callback_host_status(response.status, operation)?;

    let page = unsafe { response.page.into_value()? };

    let Some(page) = page else {
        return Err(invalid_argument_value(
            "response.page",
            format!("{operation} returned success without one media page"),
        )
        .into());
    };

    Ok(page)
}

/// Submit one iOS media-read request.
fn submit_media_read(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    let mut response = MaybeUninit::<HostMediaReadResponse>::uninit();
    let status = unsafe {
        destack_host_ios_media_read(runtime_id, NativeStringRef::from(id), response.as_mut_ptr())
    };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    decode_callback_host_status(response.status, operation)?;

    let descriptor = unsafe { response.descriptor.into_value()? };

    let Some(descriptor) = descriptor else {
        return Err(invalid_argument_value(
            "response.descriptor",
            format!("{operation} returned success without one media asset descriptor"),
        )
        .into());
    };

    Ok(descriptor)
}

/// Submit one iOS media-import request.
fn submit_media_import_path(
    runtime_id: u64,
    operation: &'static str,
    path: OsPath,
    kind: MediaAssetKind,
) -> RuntimeResult<String> {
    let binding = BindingCallContext::from_current_worker_for_native()?;
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    let request = HostMediaImportPathRequest {
        path: NativeStringRef::from_value(&binding, path),
        kind: HostMediaAssetKind::from_value(&binding, kind),
    };
    let mut response = MaybeUninit::<HostMediaImportPathResponse>::uninit();
    let status =
        unsafe { destack_host_ios_media_import_path(runtime_id, request, response.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    decode_callback_host_status(response.status, operation)?;

    let identifier = unsafe { response.identifier.into_value()? };

    let Some(identifier) = identifier else {
        return Err(invalid_argument_value(
            "response.identifier",
            format!("{operation} returned success without one media identifier"),
        )
        .into());
    };

    Ok(identifier)
}

/// Submit one iOS media-delete request.
fn submit_media_delete(
    runtime_id: u64,
    operation: &'static str,
    ids: &[String],
) -> RuntimeResult<u32> {
    let binding = BindingCallContext::from_current_worker_for_native()?;
    let request = HostMediaDeleteRequest {
        identifiers: NativeStringSlice::from_value(&binding, ids.to_vec()),
    };
    let mut response = MaybeUninit::<HostMediaDeleteResponse>::uninit();
    let status =
        unsafe { destack_host_ios_media_delete(runtime_id, request, response.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };
    decode_callback_host_status(response.status, operation)?;

    Ok(response.deleted_count)
}
